/**
 * 自定义标签规则
 *
 * 用户在账号管理左侧标签栏中可创建“自定义标签”，通过一组关键字（OR 语义）
 * 在条目的 url/host 上做不区分大小写的子串匹配，作为虚拟分组使用。
 *
 * 规则不会写入条目的 tags 字段，只是一种动态视图。
 * 持久化到 localStorage：ajot_custom_tag_rules_v1。
 */
import { ref } from "vue";

export interface CustomTagRule {
  id: string;
  name: string;
  /** 关键字列表，OR 语义；非空字符串才参与匹配；不区分大小写。 */
  keywords: string[];
}

const STORAGE_KEY = "ajot_custom_tag_rules_v1";

/** 内部 id 在 selectedTags 数组中的前缀，用于与真实标签区分 */
export const CUSTOM_TAG_PREFIX = "__custom__:";

/** 由规则生成在 selectedTags 中使用的 sentinel id */
export function customTagSentinel(rule: CustomTagRule): string {
  return CUSTOM_TAG_PREFIX + rule.id;
}

/** 判断某 sentinel 是否为自定义标签 */
export function isCustomTagSentinel(tag: string): boolean {
  return typeof tag === "string" && tag.startsWith(CUSTOM_TAG_PREFIX);
}

/** 由 sentinel 反查 rule.id */
export function ruleIdOfSentinel(tag: string): string {
  return tag.slice(CUSTOM_TAG_PREFIX.length);
}

/** 关键字匹配：任一关键字命中 url 即返回 true */
export function matchCustomRule(rule: CustomTagRule, url: string | undefined): boolean {
  if (!rule || !rule.keywords || rule.keywords.length === 0) return false;
  const u = (url || "").trim().toLowerCase();
  if (!u) return false;
  for (const k of rule.keywords) {
    const kw = (k || "").trim().toLowerCase();
    if (kw && u.includes(kw)) return true;
  }
  return false;
}

function loadFromStorage(): CustomTagRule[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw);
    if (!Array.isArray(arr)) return [];
    return arr
      .filter(
        (r: any) =>
          r &&
          typeof r.id === "string" &&
          typeof r.name === "string" &&
          Array.isArray(r.keywords),
      )
      .map((r: any) => ({
        id: r.id,
        name: r.name,
        keywords: r.keywords.filter((x: any) => typeof x === "string"),
      }));
  } catch {
    return [];
  }
}

// 跨组件共享的响应式规则列表
const rules = ref<CustomTagRule[]>(loadFromStorage());

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(rules.value));
  } catch {
    /* ignore */
  }
}

function newId(): string {
  return `r${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

export function useCustomTagRules() {
  function addRule(name: string, keywords: string[]): CustomTagRule {
    const rule: CustomTagRule = {
      id: newId(),
      name: name.trim(),
      keywords: keywords.map((k) => k.trim()).filter(Boolean),
    };
    rules.value = [...rules.value, rule];
    persist();
    return rule;
  }

  function updateRule(id: string, patch: Partial<Omit<CustomTagRule, "id">>) {
    rules.value = rules.value.map((r) => {
      if (r.id !== id) return r;
      return {
        ...r,
        ...(patch.name !== undefined ? { name: patch.name.trim() } : {}),
        ...(patch.keywords !== undefined
          ? { keywords: patch.keywords.map((k) => k.trim()).filter(Boolean) }
          : {}),
      };
    });
    persist();
  }

  function removeRule(id: string) {
    rules.value = rules.value.filter((r) => r.id !== id);
    persist();
  }

  function findRule(id: string): CustomTagRule | undefined {
    return rules.value.find((r) => r.id === id);
  }

  return { rules, addRule, updateRule, removeRule, findRule };
}
