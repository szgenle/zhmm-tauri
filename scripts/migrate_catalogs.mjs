#!/usr/bin/env node
// [HISTORICAL] 一次性迁移脚本（已完成使命，留作参考）：
// 当年把 resources/catalogs/{base,creator,...}.json（每身份带 sites 子集）合并去重为
//   - resources/catalogs/sites.json           （站点全集，唯一事实源）
//   - resources/catalogs/personas/<id>.json   （每个身份只保留 primary_tags + 元信息）
//
// 当前数据布局已是合并后的结构，脚本不再适用，重跑前请先备份。
//
// 用法：
//   node scripts/migrate_catalogs.mjs            # dry-run，仅打印统计与冲突预览
//   node scripts/migrate_catalogs.mjs --apply    # 实际写入 OUT_DIR
//   node scripts/migrate_catalogs.mjs --apply --out resources/catalogs.merged
//
// 设计：
// - 每个 site 不保留 personas 字段（站点-标签是客观事实，与身份解耦）
// - tags 跨 persona 取并集（保持首次出现顺序）
// - name 冲突优先取 NAME_OVERRIDES 指定值；否则按 PERSONA_ORDER 取靠前者，冲突会列出
// - crypto persona 整体排除（合规考虑）

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const SRC_DIR = path.join(ROOT, "resources/catalogs");

const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const outIdx = args.indexOf("--out");
const OUT_DIR = outIdx >= 0 ? path.resolve(ROOT, args[outIdx + 1]) : path.join(ROOT, "resources/catalogs.merged");

// 优先级：name 冲突时取靠前者；同时决定 personas 数组的顺序
// 注意：crypto persona 已整体排除（合规考虑），不再读取 crypto.json
const PERSONA_ORDER = [
  "base",
  "developer",
  "game-dev",
  "creator",
  "small-biz",
  "cross-border",
];

// 身份 id -> 中文展示名（前端身份选择 UI 用）
const PERSONA_DISPLAY_NAMES = {
  base: "通用底座",
  developer: "软件开发",
  "game-dev": "游戏开发",
  creator: "内容创作者",
  "small-biz": "小微主理人",
  "cross-border": "跨境/外贸",
};

// host -> 强制 name（解决多 persona 同 host 不同 name 的冲突）
// 优先级高于 PERSONA_ORDER；命中后该 host 的 name 一律取此值。
const NAME_OVERRIDES = {
  "mp.weixin.qq.com": "微信公众平台",
  "ad.oceanengine.com": "巨量引擎",
  "beian.miit.gov.cn": "工信部 ICP 备案",
  "fxg.jinritemai.com": "抖店后台",
  "channels.weixin.qq.com": "视频号助手",
};

// persona id -> 来源标签：合并到 sites.json 时，为该 persona 贡献的 site
// 在 tags 末尾注入此标签（去重），同时该标签会前置写入对应 persona 的 primary_tags。
// base 不注入：通用底座覆盖所有大站，注入会污染语义。
const SOURCE_TAG_INJECT = {
  developer: "开发",
  "game-dev": "游戏开发",
  creator: "内容创作",
  "small-biz": "小微主理",
  "cross-border": "跨境外贸",
};

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf-8"));
}

function ensureDir(p) {
  fs.mkdirSync(p, { recursive: true });
}

/** 数组去重并保持首次出现顺序 */
function uniqOrdered(arr) {
  const seen = new Set();
  const out = [];
  for (const x of arr) {
    if (!seen.has(x)) {
      seen.add(x);
      out.push(x);
    }
  }
  return out;
}

// ---------- 加载所有 catalog ----------
const catalogs = []; // {id, meta, sites, raw}
for (const id of PERSONA_ORDER) {
  const file = path.join(SRC_DIR, `${id}.json`);
  if (!fs.existsSync(file)) {
    console.warn(`⚠ 跳过：${file}（不存在）`);
    continue;
  }
  const raw = readJson(file);
  catalogs.push({ id, meta: raw._meta || {}, sites: raw.sites || {}, raw });
}

// ---------- 合并 sites ----------
/** host -> { name, tagsOrder: string[] (用于保持顺序去重), tagsSet: Set, personas: Set, firstFrom: string } */
const merged = new Map();
const nameConflicts = []; // {host, kept, ignored: [{persona, name}]}
const stats = {
  perPersonaIn: {},
  totalIn: 0,
};

for (const cat of catalogs) {
  const ids = Object.keys(cat.sites);
  stats.perPersonaIn[cat.id] = ids.length;
  stats.totalIn += ids.length;

  for (const host of ids) {
    const s = cat.sites[host];
    const overrideName = NAME_OVERRIDES[host];
    const incomingName = overrideName || s.name || host;
    const rawTags = Array.isArray(s.tags) ? s.tags.filter((t) => !!t) : [];
    const sourceTag = SOURCE_TAG_INJECT[cat.id];
    // 来源标签注入：追加到末尾后整体去重（处理「原始 tags 已含注入标签」的情况）
    const incomingTags = uniqOrdered(sourceTag ? [...rawTags, sourceTag] : rawTags);

    let cur = merged.get(host);
    if (!cur) {
      cur = {
        name: incomingName,
        tagsOrder: [...incomingTags],
        tagsSet: new Set(incomingTags),
        personas: new Set([cat.id]),
        firstFrom: cat.id,
        forced: !!overrideName,
      };
      merged.set(host, cur);
    } else {
      // 仅在该 host 未被 override 强制时记录 name 冲突
      if (!cur.forced && cur.name !== incomingName) {
        const conflict = nameConflicts.find((c) => c.host === host);
        if (conflict) {
          conflict.ignored.push({ persona: cat.id, name: incomingName });
        } else {
          nameConflicts.push({
            host,
            kept: { persona: cur.firstFrom, name: cur.name },
            ignored: [{ persona: cat.id, name: incomingName }],
          });
        }
      }
      for (const t of incomingTags) {
        if (!cur.tagsSet.has(t)) {
          cur.tagsSet.add(t);
          cur.tagsOrder.push(t);
        }
      }
      cur.personas.add(cat.id);
    }
  }
}

// ---------- 构造 sites.json ----------
const sitesOut = {};
const sortedHosts = [...merged.keys()].sort((a, b) => a.localeCompare(b));
for (const host of sortedHosts) {
  const m = merged.get(host);
  // 不保留 personas 字段：站点-标签是客观事实，与身份解耦
  sitesOut[host] = {
    name: m.name,
    tags: m.tagsOrder,
  };
}

const sitesFile = {
  _meta: {
    version: 3,
    description:
      "站点全集（唯一事实源）。tags 数组顺序无关；身份过滤由 personas/<id>.json 的 primary_tags 在前端运行时进行。",
    schema: "sites-v3",
    updated: new Date().toISOString().slice(0, 10),
    persona_order: PERSONA_ORDER,
  },
  sites: sitesOut,
};

// ---------- 构造 personas/*.json ----------
const personaOut = {}; // id -> object
for (const cat of catalogs) {
  const m = cat.meta;
  const sourceTag = SOURCE_TAG_INJECT[cat.id];
  const rawPrimary = Array.isArray(m.primary_tags) ? m.primary_tags : [];
  // 来源标签前置作为身份核心一级（base 不注入）
  const primaryTags = uniqOrdered(sourceTag ? [sourceTag, ...rawPrimary] : rawPrimary);
  personaOut[cat.id] = {
    _meta: {
      version: 3,
      id: cat.id,
      name: PERSONA_DISPLAY_NAMES[cat.id] || cat.id,
      description: m.description || "",
      primary_tags: primaryTags,
      ...(m.community ? { community: true } : {}),
      ...(m.default_enabled === false ? { default_enabled: false } : {}),
      updated: m.updated || new Date().toISOString().slice(0, 10),
    },
  };
}

// ---------- 统计与冲突预览 ----------
function fmtPct(n, total) {
  return total === 0 ? "0%" : `${((n / total) * 100).toFixed(1)}%`;
}

console.log("===== 迁移预览 =====");
console.log(`源目录：${path.relative(ROOT, SRC_DIR)}`);
console.log(`输出目录：${path.relative(ROOT, OUT_DIR)}${APPLY ? "" : "（dry-run，未写入）"}`);

console.log("\n[1] 各 persona 站点数：");
for (const id of PERSONA_ORDER) {
  if (stats.perPersonaIn[id] != null) {
    console.log(`  ${id.padEnd(13)} ${stats.perPersonaIn[id]}`);
  }
}
console.log(`  ${"合计".padEnd(13)} ${stats.totalIn}`);

console.log("\n[2] 合并后：");
console.log(`  唯一站点 host: ${merged.size}`);
console.log(`  去重消除: ${stats.totalIn - merged.size} 条 (${fmtPct(stats.totalIn - merged.size, stats.totalIn)})`);

// 多 persona 命中的 host
const multiPersonaHosts = [...merged.entries()].filter(([, m]) => m.personas.size > 1);
console.log(`\n[3] 跨 persona 重复出现的 host：${multiPersonaHosts.length} 个`);
for (const [host, m] of multiPersonaHosts.slice(0, 30)) {
  console.log(`  ${host.padEnd(36)} <- ${[...m.personas].join(", ")}`);
}
if (multiPersonaHosts.length > 30) {
  console.log(`  ... 其余 ${multiPersonaHosts.length - 30} 个省略`);
}

// name 冲突
console.log(`\n[4] name 冲突（取靠前 persona 的版本）：${nameConflicts.length} 个`);
for (const c of nameConflicts) {
  const ignoredStr = c.ignored.map((x) => `${x.persona}="${x.name}"`).join(", ");
  console.log(`  ${c.host} 保留 ${c.kept.persona}="${c.kept.name}" | 忽略 ${ignoredStr}`);
}

// primary_tags 跨 persona 重叠
console.log("\n[5] 各 persona 一级标签：");
const primaryByPersona = {};
for (const id of PERSONA_ORDER) {
  if (personaOut[id]) {
    primaryByPersona[id] = personaOut[id]._meta.primary_tags;
    console.log(`  ${id.padEnd(13)} (${primaryByPersona[id].length}) ${primaryByPersona[id].join(" / ")}`);
  }
}

// 同名 primary_tag 出现在多个 persona 中（如「海外」）
const tagToPersonas = new Map(); // tag -> Set<persona>
for (const [pid, tags] of Object.entries(primaryByPersona)) {
  for (const t of tags) {
    if (!tagToPersonas.has(t)) tagToPersonas.set(t, new Set());
    tagToPersonas.get(t).add(pid);
  }
}
const sharedPrimary = [...tagToPersonas.entries()].filter(([, s]) => s.size > 1);
console.log(`\n[6] 出现在多个 persona 的一级标签：${sharedPrimary.length} 个`);
for (const [t, s] of sharedPrimary) {
  console.log(`  「${t}」 -> ${[...s].join(", ")}`);
}

// 抽样预览 5 条 sites
console.log("\n[7] sites.json 抽样（前 5 条按 host 字典序）：");
for (const h of sortedHosts.slice(0, 5)) {
  console.log(`  ${h} -> ${JSON.stringify(sitesOut[h])}`);
}

// 抽样 personas
console.log("\n[8] personas/*.json 抽样（developer）：");
console.log(JSON.stringify(personaOut.developer || {}, null, 2));

// ---------- 写入 ----------
if (APPLY) {
  ensureDir(OUT_DIR);
  ensureDir(path.join(OUT_DIR, "personas"));
  fs.writeFileSync(path.join(OUT_DIR, "sites.json"), JSON.stringify(sitesFile, null, 2) + "\n", "utf-8");
  for (const [id, obj] of Object.entries(personaOut)) {
    fs.writeFileSync(
      path.join(OUT_DIR, "personas", `${id}.json`),
      JSON.stringify(obj, null, 2) + "\n",
      "utf-8",
    );
  }
  console.log("\n✓ 已写入：");
  console.log(`  ${path.relative(ROOT, path.join(OUT_DIR, "sites.json"))}`);
  for (const id of Object.keys(personaOut)) {
    console.log(`  ${path.relative(ROOT, path.join(OUT_DIR, "personas", `${id}.json`))}`);
  }
} else {
  console.log("\n（dry-run）确认无误后追加 --apply 写入。");
}
