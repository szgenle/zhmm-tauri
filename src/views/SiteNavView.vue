<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { SearchOutline } from "@vicons/ionicons5";
import { useMessage } from "naive-ui";
import { api, type PasswordEntry, type PasswordSummary, type SiteSuggestion } from "../api";
import { openUrl } from "@tauri-apps/plugin-opener";
import PasswordEditDialog from "../components/PasswordEditDialog.vue";
import TagSidebar, {
  UNCATEGORIZED_TAG,
  isOtherSubSentinel,
  primaryOfOtherSub,
  computeVisibleChildrenMap,
  entryMatchesOtherSub,
} from "../components/TagSidebar.vue";
import {
  useCustomTagRules,
  isCustomTagSentinel,
  ruleIdOfSentinel,
  matchCustomRule,
} from "../composables/useCustomTagRules";

const message = useMessage();

const loading = ref(false);
const allData = ref<PasswordSummary[]>([]);
const searchQuery = ref("");
/** 单选标签数组（最多一项），与 TagSidebar 统一接口 */
const selectedTags = ref<string[]>([]);
/** 用户在「标签词典」勾选的一级标签集合 */
const primaryTags = ref<string[]>([]);

const { findRule } = useCustomTagRules();

// 最近使用标签（与账号管理共享同一组件，独立 key）
const RECENT_TAGS_KEY = "ajot_role_mgmt_recent_tags";

/** 当前选中的单标签（"" 表示未选） */
const selectedTag = computed(() => selectedTags.value[0] || "");

/** TagSidebar 内部会更新 localStorage，本地缓存用于 groupedEntries 排序与 sidebar 顺序保持一致 */
function loadRecentTags(): string[] {
  try {
    const stored = localStorage.getItem(RECENT_TAGS_KEY);
    if (stored) {
      const arr = JSON.parse(stored);
      if (Array.isArray(arr)) return arr.filter((x) => typeof x === "string");
    }
  } catch { /* ignore */ }
  return [];
}
const recentTags = ref<string[]>(loadRecentTags());
watch(selectedTags, () => {
  // TagSidebar 已写入 localStorage，这里同步刷新本地缓存
  recentTags.value = loadRecentTags();
});

// 站点建议缓存: domain -> SiteSuggestion
const suggestCache = ref<Record<string, SiteSuggestion>>({});

// Favicon 缓存: domain -> base64 data-url
const faviconCache = ref<Record<string, string>>({});

// 防抖搜索
let searchTimer: ReturnType<typeof setTimeout> | null = null;
const debouncedQuery = ref("");
watch(searchQuery, (val) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    debouncedQuery.value = val.trim().toLowerCase();
  }, 200);
});

/** 从 URL 提取域名 */
function extractDomain(url: string): string {
  try {
    let u = url.trim();
    if (!/^https?:\/\//i.test(u)) u = "https://" + u;
    return new URL(u).hostname;
  } catch {
    return url.replace(/^https?:\/\//i, "").split("/")[0] || url;
  }
}

/** 获取显示名称：name > suggestCache > domain */
function getDisplayName(entry: PasswordSummary): string {
  if (entry.name) return entry.name;
  const domain = extractDomain(entry.url);
  const suggestion = suggestCache.value[domain];
  if (suggestion?.name) return suggestion.name;
  // 移除 www. 前缀并取域名作为 fallback
  return domain.replace(/^www\./, "");
}

/** 获取已缓存的 favicon data-url，未缓存时返回空串 */
function getCachedFavicon(url: string): string {
  const domain = extractDomain(url);
  return faviconCache.value[domain] || "";
}

/** 过滤出有 url 的条目 */
const entriesWithUrl = computed(() =>
  allData.value.filter((e) => e.url && e.url.trim().length > 0)
);

/** 过滤后的条目（标签筛选：tags 数组包含选中标签即匹配，无论位置） */
const filteredEntries = computed(() => {
  let result = entriesWithUrl.value;
  const primarySet = new Set(primaryTags.value);
  // 标签筛选
  if (selectedTag.value === UNCATEGORIZED_TAG) {
    // 「未分类」：用户已配置一级标签时，指 tags 不含任何一级标签的条目；
    // 未配置时，回退到「无任何 tag」语义。
    if (primarySet.size > 0) {
      result = result.filter((e) => !((e.tags || []).some((t) => primarySet.has(t))));
    } else {
      result = result.filter((e) => !((e.tags || []).find((t) => !!t)));
    }
  } else if (isOtherSubSentinel(selectedTag.value)) {
    const visibleMap = computeVisibleChildrenMap(entriesWithUrl.value, undefined, primarySet);
    result = result.filter((e) => entryMatchesOtherSub(e, selectedTag.value, visibleMap, primarySet));
  } else if (isCustomTagSentinel(selectedTag.value)) {
    const rule = findRule(ruleIdOfSentinel(selectedTag.value));
    if (rule) {
      result = result.filter((e) => matchCustomRule(rule, e.url));
    }
  } else if (selectedTag.value) {
    const want = selectedTag.value;
    result = result.filter((e) => (e.tags || []).includes(want));
  }
  // 搜索过滤
  const q = debouncedQuery.value;
  if (q) {
    result = result.filter((e) => {
      const name = getDisplayName(e).toLowerCase();
      const url = e.url.toLowerCase();
      const tags = e.tags.join(" ").toLowerCase();
      return name.includes(q) || url.includes(q) || tags.includes(q);
    });
  }
  return result;
});

/** 二级子组兜底标签：所属一级分组下没有 tags[1] 的条目归入此组 */
const OTHER_SUB_TAG = "其它";

interface SubGroup {
  tag: string;
  entries: PasswordSummary[];
}
interface PrimaryGroup {
  tag: string;
  count: number;
  subgroups: SubGroup[];
}

/** 在指定一级分组内，按二级标签拆分子组：
 * - visibleSubs 给出该一级分组下「频次 ≥ 阈值」的二级标签集合（与左侧 TagSidebar 对齐）
 * - 不在 visibleSubs 内的二级标签 / 无可用二级标签的条目 → 「其它」子组
 * - primary 给定时（用户已配置一级标签场景），子标签从 entry.tags 中除 primary 外挑选；
 *   未给定时（旧路径），保留 tags[1:] 行为。
 * - 子组排序：按条目数降序，「其它」固定排末尾
 */
function buildSubgroups(
  entries: PasswordSummary[],
  visibleSubs?: Set<string>,
  primary?: string,
): SubGroup[] {
  const map: Record<string, PasswordSummary[]> = {};
  for (const e of entries) {
    const ts = (e.tags || []).filter((t) => !!t);
    const candidates = primary ? ts.filter((t) => t !== primary) : ts.slice(1);
    let sub = OTHER_SUB_TAG;
    if (visibleSubs) {
      const hit = candidates.find((t) => visibleSubs.has(t));
      if (hit) sub = hit;
    } else if (candidates.length > 0 && candidates[0]) {
      sub = candidates[0];
    }
    if (!map[sub]) map[sub] = [];
    map[sub].push(e);
  }
  const subs = Object.entries(map).map(([tag, items]) => ({ tag, entries: items }));
  subs.sort((a, b) => {
    if (a.tag === OTHER_SUB_TAG) return 1;
    if (b.tag === OTHER_SUB_TAG) return -1;
    if (b.entries.length !== a.entries.length) return b.entries.length - a.entries.length;
    return a.tag.localeCompare(b.tag);
  });
  return subs;
}

/** 按标签分组（两级）：
 * - 一级：用户在「标签词典」勾选的 primary_tags。
 *   一个条目的 tags 命中多个一级标签时，会在多个分组中重复出现（交叉归属）。
 *   未配置时回退为 tags[0]，保持向后兼容。
 * - 二级：在所属一级分组内，从 entry.tags 中除一级外挑选第一个落入 visibleSubs 的标签；
 *   无可用二级标签则归入「其它」。
 * - 选中具体标签时：单一一级分组，内部仍按二级细分
 */
const groupedEntries = computed<PrimaryGroup[]>(() => {
  const primarySet = new Set(primaryTags.value);
  const useConfig = primarySet.size > 0;
  // 全局阈值可见子标签集（与左侧 TagSidebar 对齐：频次 ≥ CHILD_MIN_COUNT 的标签才独立显示）
  const visibleMap = computeVisibleChildrenMap(entriesWithUrl.value, undefined, primarySet);

  // 选中具体标签：单一一级分组，内部保留二级分组
  if (selectedTag.value && selectedTag.value !== UNCATEGORIZED_TAG) {
    let groupTitle = selectedTag.value;
    if (isCustomTagSentinel(selectedTag.value)) {
      const rule = findRule(ruleIdOfSentinel(selectedTag.value));
      groupTitle = rule ? rule.name : "自定义筛选";
    } else if (isOtherSubSentinel(selectedTag.value)) {
      // 「其它」虚拟标签：所有命中条目作为单一分组展示，不再按二级细分
      groupTitle = `${primaryOfOtherSub(selectedTag.value)} · 其它`;
      return [{
        tag: groupTitle,
        count: filteredEntries.value.length,
        subgroups: [{ tag: OTHER_SUB_TAG, entries: filteredEntries.value }],
      }];
    }
    // 选中真实标签时，该分组内沿用全局可见集做二级细分
    const visibleForThis = visibleMap.get(selectedTag.value);
    return [{
      tag: groupTitle,
      count: filteredEntries.value.length,
      subgroups: buildSubgroups(filteredEntries.value, visibleForThis, selectedTag.value),
    }];
  }
  if (selectedTag.value === UNCATEGORIZED_TAG) {
    return [{
      tag: "未分类",
      count: filteredEntries.value.length,
      subgroups: [{ tag: OTHER_SUB_TAG, entries: filteredEntries.value }],
    }];
  }
  // 未选标签：按一级标签集合聚合
  // - useConfig：一个条目命中多个一级标签时，在每个对应分组都重复出现
  // - 未配置：不分级 — 所有出现过的 tag 都是独立分组（条目按 tag 重复归入），分组内不再二级细分
  const groups: Record<string, PasswordSummary[]> = {};
  const uncategorized: PasswordSummary[] = [];
  for (const entry of filteredEntries.value) {
    const ts = (entry.tags || []).filter((t) => !!t);
    if (useConfig) {
      const myPrimaries = ts.filter((t) => primarySet.has(t));
      if (myPrimaries.length === 0) {
        uncategorized.push(entry);
        continue;
      }
      for (const p of myPrimaries) {
        if (!groups[p]) groups[p] = [];
        groups[p].push(entry);
      }
    } else {
      // 未配置一级标签：完全不分级，每个 tag 都是独立分组
      if (ts.length === 0) {
        uncategorized.push(entry);
        continue;
      }
      for (const t of ts) {
        if (!groups[t]) groups[t] = [];
        groups[t].push(entry);
      }
    }
  }
  if (uncategorized.length > 0) groups["未分类"] = uncategorized;
  // 与左侧 TagSidebar 排序一致：按最近使用，"未分类" 放最后
  const recentOrder = recentTags.value;
  const sorted = Object.entries(groups).sort(([a], [b]) => {
    if (a === "未分类") return 1;
    if (b === "未分类") return -1;
    const ai = recentOrder.indexOf(a);
    const bi = recentOrder.indexOf(b);
    if (ai >= 0 && bi >= 0) return ai - bi;
    if (ai >= 0) return -1;
    if (bi >= 0) return 1;
    return a.localeCompare(b);
  });
  return sorted.map(([tag, items]) => ({
    tag,
    count: items.length,
    // useConfig：分组内按二级细分；未配置：不细分（无"一级"概念，避免再分层）
    subgroups: tag === "未分类" || !useConfig
      ? [{ tag: OTHER_SUB_TAG, entries: items }]
      : buildSubgroups(items, visibleMap.get(tag), tag),
  }));
});

/** 加载数据 */
async function loadData() {
  loading.value = true;
  try {
    const [list, primary] = await Promise.all([
      api.listPasswords(),
      api.getUserPrimaryTags().catch(() => [] as string[]),
    ]);
    allData.value = list;
    primaryTags.value = primary;
  } catch (e: any) {
    message.error(`加载数据失败: ${e}`);
  } finally {
    loading.value = false;
  }

  // 以下在后台加载，不阻塞 UI 渲染
  const withUrl = allData.value.filter((e) => e.url && e.url.trim().length > 0);

  // 异步补全站点名称（限流）
  const needSuggest = withUrl.filter((e) => !e.name);
  const suggestDomains = Array.from(new Set(needSuggest.map((e) => extractDomain(e.url))));
  loadSuggestions(suggestDomains);

  // 异步缓存 favicon（限流）
  const allDomains = Array.from(new Set(withUrl.map((e) => extractDomain(e.url))));
  loadFavicons(allDomains);
}

/** 限流加载站点建议，每次最多 3 个并发 */
async function loadSuggestions(domains: string[]) {
  const concurrency = 3;
  let i = 0;
  async function next() {
    while (i < domains.length) {
      const domain = domains[i++];
      if (suggestCache.value[domain]) continue;
      try {
        const suggestion = await api.suggestSite(domain);
        if (suggestion.name || suggestion.tags.length > 0) {
          suggestCache.value[domain] = suggestion;
        }
      } catch { /* 静默忽略 */ }
    }
  }
  const workers = Array.from({ length: Math.min(concurrency, domains.length) }, () => next());
  await Promise.all(workers);
}

/** 限流加载 favicons，批量更新减少 re-render */
async function loadFavicons(domains: string[]) {
  const concurrency = 3;
  let i = 0;
  const pending: Record<string, string> = {};
  let timer: ReturnType<typeof setTimeout> | null = null;
  function flush() {
    const keys = Object.keys(pending);
    if (keys.length > 0) {
      faviconCache.value = { ...faviconCache.value, ...pending };
      keys.forEach((k) => delete pending[k]);
    }
    timer = null;
  }
  async function next() {
    while (i < domains.length) {
      const domain = domains[i++];
      if (faviconCache.value[domain]) continue;
      try {
        const dataUrl = await api.cacheFavicon(domain);
        pending[domain] = dataUrl;
        if (Object.keys(pending).length >= 5) flush();
        else if (!timer) timer = setTimeout(flush, 200);
      } catch { /* skip */ }
    }
  }
  const workers = Array.from({ length: Math.min(concurrency, domains.length) }, () => next());
  await Promise.all(workers);
  flush();
}

/** 打开网址 */
async function handleOpen(entry: PasswordSummary) {
  try {
    let url = entry.url.trim();
    if (!/^https?:\/\//i.test(url)) url = "https://" + url;
    await openUrl(url);
  } catch (e: any) {
    message.error(`打开失败: ${e}`);
  }
}

/** 右键编辑记录 */
const showEditDialog = ref(false);
const editEntry = ref<PasswordEntry | null>(null);
async function handleEdit(entry: PasswordSummary, event: MouseEvent) {
  event.preventDefault();
  try {
    editEntry.value = await api.getPassword(entry.id);
    showEditDialog.value = true;
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  }
}



onMounted(loadData);
</script>

<template>
  <div class="site-nav-page">
    <TagSidebar
      :entries="entriesWithUrl"
      :selected-tags="selectedTags"
      :primary-tags="primaryTags"
      sort-mode="recent"
      :recent-tags-key="RECENT_TAGS_KEY"
      @update:selected-tags="v => selectedTags = v"
    />
    <div class="site-nav-main">
      <!-- 加载状态 -->
      <div v-if="loading" class="site-nav-loading">
        <n-spin size="medium" />
      </div>

      <!-- 内容区 -->
      <div v-else class="site-nav-body">
        <div class="site-nav-toolbar">
          <n-input
            v-model:value="searchQuery"
            placeholder="搜索网站名称、网址、标签..."
            clearable
            style="width: 320px"
          >
            <template #prefix>
              <n-icon :component="SearchOutline" />
            </template>
          </n-input>
        </div>

        <!-- 空状态 -->
        <n-empty
          v-if="entriesWithUrl.length === 0"
          description="暂无包含网址的账号记录"
          style="margin-top: 80px"
        />
        <n-empty
          v-else-if="filteredEntries.length === 0"
          description="无匹配结果"
          style="margin-top: 80px"
        />

        <!-- 分组卡片宫格（一级 → 二级） -->
        <div v-else class="site-nav-groups">
          <div v-for="group in groupedEntries" :key="group.tag" class="site-nav-group">
            <h3 class="group-title">
              {{ group.tag }}
              <span class="group-count">{{ group.count }}</span>
            </h3>
            <template v-for="sub in group.subgroups" :key="sub.tag">
              <h4
                v-if="!(group.subgroups.length === 1 && sub.tag === '其它')"
                class="subgroup-title"
              >
                {{ sub.tag }}
                <span class="group-count">{{ sub.entries.length }}</span>
              </h4>
              <div class="site-grid">
                <div
                  v-for="entry in sub.entries"
                  :key="entry.id"
                  class="site-card"
                  @click="handleOpen(entry)"
                  @contextmenu="handleEdit(entry, $event)"
                >
                  <div class="card-icon">
                    <img
                      v-if="getCachedFavicon(entry.url)"
                      :src="getCachedFavicon(entry.url)"
                      :alt="getDisplayName(entry)"
                      class="favicon"
                    />
                    <span v-else class="favicon-fallback">
                      {{ getDisplayName(entry).charAt(0) }}
                    </span>
                  </div>
                  <span class="card-name">{{ getDisplayName(entry) }}</span>
                  <span class="card-user" :title="entry.userID || '未设置账号'">{{ entry.userID || '—' }}</span>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- 编辑对话框（右键卡片触发） -->
    <PasswordEditDialog
      :show="showEditDialog"
      :edit-entry="editEntry"
      @update:show="showEditDialog = $event"
      @saved="loadData"
    />
  </div>
</template>

<style scoped>
.site-nav-page {
  display: flex;
  height: 100%;
  overflow: hidden;
  gap: 0;
}

.site-nav-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: none;
  padding: 0;
}

.site-nav-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
}

.site-nav-body {
  flex: 1;
  min-height: 0;
}

.site-nav-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}

.site-nav-groups {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 4px;
  opacity: 0.7;
}

.subgroup-title {
  font-size: 12px;
  font-weight: 500;
  margin: 14px 0 8px 12px;
  opacity: 0.55;
}

.group-count {
  font-size: 12px;
  font-weight: 400;
  opacity: 0.6;
  margin-left: 6px;
}

.site-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 12px;
}

.site-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 14px 8px;
  border-radius: 10px;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease, background 0.2s ease;
  background: var(--app-card-bg, #ffffff);
  border: 1px solid var(--app-border-color, rgba(0, 0, 0, 0.06));
  box-shadow: var(--app-shadow-sm, 0 2px 8px rgba(0, 0, 0, 0.04));
  backdrop-filter: blur(8px);
  contain: layout style;
}

.site-card:hover {
  transform: translateY(var(--app-hover-lift, -2px));
  box-shadow: var(--app-shadow-md, 0 4px 16px rgba(0, 0, 0, 0.06));
}

.site-card:active {
  transform: translateY(0);
}

.card-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.favicon {
  width: 32px;
  height: 32px;
  border-radius: 4px;
  object-fit: contain;
}

.favicon-fallback {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: var(--app-border-color, rgba(0, 0, 0, 0.06));
  color: var(--n-text-color, #333);
  font-size: 16px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-name {
  font-size: 12px;
  font-weight: 500;
  text-align: center;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-user {
  font-size: 10px;
  color: var(--n-text-color-2, #666);
  opacity: 0.75;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ========== 深色主题：降低卡片视觉密度 ==========
   卡片数量大时，深色背景下的边框/阴影会显得拥挤；
   故采用「幽灵卡」策略：默认透明，仅 hover 时浮现轻量容器。 */
:global(html[data-theme="dark"]) .site-card {
  background: transparent;
  border-color: transparent;
  box-shadow: none;
  backdrop-filter: none;
}

:global(html[data-theme="dark"]) .site-card:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.06);
  box-shadow: none;
}

/* favicon 兜底字符在深色下保留弱底，避免「裸字」 */
:global(html[data-theme="dark"]) .favicon-fallback {
  background: rgba(255, 255, 255, 0.06);
  color: var(--n-text-color, rgba(255, 255, 255, 0.85));
}
</style>
