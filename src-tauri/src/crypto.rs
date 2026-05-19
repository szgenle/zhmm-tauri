//! 加密核心：Argon2id 密钥派生 + SM4-CBC 加密 + HMAC-SM3 完整性校验
//!
//! 设计：
//! - Argon2id 从主密码派生 48 字节密钥：前 16 字节 SM4 密钥，后 32 字节 HMAC 密钥
//! - 加密：PKCS7 填充 + SM4-CBC，HMAC-SM3 覆盖 IV || ciphertext（Encrypt-then-MAC）
//! - 验证：恒时间比较 HMAC，再解密 SM4-CBC，去 PKCS7 填充

use argon2::{Algorithm, Argon2, Params, Version};
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sm3::Sm3;
use sm4::Sm4;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::errors::{AppError, AppResult};

type Sm4CbcEnc = cbc::Encryptor<Sm4>;
type Sm4CbcDec = cbc::Decryptor<Sm4>;
type HmacSm3 = Hmac<Sm3>;

pub const SALT_LEN: usize = 16;
pub const IV_LEN: usize = 16;
pub const TAG_LEN: usize = 32;
pub const SM4_KEY_LEN: usize = 16;
pub const HMAC_KEY_LEN: usize = 32;
pub const DERIVED_LEN: usize = SM4_KEY_LEN + HMAC_KEY_LEN;

/// Argon2id 参数（64 MiB / 3 iter / 4 thread），与 Python 版对齐
const ARGON2_MEM_KIB: u32 = 64 * 1024;
const ARGON2_TIME: u32 = 3;
const ARGON2_PARALLEL: u32 = 4;

#[derive(ZeroizeOnDrop)]
pub struct DerivedKey {
    pub sm4_key: [u8; SM4_KEY_LEN],
    pub hmac_key: [u8; HMAC_KEY_LEN],
}

impl DerivedKey {
    pub fn derive(password: &[u8], salt: &[u8]) -> AppResult<Self> {
        let params = Params::new(ARGON2_MEM_KIB, ARGON2_TIME, ARGON2_PARALLEL, Some(DERIVED_LEN))
            .map_err(|e| AppError::Crypto(format!("argon2 params: {e}")))?;
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut out = [0u8; DERIVED_LEN];
        argon
            .hash_password_into(password, salt, &mut out)
            .map_err(|e| AppError::Crypto(format!("argon2 derive: {e}")))?;
        let mut sm4_key = [0u8; SM4_KEY_LEN];
        let mut hmac_key = [0u8; HMAC_KEY_LEN];
        sm4_key.copy_from_slice(&out[..SM4_KEY_LEN]);
        hmac_key.copy_from_slice(&out[SM4_KEY_LEN..]);
        out.zeroize();
        Ok(Self { sm4_key, hmac_key })
    }
}

/// 加密包：salt(16) || iv(16) || ciphertext || hmac(32)
pub struct Envelope {
    pub salt: [u8; SALT_LEN],
    pub iv: [u8; IV_LEN],
    pub ciphertext: Vec<u8>,
    pub tag: [u8; TAG_LEN],
}

impl Envelope {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(SALT_LEN + IV_LEN + self.ciphertext.len() + TAG_LEN);
        buf.extend_from_slice(&self.salt);
        buf.extend_from_slice(&self.iv);
        buf.extend_from_slice(&self.ciphertext);
        buf.extend_from_slice(&self.tag);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> AppResult<Self> {
        let min = SALT_LEN + IV_LEN + TAG_LEN;
        if data.len() < min {
            return Err(AppError::Crypto(format!("加密包长度不足: {}", data.len())));
        }
        let mut salt = [0u8; SALT_LEN];
        let mut iv = [0u8; IV_LEN];
        let mut tag = [0u8; TAG_LEN];
        salt.copy_from_slice(&data[..SALT_LEN]);
        iv.copy_from_slice(&data[SALT_LEN..SALT_LEN + IV_LEN]);
        let ct_end = data.len() - TAG_LEN;
        let ciphertext = data[SALT_LEN + IV_LEN..ct_end].to_vec();
        tag.copy_from_slice(&data[ct_end..]);
        Ok(Self {
            salt,
            iv,
            ciphertext,
            tag,
        })
    }
}

pub fn encrypt(plaintext: &[u8], password: &[u8]) -> AppResult<Envelope> {
    let mut salt = [0u8; SALT_LEN];
    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut iv);

    let key = DerivedKey::derive(password, &salt)?;
    let cipher = Sm4CbcEnc::new(&key.sm4_key.into(), &iv.into());
    let ciphertext = cipher.encrypt_padded_vec_mut::<Pkcs7>(plaintext);

    let mut mac = <HmacSm3 as Mac>::new_from_slice(&key.hmac_key)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(&iv);
    mac.update(&ciphertext);
    let tag_bytes = mac.finalize().into_bytes();
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&tag_bytes);

    Ok(Envelope {
        salt,
        iv,
        ciphertext,
        tag,
    })
}

pub fn decrypt(envelope: &Envelope, password: &[u8]) -> AppResult<Vec<u8>> {
    let key = DerivedKey::derive(password, &envelope.salt)?;

    // 先校验 HMAC（恒时间）
    let mut mac = <HmacSm3 as Mac>::new_from_slice(&key.hmac_key)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(&envelope.iv);
    mac.update(&envelope.ciphertext);
    let computed = mac.finalize().into_bytes();
    if computed.ct_eq(&envelope.tag).unwrap_u8() != 1 {
        // HMAC 不匹配既可能是密码错也可能是数据被篡改
        // 上层结合 vault.zhmm 是否能解出合法 JSON 区分；这里返回 InvalidPassword
        return Err(AppError::InvalidPassword);
    }

    let cipher = Sm4CbcDec::new(&key.sm4_key.into(), &envelope.iv.into());
    let plaintext = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(&envelope.ciphertext)
        .map_err(|e| AppError::Crypto(format!("sm4-cbc decrypt: {e}")))?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pwd = b"correct horse battery staple";
        let plain = b"hello, sm4 world! 123 \xe4\xb8\xad\xe6\x96\x87";
        let env = encrypt(plain, pwd).unwrap();
        let bytes = env.to_bytes();
        let env2 = Envelope::from_bytes(&bytes).unwrap();
        let dec = decrypt(&env2, pwd).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn wrong_password() {
        let env = encrypt(b"data", b"good").unwrap();
        let res = decrypt(&env, b"bad");
        assert!(matches!(res, Err(AppError::InvalidPassword)));
    }

    #[test]
    fn tampered_ciphertext() {
        let mut env = encrypt(b"hello", b"pwd").unwrap();
        env.ciphertext[0] ^= 0xFF;
        let res = decrypt(&env, b"pwd");
        assert!(matches!(res, Err(AppError::InvalidPassword)));
    }
}
