//! zhmm-cli：基于国密 SM3/SM4 的 .zmb 密码库命令行客户端
//!
//! 直接复用 Tauri 版核心模块（`vault`/`models`/`crypto`/`totp`/`io_xlsx`），
//! 与 GUI 端共享同一份密库格式，互通。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use zhmm_tauri_lib::{
    errors::{AppError, AppResult},
    io_xlsx,
    models::{PasswordEntry, PasswordInput, PasswordSummary, DEFAULT_ROLE},
    totp,
    vault::VaultState,
};

#[derive(Parser, Debug)]
#[command(
    name = "zhmm-cli",
    version,
    about = "zhmm 国密密码管理器命令行版（与 .zmb 密库互通）",
    long_about = None,
)]
struct Cli {
    /// 密码库文件路径（.zmb），可用环境变量 ZHMM_FILE
    #[arg(short = 'f', long, env = "ZHMM_FILE", global = true)]
    file: Option<PathBuf>,

    /// 账号名（与主密码共同派生密钥），可用环境变量 ZHMM_ACCOUNT
    #[arg(short = 'a', long, env = "ZHMM_ACCOUNT", global = true)]
    account: Option<String>,

    /// 主密码（建议改用环境变量 ZHMM_PASSWORD；不传则交互输入）
    #[arg(
        long,
        env = "ZHMM_PASSWORD",
        global = true,
        hide_env_values = true
    )]
    password: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 创建新密码库
    Init,

    /// 列出条目（轻量视图，不含密码）
    List {
        /// 按 role 过滤
        #[arg(short = 'r', long)]
        role: Option<String>,
        /// 按 tag 过滤
        #[arg(short = 't', long)]
        tag: Option<String>,
        /// 关键字搜索（user/url/desc）
        #[arg(short = 'q', long)]
        query: Option<String>,
    },

    /// 显示某条目（含密码）
    Get {
        /// 条目 id 或关键字（按 user/url/desc 子串模糊匹配，唯一命中时返回）
        query: String,
        /// 仅输出密码，便于管道使用：`zhmm-cli get github -p | pbcopy`
        #[arg(short = 'p', long)]
        password_only: bool,
    },

    /// 生成 TOTP 验证码
    Totp {
        /// 条目 id 或关键字
        query: String,
    },

    /// 新增条目
    Add {
        /// 用户名/账号
        #[arg(short = 'u', long)]
        user: String,
        /// 网址
        #[arg(long)]
        url: Option<String>,
        /// 备注
        #[arg(long)]
        desc: Option<String>,
        /// 手机号
        #[arg(long)]
        phone: Option<String>,
        /// 邮箱
        #[arg(long)]
        email: Option<String>,
        /// 分类（默认"个人"）
        #[arg(short = 'r', long)]
        role: Option<String>,
        /// 标签（逗号分隔）
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// 条目密码（不传则交互输入；输入空则随机生成 16 位）
        #[arg(long)]
        pwd: Option<String>,
    },

    /// 删除条目
    Del {
        /// 条目 id 或关键字
        query: String,
    },

    /// 导出为 xlsx
    ExportXlsx {
        /// 输出文件路径
        out: PathBuf,
    },

    /// 从 xlsx 批量导入
    ImportXlsx {
        /// 输入文件路径
        input: PathBuf,
    },

    /// 在密库同目录的 .backups/ 下创建一份备份
    Backup,

    /// 修改主密码（自动产生 rekey_*.zhmm 保险备份）
    Rekey,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✗ {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> AppResult<()> {
    let file = cli
        .file
        .clone()
        .ok_or_else(|| AppError::Invalid("缺少 -f/--file（或 ZHMM_FILE 环境变量）".into()))?;
    let account = cli
        .account
        .clone()
        .ok_or_else(|| AppError::Invalid("缺少 -a/--account（或 ZHMM_ACCOUNT 环境变量）".into()))?;

    match cli.cmd {
        Cmd::Init => {
            if file.exists() {
                return Err(AppError::Other(format!("文件已存在: {}", file.display())));
            }
            let pwd = read_password_confirm(cli.password.as_deref())?;
            let state = VaultState::new();
            state.create(&file, &account, &pwd)?;
            println!("✓ 已创建密码库: {}", file.display());
        }

        Cmd::List { role, tag, query } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let mut items = state.list()?;
            if let Some(r) = role.as_deref() {
                items.retain(|i| i.role == r);
            }
            if let Some(t) = tag.as_deref() {
                items.retain(|i| i.tags.iter().any(|x| x == t));
            }
            if let Some(q) = query.as_deref() {
                let q_low = q.to_lowercase();
                items.retain(|i| {
                    i.user_id.to_lowercase().contains(&q_low)
                        || i.url.to_lowercase().contains(&q_low)
                        || i.desc.to_lowercase().contains(&q_low)
                });
            }
            print_list(&items);
        }

        Cmd::Get { query, password_only } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            if password_only {
                // -p 仍要求唯一命中：方便 `... | pbcopy`
                let entry = resolve_entry(&state, &query)?;
                print!("{}", entry.pwd);
            } else {
                // 普通 get：唯一命中显示该条；多条命中全部显示（含密码）
                let entries = resolve_entries(&state, &query)?;
                let n = entries.len();
                if n > 1 {
                    eprintln!("⚠ 匹配到 {n} 条，全部显示如下：\n");
                }
                for (idx, e) in entries.iter().enumerate() {
                    if idx > 0 {
                        println!("{}", "-".repeat(60));
                    }
                    print_entry(e);
                }
            }
        }

        Cmd::Totp { query } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let entry = resolve_entry(&state, &query)?;
            if entry.totp_secret.is_empty() {
                return Err(AppError::Invalid(format!(
                    "条目 {} 未配置 TOTP",
                    entry.id
                )));
            }
            let algo = if entry.totp_algo.is_empty() {
                "SHA1"
            } else {
                entry.totp_algo.as_str()
            };
            let digits = if entry.totp_digits == 0 {
                6
            } else {
                entry.totp_digits
            };
            let period = if entry.totp_period == 0 {
                30
            } else {
                entry.totp_period
            };
            let code = totp::generate(&entry.totp_secret, algo, digits, period, None)?;
            let remain = totp::remaining_seconds(period, None)?;
            println!("{code}    （剩余 {remain}s）");
        }

        Cmd::Add {
            user,
            url,
            desc,
            phone,
            email,
            role,
            tags,
            pwd,
        } => {
            // 先准备密码（不解锁也能交互输入，避免误密码后还要重输）
            let pwd_input = match pwd {
                Some(p) => p,
                None => prompt_password("条目密码（留空表示随机生成 16 位）: ")?,
            };
            let final_pwd = if pwd_input.is_empty() {
                gen_random_password(16)
            } else {
                pwd_input
            };

            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let input = PasswordInput {
                role: role.unwrap_or_else(|| DEFAULT_ROLE.to_string()),
                user_id: user,
                pwd: final_pwd,
                phone: phone.unwrap_or_default(),
                email: email.unwrap_or_default(),
                url: url.unwrap_or_default(),
                desc: desc.unwrap_or_default(),
                tags,
                totp_secret: String::new(),
                totp_algo: String::new(),
                totp_digits: 6,
                totp_period: 30,
            };
            let entry = state.add(input)?;
            println!(
                "✓ 已添加 id={} role={} user={}",
                entry.id, entry.role, entry.user_id
            );
        }

        Cmd::Del { query } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let entry = resolve_entry(&state, &query)?;
            let id = entry.id;
            state.remove(id)?;
            println!("✓ 已删除 id={id} user={}", entry.user_id);
        }

        Cmd::ExportXlsx { out } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let snap = state.snapshot()?;
            io_xlsx::export_xlsx(&out, &snap.entries)?;
            println!(
                "✓ 已导出 {} 条 -> {}",
                snap.entries.len(),
                out.display()
            );
        }

        Cmd::ImportXlsx { input } => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let entries = io_xlsx::import_xlsx(&input)?;
            let n = state.extend_entries(entries)?;
            println!("✓ 已导入 {n} 条");
        }

        Cmd::Backup => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            let name = state.create_local_backup()?;
            println!("✓ 已创建备份: {name}");
        }

        Cmd::Rekey => {
            let state = unlock(cli.password.as_deref(), &file, &account)?;
            // unlock 时已校验过老密码（不正确根本进不来），这里仍要求显式输入新密码
            let old = match cli.password.clone() {
                Some(p) => p,
                None => prompt_password("当前主密码: ")?,
            };
            let new = read_password_confirm_msg("新主密码: ", "再次输入新主密码: ")?;
            let backup = state.rekey(&old, &new)?;
            println!("✓ 主密码已更换，保险备份: {backup}");
        }
    }

    Ok(())
}

// ============== 辅助函数 ==============

/// 解析用户输入为具体条目（要求唯一命中，用于 totp/del/get -p）。
/// 1. 纯数字且能记录 id 精确命中 → 返回该条目
/// 2. 否则按 user_id / url / desc 子串不区分大小写匹配
///    - 0 条 → 报错
///    - 1 条 → 返回
///    - >1 条 → 列出候选让用户细化
fn resolve_entry(state: &VaultState, query: &str) -> AppResult<PasswordEntry> {
    let q = query.trim();
    if q.is_empty() {
        return Err(AppError::Invalid("查询字串为空".into()));
    }
    // 1) 先试 id
    if let Ok(id) = q.parse::<i64>() {
        if let Ok(entry) = state.get(id) {
            return Ok(entry);
        }
    }
    // 2) 子串搜索
    let q_low = q.to_lowercase();
    let items = state.list()?;
    let hits: Vec<&PasswordSummary> = items
        .iter()
        .filter(|i| {
            i.user_id.to_lowercase().contains(&q_low)
                || i.url.to_lowercase().contains(&q_low)
                || i.desc.to_lowercase().contains(&q_low)
        })
        .collect();
    match hits.len() {
        0 => Err(AppError::Other(format!(
            "找不到与 “{q}” 匹配的条目（请试 zhmm-cli list 查看现有条目）"
        ))),
        1 => state.get(hits[0].id),
        n => {
            eprintln!("⚠ 匹配到 {n} 条，请进一步细化关键字或直接传 id：");
            for i in &hits {
                eprintln!(
                    "  id={:<12} role={:<4} user={:<24} url={}",
                    i.id,
                    ellipsize(&i.role, 4),
                    ellipsize(&i.user_id, 24),
                    i.url
                );
            }
            Err(AppError::Invalid(format!("“{q}” 不唯一")))
        }
    }
}

/// 解析用户输入为一或多条完整条目（用于 get 多命中全部展示）。
/// 规则同 resolve_entry，但多条命中不报错、全部返回。
fn resolve_entries(state: &VaultState, query: &str) -> AppResult<Vec<PasswordEntry>> {
    let q = query.trim();
    if q.is_empty() {
        return Err(AppError::Invalid("查询字串为空".into()));
    }
    // 1) 先试 id。纯数字命中 → 只返该条，不再做子串搜
    if let Ok(id) = q.parse::<i64>() {
        if let Ok(entry) = state.get(id) {
            return Ok(vec![entry]);
        }
    }
    // 2) 子串搜索
    let q_low = q.to_lowercase();
    let items = state.list()?;
    let hit_ids: Vec<i64> = items
        .iter()
        .filter(|i| {
            i.user_id.to_lowercase().contains(&q_low)
                || i.url.to_lowercase().contains(&q_low)
                || i.desc.to_lowercase().contains(&q_low)
        })
        .map(|i| i.id)
        .collect();
    if hit_ids.is_empty() {
        return Err(AppError::Other(format!(
            "找不到与 “{q}” 匹配的条目（请试 zhmm-cli list 查看现有条目）"
        )));
    }
    let mut entries = Vec::with_capacity(hit_ids.len());
    for id in hit_ids {
        entries.push(state.get(id)?);
    }
    Ok(entries)
}

fn unlock(password: Option<&str>, file: &Path, account: &str) -> AppResult<VaultState> {
    let pwd = match password {
        Some(p) => p.to_string(),
        None => prompt_password("主密码: ")?,
    };
    let state = VaultState::new();
    state.unlock_with_path(file, account, &pwd)?;
    Ok(state)
}

fn prompt_password(prompt: &str) -> AppResult<String> {
    rpassword::prompt_password(prompt)
        .map_err(|e| AppError::Other(format!("读取密码失败: {e}")))
}

fn read_password_confirm(env_pwd: Option<&str>) -> AppResult<String> {
    if let Some(p) = env_pwd {
        if p.is_empty() {
            return Err(AppError::Invalid("主密码不能为空".into()));
        }
        return Ok(p.to_string());
    }
    read_password_confirm_msg("主密码: ", "再次输入: ")
}

fn read_password_confirm_msg(p1: &str, p2: &str) -> AppResult<String> {
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

fn gen_random_password(len: usize) -> String {
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

fn print_list(items: &[PasswordSummary]) {
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

fn print_entry(e: &PasswordEntry) {
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
    println!(
        "History:  {} 条",
        e.history.len()
    );
}

/// 简单按字符数截断（CJK 显示宽度不一定准，CLI 场景可接受）
fn ellipsize(s: &str, n: usize) -> String {
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
