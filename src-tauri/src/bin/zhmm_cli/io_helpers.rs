//! IO 与展示辅助：交互输入、随机密码、表格/详情打印

use zhmm_tauri_lib::{
    errors::{AppError, AppResult},
    models::{PasswordEntry, PasswordSummary},
};

pub fn prompt_password(prompt: &str) -> AppResult<String> {
    rpassword::prompt_password(prompt).map_err(|e| AppError::Other(format!("读取密码失败: {e}")))
}

pub fn read_password_confirm(env_pwd: Option<&str>) -> AppResult<String> {
    if let Some(p) = env_pwd {
        if p.is_empty() {
            return Err(AppError::Invalid("主密码不能为空".into()));
        }
        return Ok(p.to_string());
    }
    read_password_confirm_msg("主密码: ", "再次输入: ")
}

pub fn read_password_confirm_msg(p1: &str, p2: &str) -> AppResult<String> {
    let a = prompt_password(p1)?;
    let b = prompt_password(p2)?;
    if a != b {
        return Err(AppError::Invalid("两次输入不一致".into()));
    }
    if a.is_empty() {
        return Err(AppError::Invalid("主密码不能为空".into()));
    }
    Ok(a)
}

pub fn gen_random_password(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*-_=+";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let i = rng.gen_range(0..CHARS.len());
            CHARS[i] as char
        })
        .collect()
}

pub fn print_list(items: &[PasswordSummary]) {
    if items.is_empty() {
        println!("(无匹配条目)");
        return;
    }
    println!(
        "{:<12} {:<8} {:<22} {:<30} {:<5} DESC",
        "ID", "ROLE", "USER", "URL", "TOTP"
    );
    println!("{}", "-".repeat(100));
    for i in items {
        println!(
            "{:<12} {:<8} {:<22} {:<30} {:<5} {}",
            i.id,
            ellipsize(&i.role, 8),
            ellipsize(&i.user_id, 22),
            ellipsize(&i.url, 30),
            if i.has_totp { "✓" } else { "" },
            ellipsize(&i.desc, 40),
        );
    }
    println!("\n共 {} 条", items.len());
}

pub fn print_entry(e: &PasswordEntry) {
    println!("ID:       {}", e.id);
    println!("Role:     {}", e.role);
    println!("User:     {}", e.user_id);
    println!("Password: {}", e.pwd);
    println!("Phone:    {}", e.phone);
    println!("Email:    {}", e.email);
    println!("URL:      {}", e.url);
    println!("Desc:     {}", e.desc);
    println!(
        "Tags:     {}",
        if e.tags.is_empty() {
            "—".to_string()
        } else {
            e.tags.join(", ")
        }
    );
    println!(
        "TOTP:     {}",
        if e.totp_secret.is_empty() {
            "—"
        } else {
            "✓ 已配置（用 `zhmm-cli totp <id>` 取码）"
        }
    );
    println!("History:  {} 条", e.history.len());
}

/// 简单按字符数截断（CJK 显示宽度不一定准，CLI 场景可接受）
pub fn ellipsize(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        s.to_string()
    } else {
        let take = n.saturating_sub(1);
        let mut out: String = chars[..take].iter().collect();
        out.push('…');
        out
    }
}
