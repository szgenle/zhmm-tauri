//! SM4-GCM 自实现：单块加密 + GHASH 认证 + CTR 流
//!
//! Rust 生态没有现成 SM4-GCM crate，本子模块基于 RustCrypto `sm4` 单块加密
//! 原语自实现 GCM（CTR 流 + GHASH 认证），仅支持 96-bit IV 与 128-bit tag。
//! 仅供 [`super`] 内部 seal/open 使用。

use sm4::cipher::generic_array::GenericArray;
use sm4::cipher::{BlockEncrypt, KeyInit};
use sm4::Sm4;
use subtle::ConstantTimeEq;

use crate::errors::{AppError, AppResult};

use super::{IV_LEN, SM4_KEY_LEN, TAG_LEN};

// GCM 约化多项式（NIST SP 800-38D）：R = 0xE1 || 0^120
const GHASH_R: u128 = 0xE100_0000_0000_0000_0000_0000_0000_0000;

// ---------- SM4 单块加密原语 ----------

pub(super) fn sm4_encrypt_block(key: &[u8; SM4_KEY_LEN], block: &[u8; 16]) -> [u8; 16] {
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

pub(super) fn sm4_gcm_seal(
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

pub(super) fn sm4_gcm_open(
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
