<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { SearchOutline } from "@vicons/ionicons5";
import { useMessage } from "naive-ui";
import { api, type PasswordEntry, type PasswordSummary, type SiteSuggestion } from "../api";
import { openUrl } from "@tauri-apps/plugin-opener";
import PasswordEditDialog from "../components/PasswordEditDialog.vue";

const message = useMessage();

const loading = ref(false);
const allData = ref<PasswordSummary[]>([]);
const searchQuery = ref("");
const selectedTag = ref("");

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

/** 所有标签选项 */
const tagOptions = computed(() => {
  const tags = new Set<string>();
  for (const entry of entriesWithUrl.value) {
    for (const t of entry.tags) {
      if (t) tags.add(t);
    }
  }
  return Array.from(tags).sort();
});

/** 标签芯片栏：包含名称、计数，未分类放最后 */
const UNTAGGED_KEY = "__untagged__";
const tagChips = computed(() => {
  const counts = new Map<string, number>();
  let untagged = 0;
  for (const entry of entriesWithUrl.value) {
    if (!entry.tags || entry.tags.length === 0) {
      untagged++;
      continue;
    }
    for (const t of entry.tags) {
      if (!t) continue;
      counts.set(t, (counts.get(t) || 0) + 1);
    }
  }
  const arr: { key: string; label: string; count: number }[] = [];
  for (const t of tagOptions.value) {
    arr.push({ key: t, label: t, count: counts.get(t) || 0 });
  }
  if (untagged > 0) {
    arr.push({ key: UNTAGGED_KEY, label: "未分类", count: untagged });
  }
  return arr;
});

function selectTag(key: string) {
  // 再次点击选中的 chip 取消选中
  selectedTag.value = selectedTag.value === key ? "" : key;
}

/** 过滤后的条目 */
const filteredEntries = computed(() => {
  let result = entriesWithUrl.value;
  // 标签筛选
  if (selectedTag.value === UNTAGGED_KEY) {
    result = result.filter((e) => !e.tags || e.tags.length === 0);
  } else if (selectedTag.value) {
    result = result.filter((e) => e.tags.includes(selectedTag.value));
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

/** 按标签分组：每个条目按其首个标签归入对应分组，无标签归入"未分类" */
const groupedEntries = computed(() => {
  const groups: Record<string, PasswordSummary[]> = {};
  for (const entry of filteredEntries.value) {
    const tag = (entry.tags && entry.tags.length > 0) ? entry.tags[0] : "未分类";
    if (!groups[tag]) groups[tag] = [];
    groups[tag].push(entry);
  }
  // 按标签名排序，"未分类" 放最后
  const sorted = Object.entries(groups).sort(([a], [b]) => {
    if (a === "未分类") return 1;
    if (b === "未分类") return -1;
    return a.localeCompare(b);
  });
  return sorted;
});

/** 加载数据 */
async function loadData() {
  loading.value = true;
  try {
    allData.value = await api.listPasswords();
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
  <n-spin :show="loading">
    <!-- 顶部工具栏 -->
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

    <!-- 标签芯片栏 -->
    <div v-if="tagChips.length" class="tag-chips-bar">
      <div class="tag-chips-scroll">
        <span
          class="tag-chip"
          :class="{ active: selectedTag === '' }"
          @click="selectedTag = ''"
        >
          全部
          <span class="chip-count">{{ entriesWithUrl.length }}</span>
        </span>
        <span
          v-for="chip in tagChips"
          :key="chip.key"
          class="tag-chip"
          :class="{ active: selectedTag === chip.key }"
          @click="selectTag(chip.key)"
        >
          {{ chip.label }}
          <span class="chip-count">{{ chip.count }}</span>
        </span>
      </div>
    </div>

    <!-- 空状态 -->
    <n-empty
      v-if="!loading && entriesWithUrl.length === 0"
      description="暂无包含网址的账号记录"
      style="margin-top: 80px"
    />
    <n-empty
      v-else-if="!loading && filteredEntries.length === 0"
      description="无匹配结果"
      style="margin-top: 80px"
    />

    <!-- 分组卡片宫格 -->
    <div v-else class="site-nav-groups">
      <div v-for="[tag, entries] in groupedEntries" :key="tag" class="site-nav-group">
        <h3 class="group-title">{{ tag }} <span class="group-count">{{ entries.length }}</span></h3>
        <div class="site-grid">
          <div
            v-for="entry in entries"
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
      </div>
    </div>

    <!-- 编辑对话框（右键卡片触发） -->
    <PasswordEditDialog
      :show="showEditDialog"
      :edit-entry="editEntry"
      @update:show="showEditDialog = $event"
      @saved="loadData"
    />
  </n-spin>
</template>

<style scoped>
.site-nav-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}

.tag-chips-bar {
  margin-bottom: 18px;
  padding: 10px 14px;
  background: var(--app-card-bg);
  border: 1px solid var(--app-border-color, rgba(0, 0, 0, 0.06));
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm, 0 2px 8px rgba(0, 0, 0, 0.04));
  backdrop-filter: blur(8px);
}

.tag-chips-scroll {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  border-radius: 16px;
  font-size: 13px;
  cursor: pointer;
  user-select: none;
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
  border: 1px solid transparent;
  transition: all 0.2s ease;
}

.tag-chip:hover {
  background: var(--n-color-pressed, rgba(0, 0, 0, 0.08));
  transform: translateY(-1px);
}

.tag-chip.active {
  background: var(--n-color-primary, #18a058);
  color: #fff;
  border-color: var(--n-color-primary, #18a058);
  box-shadow: 0 2px 8px rgba(24, 160, 88, 0.25);
}

.chip-count {
  font-size: 11px;
  opacity: 0.7;
  margin-left: 2px;
}

.tag-chip.active .chip-count {
  opacity: 0.85;
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

/* 标签芯片栏在深色下同样降级，仅作为内容容器存在 */
:global(html[data-theme="dark"]) .tag-chips-bar {
  background: transparent;
  border-color: transparent;
  box-shadow: none;
  backdrop-filter: none;
  padding: 6px 0;
}

:global(html[data-theme="dark"]) .tag-chip {
  background: rgba(255, 255, 255, 0.05);
}

:global(html[data-theme="dark"]) .tag-chip:hover {
  background: rgba(255, 255, 255, 0.10);
}

/* favicon 兜底字符在深色下保留弱底，避免「裸字」 */
:global(html[data-theme="dark"]) .favicon-fallback {
  background: rgba(255, 255, 255, 0.06);
  color: var(--n-text-color, rgba(255, 255, 255, 0.85));
}
</style>
