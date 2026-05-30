/**
 * 模板自动推荐匹配器
 *
 * 根据用户在编辑账号时填写的 url / 名称 / role / 描述，对当前 vault 内
 * 所有 AccountTemplate 的 match_rules 评分，选出最佳推荐模板。
 *
 * 与后端 [src-tauri/src/models.rs] 中的 TemplateMatchRule 数据结构对齐：
 *   - url_contains: 在 url 中做不区分大小写的子串匹配
 *   - keyword:      在 name / desc / userID 中做不区分大小写的子串匹配
 *   - role:         与当前 role 严格相等
 *
 * 评分规则（数值仅用于排序，不暴露给用户）：
 *   - url_contains 命中：+10（精度最高，域名匹配几乎不会误伤）
 *   - role         命中：+6  （role 由用户主动选，意图明确）
 *   - keyword      命中：+3  （子串匹配，存在误伤风险，权重低）
 *
 * 设计取舍：
 *   - 命中即累加，不限定每类一次（多个银行域名同时命中会累计，强化置信度）
 *   - 阈值 >= 6（一次 url_contains 或 role 即可触发，单 keyword 命中不触发，
 *     防止"个人"两个字撞到任何含「个人」字样的模板规则导致频繁打扰）
 */
import type { AccountTemplate, TemplateMatchRule } from "../api";

/** 推荐触发的最低分阈值 */
export const RECOMMEND_THRESHOLD = 6;

export interface MatchInput {
  url?: string;
  name?: string;
  role?: string;
  desc?: string;
  userID?: string;
}

export interface MatchResult {
  template: AccountTemplate;
  score: number;
  /** 命中规则的简短说明，用于 UI 提示（如"网址包含 icbc.com.cn"） */
  reasons: string[];
}

function lower(v: string | undefined): string {
  return (v || "").trim().toLowerCase();
}

/** 单个模板对当前输入的得分；返回 0 表示未命中 */
export function scoreTemplate(
  tpl: AccountTemplate,
  input: MatchInput,
): MatchResult {
  const rules = tpl.match_rules || [];
  if (rules.length === 0) {
    return { template: tpl, score: 0, reasons: [] };
  }
  const url = lower(input.url);
  const text = `${lower(input.name)} ${lower(input.desc)} ${lower(input.userID)}`;
  const role = (input.role || "").trim(); // role 严格相等，保留大小写

  let score = 0;
  const reasons: string[] = [];

  for (const r of rules as TemplateMatchRule[]) {
    if (r.kind === "url_contains") {
      if (!r.value || !url) continue;
      if (url.includes(r.value.toLowerCase())) {
        score += 10;
        reasons.push(`网址含「${r.value}」`);
      }
    } else if (r.kind === "keyword") {
      if (!r.value || !text.trim()) continue;
      if (text.includes(r.value.toLowerCase())) {
        score += 3;
        reasons.push(`关键词「${r.value}」`);
      }
    } else if (r.kind === "role") {
      if (!r.value || !role) continue;
      if (role === r.value) {
        score += 6;
        reasons.push(`分类=「${r.value}」`);
      }
    }
  }

  return { template: tpl, score, reasons };
}

/**
 * 在所有模板中挑出最佳推荐；未达阈值返回 null。
 * 多个模板得分相同时取首个（保留 vault 中模板的原顺序）。
 */
export function pickRecommendation(
  templates: AccountTemplate[],
  input: MatchInput,
): MatchResult | null {
  let best: MatchResult | null = null;
  for (const t of templates) {
    const r = scoreTemplate(t, input);
    if (r.score < RECOMMEND_THRESHOLD) continue;
    if (!best || r.score > best.score) best = r;
  }
  return best;
}
