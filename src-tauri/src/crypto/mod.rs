//! 加密层：账号小本本 · Account Jotter v7 文件格式（兼容读 v6/v5）
//!
//! - v7（默认写）：magic=`AJOT`, ver=7，Argon2id KDF + SM4-GCM AEAD —— 协议同 v6，仅 magic/版本号不同
//! - v6（兼容读）：magic=`ZHMM`, ver=6，原 Python 版互通格式（Argon2id + SM4-GCM）
//! - v5（兼容读）：magic=`ZHMM`, ver=5，老版 Python 格式（Argon2id + SM4-CBC + HMAC-SM3）
//!
//! v7/v6 文件头（45B）：magic(4) + ver(1) + m_cost(4BE) + t_cost(4BE) + p_cost(4BE)
//!                  + salt(16) + iv(12)，header 全部纳入 GCM AAD。
//! v5 文件头（49B）：iv(16)、tag(32) HMAC-SM3 over blob[..-tag_len]。
//!
//! 写入策略：v2.0 起一律输出 v7（magic=`AJOT`）；读取按 (magic, version) 分发，
//! 旧 v6/v5 文件读后下次保存自动升级到 v7。
//!
//! SM4-GCM 自实现位于 [`sm4_gcm`] 子模块；本文件聚焦协议常量、Argon2 KDF
//! 与文件头的封装/分发。

mod sm4_gcm;

use argon2::{Algorithm, Argon2, Params, Version};
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sm3::Sm3;
use sm4::cipher::generic_array::GenericArray;
use sm4::Sm4;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

use crate::errors::{AppError, AppResult};

use sm4_gcm::{sm4_gcm_open, sm4_gcm_seal};

// ---------- 协议常量 ----------

/// v7 magic：账号小本本 · Account Jotter（v2.0 默认写入）
const MAGIC_V7: &[u8; 4] = b"AJOT";
/// v6/v5 magic：原 Python 版 zhmm（仅兼容读）
const MAGIC_LEGACY: &[u8; 4] = b"ZHMM";
const VERSION_V7: u8 = 7;
const VERSION_V6: u8 = 6;
const VERSION_V5: u8 = 5;

pub const SALT_LEN: usize = 16;
pub const IV_LEN: usize = 12; // GCM 标准 96-bit IV
pub const TAG_LEN: usize = 16; // GCM 128-bit tag
pub const SM4_KEY_LEN: usize = 16;
pub const DERIVED_KEY_LEN: usize = 32; // 仅取前 16B 作 SM4 密钥；余量预留

const V5_IV_LEN: usize = 16;
const V5_TAG_LEN: usize = 32; // HMAC-SM3
const V5_KEY_ENC_LEN: usize = 16;
const V5_KEY_MAC_LEN: usize = 16;

// header_len: magic(4) + ver(1) + m_cost(4) + t_cost(4) + p_cost(4) + salt(16) + iv(12) = 45
const V6_HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + SALT_LEN + IV_LEN;
const V6_MIN_BLOB_LEN: usize = V6_HEADER_LEN + TAG_LEN;
const V5_HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + SALT_LEN + V5_IV_LEN; // 49
const V5_MIN_BLOB_LEN: usize = V5_HEADER_LEN + V5_TAG_LEN + 16; // 至少一个填充块密文

/// Argon2id 默认参数，与 Python 版一致
pub const ARGON2_M_COST: u32 = 65_536;
pub const ARGON2_T_COST: u32 = 3;
pub const ARGON2_P_COST: u32 = 1;

// 解密时接受的 Argon2 参数范围（防 DoS）
const ARGON2_M_MIN: u32 = 8;
const ARGON2_M_MAX: u32 = 524_288; // 512 MiB
const ARGON2_T_MIN: u32 = 1;
const ARGON2_T_MAX: u32 = 100;
const ARGON2_P_MIN: u32 = 1;
const ARGON2_P_MAX: u32 = 64;

// ---------- Argon2id KDF ----------

fn validate_argon2_params(m: u32, t: u32, p: u32) -> AppResult<()> {
    if !(ARGON2_M_MIN..=ARGON2_M_MAX).contains(&m) {
        return Err(AppError::Crypto(format!("argon2 m_cost out of range: {m}")));
    }
    if !(ARGON2_T_MIN..=ARGON2_T_MAX).contains(&t) {
        return Err(AppError::Crypto(format!("argon2 t_cost out of range: {t}")));
    }
    if !(ARGON2_P_MIN..=ARGON2_P_MAX).contains(&p) {
        return Err(AppError::Crypto(format!("argon2 p_cost out of range: {p}")));
    }
    Ok(())
}

/// 用 (account, password) 派生 32B 密钥；前 16B 作 SM4 密钥使用
fn derive_key(
    account: &str,
    password: &str,
    salt: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> AppResult<[u8; DERIVED_KEY_LEN]> {
    if password.is_empty() {
        return Err(AppError::Invalid("密码不能为空".into()));
    }
    let mut material = Vec::with_capacity(account.len() + 1 + password.len());
    material.extend_from_slice(account.as_bytes());
    material.push(0u8);
    material.extend_from_slice(password.as_bytes());

    let params = Params::new(m_cost, t_cost, p_cost, Some(DERIVED_KEY_LEN))
        .map_err(|e| AppError::Crypto(format!("argon2 params: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut out = [0u8; DERIVED_KEY_LEN];
    let res = argon
        .hash_password_into(&material, salt, &mut out)
        .map_err(|e| AppError::Crypto(format!("argon2 derive: {e}")));
    material.zeroize();
    res?;
    Ok(out)
}

// ---------- 公开 API ----------

/// 用 (account, password) 加密明文，输出 v7 blob（magic=`AJOT`）
pub fn seal(account: &str, password: &str, plaintext: &[u8]) -> AppResult<Vec<u8>> {
    seal_inner(MAGIC_V7, VERSION_V7, account, password, plaintext)
}

/// 内部 sealer：参数化 magic/version，供 v7 默认写与测试造 v6 复用
fn seal_inner(
    magic: &[u8; 4],
    version: u8,
    account: &str,
    password: &str,
    plaintext: &[u8],
) -> AppResult<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut iv);

    let m_cost = ARGON2_M_COST;
    let t_cost = ARGON2_T_COST;
    let p_cost = ARGON2_P_COST;

    let mut header = Vec::with_capacity(V6_HEADER_LEN);
    header.extend_from_slice(magic);
    header.push(version);
    header.extend_from_slice(&m_cost.to_be_bytes());
    header.extend_from_slice(&t_cost.to_be_bytes());
    header.extend_from_slice(&p_cost.to_be_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&iv);

    let mut derived = derive_key(account, password, &salt, m_cost, t_cost, p_cost)?;
    let mut key = [0u8; SM4_KEY_LEN];
    key.copy_from_slice(&derived[..SM4_KEY_LEN]);

    let (tag, ciphertext) = sm4_gcm_seal(&key, &iv, &header, plaintext);

    derived.zeroize();
    key.zeroize();

    let mut out = header;
    out.extend(ciphertext);
    out.extend_from_slice(&tag);
    Ok(out)
}

/// 用 (account, password) 解密 blob，按 (magic, version) 分发
///
/// - `AJOT` + ver7 → v7（账号小本本，SM4-GCM）
/// - `ZHMM` + ver6 → v6（Python 版，SM4-GCM，单向兼容读）
/// - `ZHMM` + ver5 → v5（老 Python 版，SM4-CBC + HMAC-SM3，单向兼容读）
pub fn open(account: &str, password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() < 5 {
        return Err(AppError::Crypto("vault blob too short".into()));
    }
    let magic: &[u8; 4] = blob[..4].try_into().unwrap();
    let version = blob[4];
    match (magic, version) {
        (m, VERSION_V7) if m == MAGIC_V7 => open_v6_like(account, password, blob),
        (m, VERSION_V6) if m == MAGIC_LEGACY => open_v6_like(account, password, blob),
        (m, VERSION_V5) if m == MAGIC_LEGACY => open_v5(account, password, blob),
        (m, v) => Err(AppError::Crypto(format!(
            "unsupported vault: magic={:?} version={}",
            std::str::from_utf8(m).unwrap_or("?"),
            v
        ))),
    }
}

/// v7 与 v6 文件头/AAD 结构一致，解密路径完全共享
fn open_v6_like(account: &str, password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() < V6_MIN_BLOB_LEN {
        return Err(AppError::Crypto(format!(
            "vault blob too short: {}",
            blob.len()
        )));
    }
    let mut off = 5;
    let m_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    let t_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    let p_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    validate_argon2_params(m_cost, t_cost, p_cost)?;

    let salt = &blob[off..off + SALT_LEN];
    off += SALT_LEN;
    let iv: [u8; IV_LEN] = blob[off..off + IV_LEN].try_into().unwrap();
    off += IV_LEN;
    debug_assert_eq!(off, V6_HEADER_LEN);

    let header = &blob[..V6_HEADER_LEN];
    let tag: [u8; TAG_LEN] = blob[blob.len() - TAG_LEN..].try_into().unwrap();
    let ciphertext = &blob[V6_HEADER_LEN..blob.len() - TAG_LEN];

    let mut derived = derive_key(account, password, salt, m_cost, t_cost, p_cost)?;
    let mut key = [0u8; SM4_KEY_LEN];
    key.copy_from_slice(&derived[..SM4_KEY_LEN]);

    let result = sm4_gcm_open(&key, &iv, header, ciphertext, &tag);
    derived.zeroize();
    key.zeroize();
    result
}

fn open_v5(account: &str, password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    type Sm4CbcDec = cbc::Decryptor<Sm4>;
    type HmacSm3 = Hmac<Sm3>;

    if blob.len() < V5_MIN_BLOB_LEN {
        return Err(AppError::Crypto(format!(
            "v5 blob too short: {}",
            blob.len()
        )));
    }
    let mut off = 5;
    let m_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    let t_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    let p_cost = u32::from_be_bytes(blob[off..off + 4].try_into().unwrap());
    off += 4;
    validate_argon2_params(m_cost, t_cost, p_cost)?;

    let salt = &blob[off..off + SALT_LEN];
    off += SALT_LEN;
    let iv = &blob[off..off + V5_IV_LEN];
    off += V5_IV_LEN;
    debug_assert_eq!(off, V5_HEADER_LEN);

    let tag = &blob[blob.len() - V5_TAG_LEN..];
    let ciphertext = &blob[V5_HEADER_LEN..blob.len() - V5_TAG_LEN];
    if ciphertext.is_empty() || !ciphertext.len().is_multiple_of(16) {
        return Err(AppError::Crypto("v5 ciphertext length invalid".into()));
    }

    let mut derived = derive_key(account, password, salt, m_cost, t_cost, p_cost)?;
    let mut key_enc = [0u8; V5_KEY_ENC_LEN];
    let mut key_mac = [0u8; V5_KEY_MAC_LEN];
    key_enc.copy_from_slice(&derived[..V5_KEY_ENC_LEN]);
    key_mac.copy_from_slice(&derived[V5_KEY_ENC_LEN..V5_KEY_ENC_LEN + V5_KEY_MAC_LEN]);

    // HMAC over blob[..-tag_len]，覆盖 magic+version+argon+salt+iv+ciphertext
    let mut mac = <HmacSm3 as Mac>::new_from_slice(&key_mac)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(&blob[..blob.len() - V5_TAG_LEN]);
    let computed = mac.finalize().into_bytes();

    let auth_ok = computed.ct_eq(tag).unwrap_u8() == 1;
    if !auth_ok {
        derived.zeroize();
        key_enc.zeroize();
        key_mac.zeroize();
        return Err(AppError::InvalidPassword);
    }

    let cipher = Sm4CbcDec::new(GenericArray::from_slice(&key_enc), iv.into());
    let plaintext = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|e| AppError::Crypto(format!("sm4-cbc decrypt: {e}")));

    derived.zeroize();
    key_enc.zeroize();
    key_mac.zeroize();
    plaintext
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_v6() {
        let blob = seal("user", "pwd", b"hello, sm4-gcm!").unwrap();
        let plain = open("user", "pwd", &blob).unwrap();
        assert_eq!(plain, b"hello, sm4-gcm!");
    }

    /// v7 默认写入：magic 必为 `AJOT`，ver 必为 7
    #[test]
    fn seal_writes_v7_magic() {
        let blob = seal("user", "pwd", b"hi").unwrap();
        assert_eq!(&blob[..4], b"AJOT");
        assert_eq!(blob[4], 7);
    }

    /// v6 兼容读：造一个 magic=`ZHMM` ver=6 的 blob，能被 open() 读出
    #[test]
    fn read_legacy_v6_zhmm() {
        let blob = seal_inner(b"ZHMM", 6, "user", "pwd", b"legacy v6").unwrap();
        assert_eq!(&blob[..4], b"ZHMM");
        assert_eq!(blob[4], 6);
        let plain = open("user", "pwd", &blob).unwrap();
        assert_eq!(plain, b"legacy v6");
    }

    #[test]
    fn wrong_password() {
        let blob = seal("user", "good", b"data").unwrap();
        assert!(matches!(
            open("user", "bad", &blob),
            Err(AppError::InvalidPassword)
        ));
    }

    #[test]
    fn wrong_account() {
        let blob = seal("alice", "pwd", b"data").unwrap();
        assert!(matches!(
            open("bob", "pwd", &blob),
            Err(AppError::InvalidPassword)
        ));
    }

    #[test]
    fn tampered_ciphertext() {
        let mut blob = seal("user", "pwd", b"hello").unwrap();
        let mid = V6_HEADER_LEN;
        blob[mid] ^= 0xFF;
        assert!(matches!(
            open("user", "pwd", &blob),
            Err(AppError::InvalidPassword)
        ));
    }

    #[test]
    fn tampered_header() {
        let mut blob = seal("user", "pwd", b"hello").unwrap();
        blob[5] ^= 0x01; // 篡改 m_cost 高字节
                         // 篡改 m_cost 后参数可能仍有效但解密 tag 不匹配
        let res = open("user", "pwd", &blob);
        assert!(res.is_err());
    }

    #[test]
    fn magic_mismatch() {
        let blob = vec![0u8; 100];
        assert!(open("u", "p", &blob).is_err());
    }

    /// 拒绝未知 magic/version 组合（如 `AJOT`+ver5、`ZHMM`+ver7）
    #[test]
    fn rejects_unknown_magic_version_combo() {
        let mut blob = seal("u", "p", b"x").unwrap();
        blob[4] = 5; // AJOT + ver5 不合法
        assert!(open("u", "p", &blob).is_err());

        let mut blob2 = seal_inner(b"ZHMM", 6, "u", "p", b"x").unwrap();
        blob2[4] = 7; // ZHMM + ver7 不合法
        assert!(open("u", "p", &blob2).is_err());
    }

    #[test]
    fn unicode_account_password() {
        let blob = seal("张三", "中文密码🔑", b"plaintext").unwrap();
        let plain = open("张三", "中文密码🔑", &blob).unwrap();
        assert_eq!(plain, b"plaintext");
    }
}
