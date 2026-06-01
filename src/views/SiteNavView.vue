<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { SearchOutline } from "@vicons/ionicons5";
import { useMessage } from "naive-ui";
import { api, type PasswordSummary, type SiteSuggestion } from "../api";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const message = useMessage();

const loading = ref(false);
const allData = ref<PasswordSummary[]>([]);
const searchQuery = ref("");
const selectedRole = ref("");

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

/** 所有分类选项 */
const roleOptions = computed(() => {
  const roles = new Set<string>();
  for (const entry of entriesWithUrl.value) {
    if (entry.role) roles.add(entry.role);
  }
  return Array.from(roles).sort();
});

/** 过滤后的条目 */
const filteredEntries = computed(() => {
  let result = entriesWithUrl.value;
  // 分类筛选
  if (selectedRole.value) {
    result = result.filter((e) => e.role === selectedRole.value);
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

/** 按 role 分组 */
const groupedEntries = computed(() => {
  const groups: Record<string, PasswordSummary[]> = {};
  for (const entry of filteredEntries.value) {
    const role = entry.role || "未分类";
    if (!groups[role]) groups[role] = [];
    groups[role].push(entry);
  }
  // 按分类名排序
  const sorted = Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
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

/** 复制网址 */
async function handleCopy(entry: PasswordSummary, event: MouseEvent) {
  event.preventDefault();
  try {
    await writeText(entry.url);
    message.success("已复制网址");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
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
      <n-select
        v-model:value="selectedRole"
        :options="[
          { label: '全部分类', value: '' },
          ...roleOptions.map((r) => ({ label: r, value: r })),
        ]"
        style="width: 140px"
      />
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
      <div v-for="[role, entries] in groupedEntries" :key="role" class="site-nav-group">
        <h3 class="group-title">{{ role }}</h3>
        <div class="site-grid">
          <div
            v-for="entry in entries"
            :key="entry.id"
            class="site-card"
            @click="handleOpen(entry)"
            @contextmenu="handleCopy(entry, $event)"
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
            <span class="card-url" :title="entry.url">{{ extractDomain(entry.url) }}</span>
          </div>
        </div>
      </div>
    </div>
  </n-spin>
</template>

<style scoped>
.site-nav-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
}

.site-nav-groups {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.site-nav-group {
  /* 每个分类分组 */
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 4px;
  opacity: 0.7;
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

.card-url {
  font-size: 10px;
  color: var(--n-text-color-2, #666);
  opacity: 0.75;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
