//! 加密层：与 Python 版完全互通的 ZHMM 文件格式
//!
//! - v6（默认写）：Argon2id KDF（材料 = `account || \x00 || password`）+ SM4-GCM AEAD
//! - v5（仅读）：Argon2id KDF（同上）+ SM4-CBC + HMAC-SM3
//!
//! v6 文件头（45B）：magic(4="ZHMM") + ver(1=6) + m_cost(4BE) + t_cost(4BE) + p_cost(4BE)
//!                + salt(16) + iv(12)，header 全部纳入 GCM AAD。
//! v5 文件头（49B）：iv(16)、tag(32) HMAC-SM3 over blob[..-tag_len]。
//!
//! Rust 生态没有现成 SM4-GCM crate，本模块基于 RustCrypto `sm4` 单块加密原语
//! 自实现 GCM（CTR 流 + GHASH 认证），仅支持 96-bit IV 与 128-bit tag。

use argon2::{Algorithm, Argon2, Params, Version};
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sm3::Sm3;
use sm4::cipher::generic_array::GenericArray;
use sm4::cipher::{BlockEncrypt, KeyInit};
use sm4::Sm4;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

use crate::errors::{AppError, AppResult};

// ---------- 协议常量 ----------

const MAGIC: &[u8; 4] = b"ZHMM";
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

// GCM 约化多项式（NIST SP 800-38D）：R = 0xE1 || 0^120
const GHASH_R: u128 = 0xE100_0000_0000_0000_0000_0000_0000_0000;

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

// ---------- SM4 单块加密原语 ----------

fn sm4_encrypt_block(key: &[u8; SM4_KEY_LEN], block: &[u8; 16]) -> [u8; 16] {
    let cipher = Sm4::new(GenericArray::from_slice(key));
    let mut buf = *block;
    let arr = GenericArray::from_mut_slice(&mut buf);
    cipher.encrypt_block(arr);
    buf
}

// ---------- GCM 内部实现（CTR + GHASH） ----------

fn gf128_mul(x: u128, y: u128) -> u128 {
    // NIST SP 800-38D 位序：bit 127 是最高位
    let mut z: u128 = 0;
    let mut v = y;
    for i in 0..128 {
        if (x >> (127 - i)) & 1 == 1 {
            z ^= v;
        }
        if v & 1 == 1 {
            v = (v >> 1) ^ GHASH_R;
        } else {
            v >>= 1;
        }
    }
    z
}

fn ghash(h: &[u8; 16], data: &[u8]) -> [u8; 16] {
    debug_assert!(data.len().is_multiple_of(16));
    let h_int = u128::from_be_bytes(*h);
    let mut y: u128 = 0;
    for chunk in data.chunks(16) {
        let block = u128::from_be_bytes(chunk.try_into().unwrap());
        y = gf128_mul(y ^ block, h_int);
    }
    y.to_be_bytes()
}

fn inc32(counter: &mut [u8; 16]) {
    let v = u32::from_be_bytes(counter[12..16].try_into().unwrap()).wrapping_add(1);
    counter[12..16].copy_from_slice(&v.to_be_bytes());
}

fn sm4_ctr_xor(key: &[u8; SM4_KEY_LEN], icb: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut counter = *icb;
    for chunk in data.chunks(16) {
        let ks = sm4_encrypt_block(key, &counter);
        for (i, b) in chunk.iter().enumerate() {
            out.push(b ^ ks[i]);
        }
        inc32(&mut counter);
    }
    out
}

fn ghash_pad(b: &[u8]) -> Vec<u8> {
    let r = b.len() % 16;
    if r == 0 {
        b.to_vec()
    } else {
        let mut v = b.to_vec();
        v.resize(b.len() + 16 - r, 0);
        v
    }
}

fn build_ghash_input(aad: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let mut buf = ghash_pad(aad);
    buf.extend(ghash_pad(ciphertext));
    buf.extend_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    buf.extend_from_slice(&((ciphertext.len() as u64) * 8).to_be_bytes());
    buf
}

fn sm4_gcm_seal(
    key: &[u8; SM4_KEY_LEN],
    iv: &[u8; IV_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> ([u8; TAG_LEN], Vec<u8>) {
    // H = SM4_ENC(K, 0^128)
    let h = sm4_encrypt_block(key, &[0u8; 16]);
    // 96-bit IV: J0 = IV || 0x00000001
    let mut j0 = [0u8; 16];
    j0[..IV_LEN].copy_from_slice(iv);
    j0[15] = 1;

    // CTR 从 J0+1 开始
    let mut icb = j0;
    inc32(&mut icb);
    let ciphertext = sm4_ctr_xor(key, &icb, plaintext);

    let s = ghash(&h, &build_ghash_input(aad, &ciphertext));
    let ek_j0 = sm4_encrypt_block(key, &j0);
    let mut tag = [0u8; TAG_LEN];
    for i in 0..TAG_LEN {
        tag[i] = s[i] ^ ek_j0[i];
    }
    (tag, ciphertext)
}

fn sm4_gcm_open(
    key: &[u8; SM4_KEY_LEN],
    iv: &[u8; IV_LEN],
    aad: &[u8],
    ciphertext: &[u8],
    tag: &[u8; TAG_LEN],
) -> AppResult<Vec<u8>> {
    let h = sm4_encrypt_block(key, &[0u8; 16]);
    let mut j0 = [0u8; 16];
    j0[..IV_LEN].copy_from_slice(iv);
    j0[15] = 1;

    let s = ghash(&h, &build_ghash_input(aad, ciphertext));
    let ek_j0 = sm4_encrypt_block(key, &j0);
    let mut expected = [0u8; TAG_LEN];
    for i in 0..TAG_LEN {
        expected[i] = s[i] ^ ek_j0[i];
    }
    if expected.ct_eq(tag).unwrap_u8() != 1 {
        return Err(AppError::InvalidPassword);
    }

    let mut icb = j0;
    inc32(&mut icb);
    Ok(sm4_ctr_xor(key, &icb, ciphertext))
}

// ---------- 公开 API ----------

/// 用 (account, password) 加密明文，输出 v6 blob
pub fn seal(account: &str, password: &str, plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut iv);

    let m_cost = ARGON2_M_COST;
    let t_cost = ARGON2_T_COST;
    let p_cost = ARGON2_P_COST;

    let mut header = Vec::with_capacity(V6_HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION_V6);
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

/// 用 (account, password) 解密 blob，按版本分发
pub fn open(account: &str, password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() < 5 || &blob[..4] != MAGIC {
        return Err(AppError::Crypto("not a zhmm vault (magic mismatch)".into()));
    }
    let version = blob[4];
    match version {
        VERSION_V6 => open_v6(account, password, blob),
        VERSION_V5 => open_v5(account, password, blob),
        v => Err(AppError::Crypto(format!("unsupported vault version: {v}"))),
    }
}

fn open_v6(account: &str, password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() < V6_MIN_BLOB_LEN {
        return Err(AppError::Crypto(format!(
            "v6 blob too short: {}",
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

    #[test]
    fn unicode_account_password() {
        let blob = seal("张三", "中文密码🔑", b"plaintext").unwrap();
        let plain = open("张三", "中文密码🔑", &blob).unwrap();
        assert_eq!(plain, b"plaintext");
    }
}
