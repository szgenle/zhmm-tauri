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
};
