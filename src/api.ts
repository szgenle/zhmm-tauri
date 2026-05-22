/**
 * 与 Rust 后端的所有调用封装。
 *
 * 多账号库改造后：路径不再固定，前端通过 createVaultAt / unlockWithPath 指定。
 * 数据模型字段与 Python 版互通（userID / pwd / desc / utime）。
 */
import { invoke } from "@tauri-apps/api/core";

export interface VaultStatus {
  unlocked: boolean;
  current_path: string | null;
  current_account: string | null;
}

export interface PasswordHistoryItem {
  pwd: string;
  utime: number;
}

export interface PasswordSummary {
  id: number;
  role: string;
  userID: string;
  phone: string;
  email: string;
  url: string;
  desc: string;
  tags: string[];
  has_totp: boolean;
  utime: number;
}

export interface PasswordEntry {
  id: number;
  role: string;
  userID: string;
  pwd: string;
  phone: string;
  email: string;
  url: string;
  desc: string;
  tags: string[];
  totp_secret: string;
  totp_algo: string;
  totp_digits: number;
  totp_period: number;
  history: PasswordHistoryItem[];
  utime: number;
}

export interface PasswordInput {
  role?: string;
  userID?: string;
  pwd?: string;
  phone?: string;
  email?: string;
  url?: string;
  desc?: string;
  tags?: string[];
  totp_secret?: string;
  totp_algo?: string;
  totp_digits?: number;
  totp_period?: number;
}

export interface RecentEntry {
  path: string;
  account: string;
  hashpw: string;
  last_access_time: string;
}

export const api = {
  vaultStatus(): Promise<VaultStatus> {
    return invoke("vault_status");
  },
  createVaultAt(path: string, account: string, masterPassword: string): Promise<void> {
    return invoke("create_vault_at", { path, account, masterPassword });
  },
  unlockWithPath(path: string, account: string, masterPassword: string): Promise<void> {
    return invoke("unlock_with_path", { path, account, masterPassword });
  },
  lockVault(): Promise<void> {
    return invoke("lock_vault");
  },
  listPasswords(): Promise<PasswordSummary[]> {
    return invoke("list_passwords");
  },
  getPassword(id: number): Promise<PasswordEntry> {
    return invoke("get_password", { id });
  },
  addPassword(input: PasswordInput): Promise<PasswordEntry> {
    return invoke("add_password", { input });
  },
  deletePassword(id: number): Promise<void> {
    return invoke("delete_password", { id });
  },
  updatePassword(id: number, input: PasswordInput): Promise<PasswordEntry> {
    return invoke("update_password", { id, input });
  },
  getPasswordHistory(id: number): Promise<PasswordHistoryItem[]> {
    return invoke("get_password_history", { id });
  },
  generateTotp(id: number): Promise<TotpCode> {
    return invoke("generate_totp", { id });
  },
  parseOtpauth(uri: string): Promise<OtpAuthParams> {
    return invoke("parse_otpauth", { uri });
  },
  exportXlsx(path: string): Promise<void> {
    return invoke("export_xlsx", { path });
  },
  importXlsx(path: string): Promise<number> {
    return invoke("import_xlsx", { path });
  },
  backupToFile(path: string, backupPassword: string): Promise<void> {
    return invoke("backup_to_file", { path, backupPassword });
  },
  restoreFromFile(path: string, backupPassword: string): Promise<void> {
    return invoke("restore_from_file", { path, backupPassword });
  },
  getSettings(): Promise<AppSettings> {
    return invoke("get_settings");
  },
  updateSettings(newSettings: AppSettings): Promise<AppSettings> {
    return invoke("update_settings", { newSettings });
  },
  listRoles(): Promise<string[]> {
    return invoke("list_roles");
  },
  // 本地备份管理
  createLocalBackup(): Promise<string> {
    return invoke("create_local_backup");
  },
  listLocalBackups(): Promise<BackupInfo[]> {
    return invoke("list_local_backups");
  },
  deleteLocalBackup(name: string): Promise<void> {
    return invoke("delete_local_backup", { name });
  },
  restoreLocalBackup(name: string): Promise<void> {
    return invoke("restore_local_backup", { name });
  },
  cleanupBackups(keep: number): Promise<number> {
    return invoke("cleanup_backups", { keep });
  },
  // 标签管理
  collectTagCounts(): Promise<TagCount[]> {
    return invoke("collect_tag_counts");
  },
  renameTag(old: string, newTag: string): Promise<number> {
    return invoke("rename_tag", { old, new: newTag });
  },
  deleteTag(tag: string): Promise<number> {
    return invoke("delete_tag", { tag });
  },
  // 密码历史回滚
  rollbackPassword(id: number, historyIndex: number): Promise<PasswordEntry> {
    return invoke("rollback_password", { id, historyIndex });
  },
  // 站点词典
  listSiteCatalog(): Promise<SiteCatalogEntry[]> {
    return invoke("list_site_catalog");
  },
  suggestSite(urlOrHost: string): Promise<SiteSuggestion> {
    return invoke("suggest_site", { urlOrHost });
  },
  // 主密码管理
  verifyMasterPassword(password: string): Promise<boolean> {
    return invoke("verify_master_password", { password });
  },
  rekeyVault(oldPassword: string, newPassword: string): Promise<string> {
    return invoke("rekey_vault", { oldPassword, newPassword });
  },
  // 防截屏
  applyAntiCapture(enabled: boolean): Promise<boolean> {
    return invoke("apply_anti_capture", { enabled });
  },
  // xlsx 模板
  exportXlsxTemplate(path: string): Promise<void> {
    return invoke("export_xlsx_template", { path });
  },
  // 最近访问列表
  listRecent(): Promise<RecentEntry[]> {
    return invoke("list_recent");
  },
  upsertRecent(entry: RecentEntry): Promise<void> {
    return invoke("upsert_recent", { entry });
  },
  removeRecent(path: string): Promise<void> {
    return invoke("remove_recent", { path });
  },
  clearRecent(): Promise<void> {
    return invoke("clear_recent");
  },
  // bcrypt（最近访问列表 UI 层快速密码预校验）
  bcryptHash(password: string): Promise<string> {
    return invoke("bcrypt_hash", { password });
  },
  bcryptVerify(password: string, hash: string): Promise<boolean> {
    return invoke("bcrypt_verify", { password, hash });
  },
  // 文件存在性
  pathExists(path: string): Promise<boolean> {
    return invoke("path_exists", { path });
  },
  // 旧版 v1 vault.zmb 不兼容检测
  legacyVaultExists(): Promise<boolean> {
    return invoke("legacy_vault_exists");
  },
};

export interface TotpCode {
  code: string;
  remaining_seconds: number;
}

export interface OtpAuthParams {
  secret: string;
  algo: string;
  digits: number;
  period: number;
  label: string;
  issuer: string;
}

export interface AppSettings {
  theme: string; // "auto" | "light" | "dark"
  auto_lock_minutes: number; // 0 = 不自动锁定
  clipboard_clear_seconds: number; // 0 = 不清空
  anti_screenshot?: boolean;
}

export interface BackupInfo {
  name: string;
  size: number;
  created_at: string;
}

export interface TagCount {
  tag: string;
  count: number;
}

export interface SiteCatalogEntry {
  host: string;
  name: string;
  tags: string[];
}

export interface SiteSuggestion {
  name: string;
  tags: string[];
  matched: string;
}

/**
 * 把秒级时间戳格式化为 "YYYY-MM-DD HH:mm:ss"。
 * 0 / 负数 / NaN 统一返回空串。
 */
export function formatUtime(ts: number | undefined | null): string {
  if (!ts || ts <= 0 || !Number.isFinite(ts)) return "";
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  );
}
