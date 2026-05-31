//! zhmm-cli / accjot：账号小本本 · Account Jotter 命令行客户端
//!
//! 直接复用 Tauri 版核心模块（`vault`/`models`/`crypto`/`totp`/`io_xlsx`），
//! 与 GUI 端共享同一份密库格式：V2.0 起默认 v7（magic=`AJOT`），
//! 兼容读取原 Python 版 .zmb（v6/v5）。

mod handlers;
mod io_helpers;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use zhmm_tauri_lib::{
    errors::{AppError, AppResult},
    io_xlsx,
    models::{PasswordInput, DEFAULT_ROLE},
    totp,
    vault::VaultState,
};

use handlers::{resolve_entries, resolve_entry, unlock};
use io_helpers::{
    gen_random_password, print_entry, print_list, prompt_password, read_password_confirm,
    read_password_confirm_msg,
};

#[derive(Parser, Debug)]
#[command(
    name = "zhmm-cli",
    version,
    about = "账号小本本 · Account Jotter 命令行客户端（默认 .ajot v7，兼容 .zmb）",
    long_about = None,
)]
struct Cli {
    /// 密码库文件路径（.ajot 或老后缀 .zmb），可用环境变量 ZHMM_FILE
    #[arg(short = 'f', long, env = "ZHMM_FILE", global = true)]
    file: Option<PathBuf>,

    /// 账号名（与主密码共同派生密钥），可用环境变量 ZHMM_ACCOUNT
    #[arg(short = 'a', long, env = "ZHMM_ACCOUNT", global = true)]
    account: Option<String>,

    /// 主密码（建议改用环境变量 ZHMM_PASSWORD；不传则交互输入）
    #[arg(long, env = "ZHMM_PASSWORD", global = true, hide_env_values = true)]
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

        Cmd::Get {
            query,
            password_only,
        } => {
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
                return Err(AppError::Invalid(format!("条目 {} 未配置 TOTP", entry.id)));
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
                name: String::new(),
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
                template_id: String::new(),
                custom_fields: Default::default(),
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
            println!("✓ 已导出 {} 条 -> {}", snap.entries.len(), out.display());
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
