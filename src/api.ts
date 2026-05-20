/**
 * 与 Rust 后端的所有调用封装。
 */
import { invoke } from "@tauri-apps/api/core";

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
}

export interface PasswordHistoryItem {
  pwd: string;
  replaced_at: string;
}

export interface PasswordSummary {
  id: string;
  title: string;
  role: string;
  username: string;
  phone: string;
  email: string;
  url: string;
  tags: string[];
  has_totp: boolean;
  updated_at: string;
}

export interface PasswordEntry {
  id: string;
  title: string;
  role: string;
  username: string;
  password: string;
  phone: string;
  email: string;
  url: string;
  notes: string;
  tags: string[];
  totp_secret: string;
  totp_algo: string;
  totp_digits: number;
  totp_period: number;
  history: PasswordHistoryItem[];
  created_at: string;
  updated_at: string;
}

export interface PasswordInput {
  title: string;
  role?: string;
  username?: string;
  password?: string;
  phone?: string;
  email?: string;
  url?: string;
  notes?: string;
  tags?: string[];
  totp_secret?: string;
  totp_algo?: string;
  totp_digits?: number;
  totp_period?: number;
}

export const api = {
  vaultStatus(): Promise<VaultStatus> {
    return invoke("vault_status");
  },
  createVault(masterPassword: string): Promise<void> {
    return invoke("create_vault", { masterPassword });
  },
  unlockVault(masterPassword: string): Promise<void> {
    return invoke("unlock_vault", { masterPassword });
  },
  lockVault(): Promise<void> {
    return invoke("lock_vault");
  },
  listPasswords(): Promise<PasswordSummary[]> {
    return invoke("list_passwords");
  },
  getPassword(id: string): Promise<PasswordEntry> {
    return invoke("get_password", { id });
  },
  addPassword(input: PasswordInput): Promise<PasswordEntry> {
    return invoke("add_password", { input });
  },
  deletePassword(id: string): Promise<void> {
    return invoke("delete_password", { id });
  },
  updatePassword(id: string, input: PasswordInput): Promise<PasswordEntry> {
    return invoke("update_password", { id, input });
  },
  getPasswordHistory(id: string): Promise<PasswordHistoryItem[]> {
    return invoke("get_password_history", { id });
  },
  generateTotp(id: string): Promise<TotpCode> {
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
  rollbackPassword(id: string, historyIndex: number): Promise<PasswordEntry> {
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
