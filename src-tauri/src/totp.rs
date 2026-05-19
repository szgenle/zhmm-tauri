//! TOTP 2FA（RFC 6238）+ 国密 SM3 扩展
//!
//! 与 zhmm Python 版 `zhmm/core/totp.py` 行为一致：
//! - 支持 SHA1 / SHA256 / SHA512 / SM3
//! - SM3 是 zhmm 私有扩展，主流 2FA App 不识别
//! - secret 用 Base32 字符串，允许大小写混用、含空格、缺 = 填充

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use sm3::Sm3;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::{AppError, AppResult};

pub const DEFAULT_ALGO: &str = "SHA1";
pub const DEFAULT_DIGITS: u8 = 6;
pub const DEFAULT_PERIOD: u32 = 30;

const MIN_DIGITS: u8 = 6;
const MAX_DIGITS: u8 = 10;
const MIN_PERIOD: u32 = 1;
const MAX_PERIOD: u32 = 300;

/// 解析 Base32 secret：去空格/横杠、转大写、补 = 填充
pub fn decode_secret(secret_b32: &str) -> AppResult<Vec<u8>> {
    let s: String = secret_b32
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_uppercase();
    if s.is_empty() {
        return Err(AppError::Invalid("totp secret 为空".into()));
    }
    // 自带 padding 的 Base32 解码
    let pad = (8 - s.len() % 8) % 8;
    let padded = format!("{}{}", s, "=".repeat(pad));
    base32_decode(&padded)
}

/// 生成 TOTP 验证码
pub fn generate(
    secret_b32: &str,
    algo: &str,
    digits: u8,
    period: u32,
    now: Option<u64>,
) -> AppResult<String> {
    if !(MIN_DIGITS..=MAX_DIGITS).contains(&digits) {
        return Err(AppError::Invalid(format!("digits out of range: {digits}")));
    }
    if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
        return Err(AppError::Invalid(format!("period out of range: {period}")));
    }
    let key = decode_secret(secret_b32)?;
    let t_now = now.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });
    let counter = t_now / period as u64;
    let msg = counter.to_be_bytes();
    let digest: Vec<u8> = match algo.to_uppercase().as_str() {
        "SHA1" => {
            let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(&key)
                .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        "SHA256" => {
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key)
                .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        "SHA512" => {
            let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(&key)
                .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        "SM3" => {
            let mut mac = <Hmac<Sm3> as Mac>::new_from_slice(&key)
                .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        other => return Err(AppError::Invalid(format!("unsupported algo: {other}"))),
    };
    // RFC 4226 动态截断
    let offset = (digest[digest.len() - 1] & 0x0F) as usize;
    let bin = ((digest[offset] & 0x7F) as u32) << 24
        | (digest[offset + 1] as u32) << 16
        | (digest[offset + 2] as u32) << 8
        | digest[offset + 3] as u32;
    let modulus = 10u32.pow(digits as u32);
    let code = bin % modulus;
    Ok(format!("{:0width$}", code, width = digits as usize))
}

/// 当前时间片距离下一刷新的剩余秒数（至少 1）
pub fn remaining_seconds(period: u32, now: Option<u64>) -> AppResult<u32> {
    if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
        return Err(AppError::Invalid(format!("period out of range: {period}")));
    }
    let t = now.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });
    let elapsed = (t % period as u64) as u32;
    let left = period - elapsed;
    Ok(if left == 0 { period } else { left })
}

/// 解析 otpauth://totp/<label>?secret=...&...
#[derive(Debug, Clone, Serialize)]
pub struct OtpAuthParams {
    pub secret: String,
    pub algo: String,
    pub digits: u8,
    pub period: u32,
    pub label: String,
    pub issuer: String,
}

pub fn parse_otpauth_uri(uri: &str) -> AppResult<OtpAuthParams> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Err(AppError::Invalid("uri 为空".into()));
    }
    let parsed =
        url::Url::parse(uri).map_err(|e| AppError::Invalid(format!("uri 解析失败: {e}")))?;
    if !parsed.scheme().eq_ignore_ascii_case("otpauth") {
        return Err(AppError::Invalid("不是 otpauth uri".into()));
    }
    let host = parsed.host_str().unwrap_or("");
    if !host.eq_ignore_ascii_case("totp") {
        return Err(AppError::Invalid(format!("不支持的 otpauth 类型: {host}")));
    }
    let mut secret = String::new();
    let mut algo = DEFAULT_ALGO.to_string();
    let mut digits: u8 = DEFAULT_DIGITS;
    let mut period: u32 = DEFAULT_PERIOD;
    let mut issuer = String::new();
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "secret" => secret = v.into_owned(),
            "algorithm" => algo = v.to_uppercase(),
            "digits" => {
                digits = v
                    .parse()
                    .map_err(|_| AppError::Invalid(format!("digits 非法: {v}")))?;
            }
            "period" => {
                period = v
                    .parse()
                    .map_err(|_| AppError::Invalid(format!("period 非法: {v}")))?;
            }
            "issuer" => issuer = v.into_owned(),
            _ => {}
        }
    }
    if secret.is_empty() {
        return Err(AppError::Invalid("uri 缺少 secret".into()));
    }
    if !["SHA1", "SHA256", "SHA512", "SM3"].contains(&algo.as_str()) {
        return Err(AppError::Invalid(format!("不支持的 algorithm: {algo}")));
    }
    if !(MIN_DIGITS..=MAX_DIGITS).contains(&digits) {
        return Err(AppError::Invalid(format!("digits 越界: {digits}")));
    }
    if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
        return Err(AppError::Invalid(format!("period 越界: {period}")));
    }
    let label = parsed.path().trim_start_matches('/').to_string();
    let label = urlencoding_decode(&label);
    if issuer.is_empty() {
        if let Some((iss, _)) = label.split_once(':') {
            issuer = iss.trim().to_string();
        }
    }
    Ok(OtpAuthParams {
        secret,
        algo,
        digits,
        period,
        label,
        issuer,
    })
}

fn urlencoding_decode(s: &str) -> String {
    // 简易 URL 解码：%XX -> byte
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                hex_digit(bytes[i + 1]),
                hex_digit(bytes[i + 2]),
            ) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 简易 Base32 (RFC 4648) 解码，输入必须是已补齐 = 的纯 Base32 字符串
fn base32_decode(s: &str) -> AppResult<Vec<u8>> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut buf = Vec::with_capacity(s.len() * 5 / 8);
    let mut bits: u32 = 0;
    let mut nbits: u32 = 0;
    for c in s.chars() {
        if c == '=' {
            break;
        }
        let v = ALPHABET
            .iter()
            .position(|&x| x == c as u8)
            .ok_or_else(|| AppError::Invalid(format!("非法 base32 字符: {c}")))?;
        bits = (bits << 5) | (v as u32);
        nbits += 5;
        if nbits >= 8 {
            nbits -= 8;
            buf.push(((bits >> nbits) & 0xFF) as u8);
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 SHA1 测试向量（secret = ASCII "12345678901234567890"）
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn rfc6238_sha1_vectors() {
        // T=59
        assert_eq!(
            generate(RFC_SECRET, "SHA1", 8, 30, Some(59)).unwrap(),
            "94287082"
        );
        // T=1111111109
        assert_eq!(
            generate(RFC_SECRET, "SHA1", 8, 30, Some(1_111_111_109)).unwrap(),
            "07081804"
        );
        // T=1111111111
        assert_eq!(
            generate(RFC_SECRET, "SHA1", 8, 30, Some(1_111_111_111)).unwrap(),
            "14050471"
        );
        // T=1234567890
        assert_eq!(
            generate(RFC_SECRET, "SHA1", 8, 30, Some(1_234_567_890)).unwrap(),
            "89005924"
        );
    }

    #[test]
    fn sm3_self_consistent() {
        // 同样 secret + 同样时间 -> 同样输出
        let a = generate(RFC_SECRET, "SM3", 6, 30, Some(1_700_000_000)).unwrap();
        let b = generate(RFC_SECRET, "SM3", 6, 30, Some(1_700_000_000)).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
        // 不同时间窗口产生不同码（以 30s 为步长）
        let c = generate(RFC_SECRET, "SM3", 6, 30, Some(1_700_000_030)).unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn parse_otpauth_minimal() {
        let p = parse_otpauth_uri(
            "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example",
        )
        .unwrap();
        assert_eq!(p.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(p.algo, "SHA1");
        assert_eq!(p.digits, 6);
        assert_eq!(p.period, 30);
        assert_eq!(p.issuer, "Example");
        assert!(p.label.contains("alice"));
    }

    #[test]
    fn parse_otpauth_full() {
        let p = parse_otpauth_uri(
            "otpauth://totp/Acme?secret=JBSWY3DPEHPK3PXP&algorithm=SHA256&digits=8&period=60",
        )
        .unwrap();
        assert_eq!(p.algo, "SHA256");
        assert_eq!(p.digits, 8);
        assert_eq!(p.period, 60);
    }

    #[test]
    fn remaining_seconds_works() {
        // T 落在 30 秒边界 -> 还剩完整周期
        assert_eq!(remaining_seconds(30, Some(60)).unwrap(), 30);
        assert_eq!(remaining_seconds(30, Some(61)).unwrap(), 29);
    }
}
