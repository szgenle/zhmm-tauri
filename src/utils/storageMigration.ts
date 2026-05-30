/**
 * localStorage 前缀迁移：zhmm_* / zhmm:* → ajot_* / ajot:*
 *
 * 背景：v2.0 起产品品牌切换为「账号小本本 · Account Jotter」，
 * 浏览器侧持久化的列配置、主题、近用标签等 key 前缀统一改为 ajot。
 *
 * 设计原则：
 *  - 幂等：以 SENTINEL_KEY 作为完成哨兵，重复启动不再重跑
 *  - 安全：仅当目标 key 未占用时写入，避免覆盖用户在新前缀下的设置
 *  - 静默：localStorage 不可用（隐私模式等）时直接吞错，不影响应用启动
 */

const SENTINEL_KEY = "__ajot_migrated_from_zhmm__";

/** 固定 key 一一映射（旧 → 新） */
const FIXED_KEYS = [
  "zhmm_visual_style",
  "zhmm_visible_columns_v6",
  "zhmm_visible_columns_v5",
  "zhmm_visible_columns_v4",
  "zhmm_role_mgmt_columns_v1",
] as const;

/** 前缀映射：所有 oldPrefix 开头的 key 统一替换前缀 */
const PREFIX_PAIRS: ReadonlyArray<readonly [string, string]> = [
  ["zhmm:recent-tags:", "ajot:recent-tags:"],
];

export function migrateZhmmToAjot(): void {
  try {
    if (typeof localStorage === "undefined") return;
    if (localStorage.getItem(SENTINEL_KEY) === "1") return;

    // 1. 固定 key 重命名
    for (const oldKey of FIXED_KEYS) {
      const v = localStorage.getItem(oldKey);
      if (v == null) continue;
      const newKey = oldKey.replace(/^zhmm_/, "ajot_");
      if (localStorage.getItem(newKey) == null) {
        localStorage.setItem(newKey, v);
      }
      localStorage.removeItem(oldKey);
    }

    // 2. 前缀重命名（先收集再写，避免遍历过程中索引漂移）
    for (const [oldPrefix, newPrefix] of PREFIX_PAIRS) {
      const toMove: Array<[string, string]> = [];
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k && k.startsWith(oldPrefix)) {
          toMove.push([k, newPrefix + k.slice(oldPrefix.length)]);
        }
      }
      for (const [oldK, newK] of toMove) {
        const v = localStorage.getItem(oldK);
        if (v == null) continue;
        if (localStorage.getItem(newK) == null) {
          localStorage.setItem(newK, v);
        }
        localStorage.removeItem(oldK);
      }
    }

    localStorage.setItem(SENTINEL_KEY, "1");
  } catch {
    // localStorage 不可用：直接放弃迁移，不阻塞启动
  }
}
