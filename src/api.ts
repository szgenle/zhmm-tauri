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
  /** 名称：网站名/App 名等通用主标识（可为空） */
  name: string;
  userID: string;
  phone: string;
  email: string;
  url: string;
  desc: string;
  tags: string[];
  has_totp: boolean;
  utime: number;
  /** 当前密码生效时间；后端从 history[0].utime 或 id 派生 */
  pwd_utime: number;
  /** 关联的模板 id（v2.0+），空串=无模板 */
  template_id?: string;
  /** 是否含扩展字段（v2.0+） */
  has_custom_fields?: boolean;
}

export interface PasswordEntry {
  id: number;
  role: string;
  /** 名称：网站名/App 名等通用主标识（可为空） */
  name: string;
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
  /** 关联的模板 id（v2.0+） */
  template_id?: string;
  /** 扩展字段（v2.0+）；后端默认 BTreeMap<String,String> ，前端以普通对象收发 */
  custom_fields?: Record<string, string>;
}

export interface PasswordInput {
  role?: string;
  name?: string;
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
  /** 关联的模板 id（v2.0+），空串/未传=无模板 */
  template_id?: string;
  /** 扩展字段（v2.0+）；key 为模板 field.key */
  custom_fields?: Record<string, string>;
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
  /**
   * 导出当前密码库为 xlsx（明文落盘）。
   * 后端会先验证 `masterPassword`，错误会以 "主密码错误" 报错。
   */
  exportXlsx(path: string, masterPassword: string): Promise<void> {
    return invoke("export_xlsx", { path, masterPassword });
  },
  importXlsx(path: string): Promise<number> {
    return invoke("import_xlsx", { path });
  },
  /**
   * 加密备份。`backupPassword` 省略或 null/空串时，
   * 后端会使用当前会话的主密码加密备份（一键备份）。
   */
  backupToFile(path: string, backupPassword?: string | null): Promise<void> {
    return invoke("backup_to_file", {
      path,
      backupPassword: backupPassword && backupPassword.length > 0 ? backupPassword : null,
    });
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
  // 账号模板（v2.0+）
  listTemplates(): Promise<AccountTemplate[]> {
    return invoke("list_templates");
  },
  upsertTemplate(template: AccountTemplate): Promise<AccountTemplate> {
    return invoke("upsert_template", { template });
  },
  deleteTemplate(id: string): Promise<void> {
    return invoke("delete_template", { id });
  },
  seedDefaultTemplates(): Promise<number> {
    return invoke("seed_default_templates");
  },
  /**
   * 导出模板为明文 JSON 模板包（v2.0+）。
   * @param ids 可选。为空/未传表示导出全部模板。
   * @returns 实际写入的模板数
   */
  exportTemplatesJson(path: string, ids?: string[]): Promise<number> {
    return invoke("export_templates_json", {
      path,
      ids: ids && ids.length > 0 ? ids : null,
    });
  },
  /**
   * 从明文 JSON 模板包导入模板。
   * @param overwrite false=merge（id 冲突跳过）；true=overwrite（id 冲突覆盖）
   */
  importTemplatesJson(
    path: string,
    overwrite: boolean,
  ): Promise<TemplateImportResult> {
    return invoke("import_templates_json", { path, overwrite });
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
  // 标签注册表
  listTagRegistry(): Promise<TagDefinition[]> {
    return invoke("list_tag_registry");
  },
  saveTagRegistry(items: TagDefinition[]): Promise<void> {
    return invoke("save_tag_registry", { items });
  },
  upsertTagDef(def: TagDefinition): Promise<void> {
    return invoke("upsert_tag_def", { def });
  },
  removeTagDef(name: string): Promise<void> {
    return invoke("remove_tag_def", { name });
  },
  mergeTags(sources: string[], target: string): Promise<number> {
    return invoke("merge_tags", { sources, target });
  },
  getTagStats(): Promise<TagStats[]> {
    return invoke("get_tag_stats");
  },
  getTagHierarchy(): Promise<TagHierarchyNode[]> {
    return invoke("get_tag_hierarchy");
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
  /** 导出合并后的完整词典到指定路径，返回条目数。
   *  - filterTags 为空数组时导出全部，非空时仅导出含任一指定标签的条目
   *  - excludeTags 排除含任一指定标签的条目
   *  - vaultOverride 为 true 时用密库标签覆盖同 host 词典标签
   *  - scope: "all" 全部 / "used" 仅密库已用 host / "unused" 仅密库未用 host */
  exportSiteCatalog(
    path: string,
    filterTags: string[] = [],
    excludeTags: string[] = [],
    vaultOverride = false,
    scope: "all" | "used" | "unused" = "all",
  ): Promise<number> {
    return invoke("export_site_catalog", {
      path,
      filterTags,
      excludeTags,
      vaultOverride,
      scope,
    });
  },
  /** 列出词典中所有出现过的标签（去重、排序） */
  listCatalogTags(): Promise<string[]> {
    return invoke("list_catalog_tags");
  },
  /** 从 JSON 文件导入为用户词典（完全替换用户层），返回条目数 */
  importSiteCatalog(path: string): Promise<number> {
    return invoke("import_site_catalog", { path });
  },
  /** 重置用户词典，恢复为纯内置 */
  resetSiteCatalog(): Promise<boolean> {
    return invoke("reset_site_catalog");
  },
  /** 用户是否有自定义词典数据 */
  hasUserCatalog(): Promise<boolean> {
    return invoke("has_user_catalog");
  },
  /** 列出全部预制身份词典（base / developer / game-dev / cross-border / creator / small-biz / crypto） */
  listPresetPersonas(): Promise<PresetPersonaInfo[]> {
    return invoke("list_preset_personas");
  },
  /**
   * 把指定预制身份词典并入用户词典层。
   * - overrideTags=false（默认）：同 host 已存在 → 保留用户当前 tags 不变
   * - overrideTags=true：同 host 已存在 → 用预制 name+tags 覆盖
   *
   * @deprecated 请使用 `importPresetPersona`。该方法保留以兼容旧版本。
   */
  importPresetCatalog(
    persona: string,
    overrideTags: boolean,
  ): Promise<PresetImportResult> {
    return invoke("import_preset_catalog", { persona, overrideTags });
  },
  /**
   * 把指定身份的「站点全集 + primary_tags」并入用户词典层。
   *
   * 行为：
   * - 把 sites.json 全集合并进用户词典（按 overrideTags 三态归类）
   * - 把对应 persona 的 primary_tags 追加到用户 primary_tags（去重）
   */
  importPresetPersona(
    personaId: string,
    overrideTags: boolean,
  ): Promise<PresetImportResult> {
    return invoke("import_preset_persona", { personaId, overrideTags });
  },
  /** 列出合并视图（builtin + user）中所有出现过的标签 + 频次 + 是否一级 */
  listCatalogTagStats(): Promise<CatalogTagStat[]> {
    return invoke("list_catalog_tag_stats");
  },
  /** 获取用户选择的一级标签集合（来自用户词典 _meta.primary_tags） */
  getUserPrimaryTags(): Promise<string[]> {
    return invoke("get_user_primary_tags");
  },
  /** 保存用户选择的一级标签集合（覆盖式更新，去重） */
  setUserPrimaryTags(tags: string[]): Promise<void> {
    return invoke("set_user_primary_tags", { tags });
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
  // Favicon 缓存
  cacheFavicon(domain: string): Promise<string> {
    return invoke("cache_favicon", { domain });
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

export interface TagDefinition {
  name: string;
  color: string;
  icon: string;
  order: number;
  /** 来源: "manual"=手动创建, "import"=导入词典, ""=未知/旧数据 */
  source: string;
}

export interface TagStats {
  name: string;
  /** 总使用次数 */
  count: number;
  /** 作为 tags[0]（一级标签）的次数 */
  primary_count: number;
  /** 是否存在于站点词典中 */
  in_catalog: boolean;
}

export interface TagHierarchyNode {
  /** 一级标签名（tags[0]） */
  name: string;
  /** 该一级标签下的子标签列表 */
  children: string[];
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

export interface PresetPersonaInfo {
  /** 身份标识，如 "base" / "game-dev" */
  persona: string;
  /** 词典描述（来自 _meta.description） */
  description: string;
  /** 站点条目数 */
  site_count: number;
  /** 一级标签（来自 _meta.primary_tags） */
  primary_tags: string[];
  /** 是否社区扩展词典（如 crypto） */
  community: boolean;
  /** UI 是否默认勾选 */
  default_enabled: boolean;
}

export interface PresetImportResult {
  /** host 不存在 → 新增 */
  added: number;
  /** host 已存在 && overrideTags=true → 用预制覆盖 */
  overwritten: number;
  /** host 已存在 && overrideTags=false → 保留用户数据 */
  kept: number;
}

export interface CatalogTagStat {
  /** 标签名 */
  tag: string;
  /** 该标签在合并视图中出现的频次 */
  count: number;
  /** 是否被用户选定为一级标签 */
  is_primary: boolean;
}

// ========== 账号模板（v2.0+） ==========

export type TemplateFieldType =
  | "text"
  | "secret"
  | "url"
  | "email"
  | "phone"
  | "multiline"
  | "date";

export interface TemplateField {
  key: string;
  label: string;
  field_type?: TemplateFieldType;
  required?: boolean;
  placeholder?: string;
}

export type TemplateMatchRule =
  | { kind: "url_contains"; value: string }
  | { kind: "keyword"; value: string }
  | { kind: "role"; value: string };

export interface AccountTemplate {
  id: string;
  name: string;
  icon?: string;
  fields: TemplateField[];
  match_rules?: TemplateMatchRule[];
  utime?: number;
}

/** 模板包导入结果统计（v2.0+） */
export interface TemplateImportResult {
  /** 新增模板数 */
  added: number;
  /** 覆盖更新数（仅 overwrite 模式） */
  updated: number;
  /** 跳过数（merge 模式下 id 冲突） */
  skipped: number;
  /** 被丢弃的非法模板数 */
  invalid: number;
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
