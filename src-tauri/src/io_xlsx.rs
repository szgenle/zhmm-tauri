//! Excel xlsx 导入导出（与 zhmm Python 版 CN_HEADS 兼容）
//!
//! - 表头与列序对齐 zhmm/core/export_service.py CN_HEADS
//! - 转义规则 \r↔[r]、\n↔[n] 与 Python 版一致
//! - 标签用 ; 分隔
//! - history 与 totp_secret 故意不导出（避免明文扩散）

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::Workbook;

use crate::errors::{AppError, AppResult};
use crate::models::{normalize_tags, PasswordEntry};

/// 13 列表头，与 Python 版 CN_HEADS 一致
const CN_HEADS: &[&str] = &[
    "ID",
    "类别",
    "账号",
    "密码",
    "手机",
    "邮箱",
    "网站",
    "备注",
    "更新时间",
    "TOTP算法",
    "TOTP位数",
    "TOTP周期",
    "标签",
];

/// 核心列：导入时仅校验前 9 列
const CORE_HEADS: &[&str] = &[
    "ID", "类别", "账号", "密码", "手机", "邮箱", "网站", "备注", "更新时间",
];

const TAG_SEP: &str = ";";

fn escape(s: &str) -> String {
    s.replace('\r', "[r]").replace('\n', "[n]")
}

fn unescape(s: &str) -> String {
    s.replace("[r]", "\r").replace("[n]", "\n")
}

/// 把条目导出为 xlsx
pub fn export_xlsx(path: &Path, entries: &[PasswordEntry]) -> AppResult<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet().set_name("密码数据").map_err(xlsx_err)?;
    for (col, h) in CN_HEADS.iter().enumerate() {
        ws.write_string(0, col as u16, *h).map_err(xlsx_err)?;
    }
    for (idx, e) in entries.iter().enumerate() {
        let r = (idx + 1) as u32;
        let cells: [String; 13] = [
            e.id.clone(),
            e.role.clone(),
            e.username.clone(),
            e.password.clone(),
            e.phone.clone(),
            e.email.clone(),
            e.url.clone(),
            e.notes.clone(),
            e.updated_at.to_rfc3339(),
            e.totp_algo.clone(),
            e.totp_digits.to_string(),
            e.totp_period.to_string(),
            e.tags.join(TAG_SEP),
        ];
        for (col, v) in cells.iter().enumerate() {
            ws.write_string(r, col as u16, escape(v)).map_err(xlsx_err)?;
        }
    }
    wb.save(path).map_err(xlsx_err)?;
    Ok(())
}

/// 从 xlsx 导入条目（id 字段会重新生成 UUID，避免冲突；history 总是空）
pub fn import_xlsx(path: &Path) -> AppResult<Vec<PasswordEntry>> {
    if !path.exists() {
        return Err(AppError::Other(format!("文件不存在: {}", path.display())));
    }
    let mut wb =
        open_workbook_auto(path).map_err(|e| AppError::Other(format!("打开 xlsx 失败: {e}")))?;
    let sheet_name = wb
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Invalid("xlsx 没有工作表".into()))?;
    let range = wb
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Other(format!("读取工作表失败: {e}")))?;

    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or_else(|| AppError::Invalid("xlsx 缺少表头".into()))?;
    let headers: Vec<String> = header.iter().map(cell_to_string).collect();
    for h in CORE_HEADS {
        if !headers.iter().any(|x| x == h) {
            return Err(AppError::Invalid(format!("缺少必需列: {h}")));
        }
    }
    let idx_of = |name: &str| -> Option<usize> { headers.iter().position(|x| x == name) };

    let mut out = Vec::new();
    for row in rows {
        let all_empty = row.iter().all(|c| matches!(c, Data::Empty));
        if all_empty {
            continue;
        }
        let mut e = PasswordEntry::new(""); // 暂用空 title，后面填
        e.role = read_str(row, idx_of("类别"));
        e.username = read_str(row, idx_of("账号"));
        e.password = read_str(row, idx_of("密码"));
        // 手机号若被读成 float，去掉小数尾
        e.phone = read_phone(row, idx_of("手机"));
        e.email = read_str(row, idx_of("邮箱"));
        e.url = read_str(row, idx_of("网站"));
        e.notes = read_str(row, idx_of("备注"));

        // title 不在 CN_HEADS 里（Python 版没有 title 列），使用账号或网站推一个
        e.title = if !e.username.is_empty() {
            e.username.clone()
        } else if !e.url.is_empty() {
            e.url.clone()
        } else {
            "导入条目".into()
        };

        // 可选 TOTP 列（注意：secret 不在表里，无法导入）
        if let Some(i) = idx_of("TOTP算法") {
            e.totp_algo = cell_to_string(&row[i]);
        }
        if let Some(i) = idx_of("TOTP位数") {
            e.totp_digits = parse_int(&row[i]).unwrap_or(6) as u8;
        }
        if let Some(i) = idx_of("TOTP周期") {
            e.totp_period = parse_int(&row[i]).unwrap_or(30) as u32;
        }

        // 标签：分号分隔
        if let Some(i) = idx_of("标签") {
            let raw = unescape(&cell_to_string(&row[i]));
            let parts: Vec<String> = raw
                .split(TAG_SEP)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            e.tags = normalize_tags(&parts);
        }
        out.push(e);
    }
    Ok(out)
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => d.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{e:?}"),
    }
}

fn read_str(row: &[Data], idx: Option<usize>) -> String {
    match idx {
        Some(i) if i < row.len() => unescape(&cell_to_string(&row[i])),
        _ => String::new(),
    }
}

fn read_phone(row: &[Data], idx: Option<usize>) -> String {
    let raw = read_str(row, idx);
    if let Some(stripped) = raw.strip_suffix(".0") {
        stripped.to_string()
    } else {
        raw
    }
}

fn parse_int(cell: &Data) -> Option<i64> {
    match cell {
        Data::Int(i) => Some(*i),
        Data::Float(f) => Some(*f as i64),
        Data::String(s) => s.trim().parse::<i64>().ok().or_else(|| s.trim().parse::<f64>().ok().map(|f| f as i64)),
        _ => None,
    }
}

fn xlsx_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Other(format!("xlsx 错误: {e}"))
}
