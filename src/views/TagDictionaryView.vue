<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { NCheckbox, useMessage, useDialog } from "naive-ui";
import {
  DownloadOutline,
  SearchOutline,
  CloudUploadOutline,
  RefreshOutline,
  SaveOutline,
} from "@vicons/ionicons5";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type CatalogTagStat, type PresetPersonaInfo, type SiteCatalogEntry } from "../api";

const message = useMessage();
const dialog = useDialog();

// ========== 数据 ==========
const tagStats = ref<CatalogTagStat[]>([]);
/** 一级标签的工作副本：勾选状态实时反映在这个 Set 上，保存后落库 */
const primarySet = ref<Set<string>>(new Set());
/** 上一次落库的快照，用于判断是否有未保存改动 */
const savedSnapshot = ref<Set<string>>(new Set());
/** 站点条目（合并视图：内置 + 用户层） */
const siteEntries = ref<SiteCatalogEntry[]>([]);
const loading = ref(false);
const catalogBusy = ref(false);
const hasUserCatalog = ref(false);
const keyword = ref("");
const siteKeyword = ref("");

async function loadAll() {
  loading.value = true;
  try {
    const [stats, primary, sites, hasUser] = await Promise.all([
      api.listCatalogTagStats(),
      api.getUserPrimaryTags(),
      api.listSiteCatalog(),
      api.hasUserCatalog(),
    ]);
    tagStats.value = stats;
    primarySet.value = new Set(primary);
    savedSnapshot.value = new Set(primary);
    siteEntries.value = sites;
    hasUserCatalog.value = hasUser;
  } catch (e: any) {
    message.error(`加载标签词典失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

onMounted(loadAll);

// ========== 标签筛选 / 统计 ==========
const filteredStats = computed(() => {
  const kw = keyword.value.trim().toLowerCase();
  if (!kw) return tagStats.value;
  return tagStats.value.filter((s) => s.tag.toLowerCase().includes(kw));
});

const totalTags = computed(() => tagStats.value.length);
const primaryCount = computed(() => primarySet.value.size);
const isDirty = computed(() => {
  if (primarySet.value.size !== savedSnapshot.value.size) return true;
  for (const t of primarySet.value) if (!savedSnapshot.value.has(t)) return true;
  return false;
});

function isPrimary(tag: string): boolean {
  return primarySet.value.has(tag);
}

function togglePrimary(tag: string) {
  const next = new Set(primarySet.value);
  if (next.has(tag)) next.delete(tag);
  else next.add(tag);
  primarySet.value = next;
}

const tagColumns = computed(() => {
  // 显式依赖 primarySet，使 primarySet 变化时 columns 引用更新，
  // 进而触发 n-data-table 重新调用 NCheckbox render（否则 checked 不会响应式更新）
  const pset = primarySet.value;
  return [
    {
      title: "一级",
      key: "is_primary",
      width: 70,
      align: "center" as const,
      render: (row: CatalogTagStat) =>
        h(NCheckbox, {
          checked: pset.has(row.tag),
          "onUpdate:checked": () => togglePrimary(row.tag),
          onClick: (e: MouseEvent) => e.stopPropagation(),
        }),
    },
    {
      title: "标签",
      key: "tag",
      ellipsis: { tooltip: true },
      sorter: (a: CatalogTagStat, b: CatalogTagStat) =>
        a.tag.localeCompare(b.tag, "zh-Hans-CN"),
    },
    {
      title: "出现频次",
      key: "count",
      width: 110,
      align: "center" as const,
      sorter: (a: CatalogTagStat, b: CatalogTagStat) => a.count - b.count,
      defaultSortOrder: "descend" as const,
    },
  ];
});

/** 整行点击切换一级标签：避免只有勾选框小热区可点 */
function tagRowProps(row: CatalogTagStat) {
  return {
    style: "cursor: pointer;",
    onClick: () => togglePrimary(row.tag),
  };
}

// ========== 站点表 ==========
const filteredSites = computed(() => {
  const kw = siteKeyword.value.trim().toLowerCase();
  if (!kw) return siteEntries.value;
  return siteEntries.value.filter(
    (s) =>
      s.host.toLowerCase().includes(kw) ||
      s.name.toLowerCase().includes(kw) ||
      s.tags.some((t) => t.toLowerCase().includes(kw)),
  );
});

const siteColumns = computed(() => [
  {
    title: "域名",
    key: "host",
    width: 200,
    ellipsis: { tooltip: true },
    sorter: (a: SiteCatalogEntry, b: SiteCatalogEntry) => a.host.localeCompare(b.host),
  },
  {
    title: "名称",
    key: "name",
    width: 160,
    ellipsis: { tooltip: true },
    sorter: (a: SiteCatalogEntry, b: SiteCatalogEntry) =>
      a.name.localeCompare(b.name, "zh-Hans-CN"),
  },
  {
    title: "标签",
    key: "tags",
    ellipsis: { tooltip: true },
    render: (row: SiteCatalogEntry) => row.tags.join(" / "),
  },
]);

async function savePrimary() {
  catalogBusy.value = true;
  try {
    const tags = Array.from(primarySet.value);
    await api.setUserPrimaryTags(tags);
    savedSnapshot.value = new Set(tags);
    // 重新拉一次 stats（is_primary 字段需要后端视角刷新）
    tagStats.value = await api.listCatalogTagStats();
    hasUserCatalog.value = await api.hasUserCatalog();
    message.success(`已保存 ${tags.length} 个一级标签`);
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

function discardChanges() {
  primarySet.value = new Set(savedSnapshot.value);
}

// ========== 导出词典 ==========
const showExportFilter = ref(false);
const allCatalogTags = ref<string[]>([]);
const selectedExportTags = ref<string[]>([]);
const excludedExportTags = ref<string[]>([]);
const vaultOverrideExport = ref(false);
const exportScope = ref<"all" | "used" | "unused">("all");

async function handleExportCatalog() {
  catalogBusy.value = true;
  try {
    allCatalogTags.value = await api.listCatalogTags();
  } catch {
    allCatalogTags.value = [];
  } finally {
    catalogBusy.value = false;
  }
  selectedExportTags.value = [];
  excludedExportTags.value = [];
  vaultOverrideExport.value = false;
  exportScope.value = "all";
  showExportFilter.value = true;
}

async function confirmExportCatalog() {
  const path = await saveDialog({
    title: "导出网站词典",
    defaultPath: `site_catalog_${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  catalogBusy.value = true;
  try {
    const count = await api.exportSiteCatalog(
      path as string,
      selectedExportTags.value,
      excludedExportTags.value,
      vaultOverrideExport.value,
      exportScope.value,
    );
    message.success(`已导出 ${count} 个站点`);
    showExportFilter.value = false;
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

function handleResetCatalog() {
  dialog.warning({
    title: "确认恢复默认",
    content: "将清除所有自定义词典数据（包含一级标签选择），恢复为内置词典。此操作不可撤销。",
    positiveText: "确认恢复",
    negativeText: "取消",
    onPositiveClick: async () => {
      catalogBusy.value = true;
      try {
        await api.resetSiteCatalog();
        message.success("已恢复为默认词典");
        await loadAll();
      } catch (e: any) {
        message.error(`重置失败: ${e}`);
      } finally {
        catalogBusy.value = false;
      }
    },
  });
}

// ========== 预制身份词典导入 ==========
const showPresetDialog = ref(false);
const presetPersonas = ref<PresetPersonaInfo[]>([]);
/** 单选：当前选中的 persona id（""=未选） */
const selectedPersona = ref<string>("");
const overrideExistingTags = ref(false);

async function openPresetDialog() {
  if (presetPersonas.value.length === 0) {
    try {
      presetPersonas.value = await api.listPresetPersonas();
    } catch (e: any) {
      message.error(`加载预制词典失败: ${e}`);
      return;
    }
  }
  // 默认选第一个 default_enabled 的身份
  const defaultPersona = presetPersonas.value.find((p) => p.default_enabled);
  selectedPersona.value = defaultPersona?.persona ?? presetPersonas.value[0]?.persona ?? "";
  overrideExistingTags.value = false;
  showPresetDialog.value = true;
}

const personaLabels: Record<string, string> = {
  base: "国民底座",
  developer: "软件开发者",
  "game-dev": "游戏开发者",
  "cross-border": "跨境/外贸",
  creator: "内容创作者",
  "small-biz": "小微主理人",
  crypto: "加密货币（社区版）",
};

async function confirmImportPreset() {
  const persona = selectedPersona.value;
  if (!persona) {
    message.warning("请选择一个身份");
    return;
  }
  catalogBusy.value = true;
  try {
    const r = await api.importPresetPersona(persona, overrideExistingTags.value);
    const label = personaLabels[persona] ?? persona;
    message.success(
      `已导入 ${label}：新增 ${r.added}，覆盖 ${r.overwritten}，保留 ${r.kept}（一级标签已自动勾选）`,
    );
    showPresetDialog.value = false;
    await loadAll();
  } catch (e: any) {
    message.error(`导入失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}
</script>

<template>
  <div class="tag-dict-page">
    <div class="catalog-toolbar">
      <n-space align="center" :size="8">
        <n-button
          type="primary"
          size="small"
          :disabled="!isDirty"
          :loading="catalogBusy"
          @click="savePrimary"
        >
          <template #icon><n-icon :component="SaveOutline" /></template>
          保存一级标签
        </n-button>
        <n-button v-if="isDirty" size="small" quaternary @click="discardChanges">
          放弃改动
        </n-button>
        <n-button size="small" :loading="catalogBusy" @click="openPresetDialog">
          <template #icon><n-icon :component="DownloadOutline" /></template>
          导入预制词典
        </n-button>
        <n-button size="small" :loading="catalogBusy" @click="handleExportCatalog">
          <template #icon><n-icon :component="CloudUploadOutline" /></template>
          导出词典
        </n-button>
        <n-button
          v-if="hasUserCatalog"
          size="small"
          quaternary
          type="warning"
          :loading="catalogBusy"
          @click="handleResetCatalog"
        >
          <template #icon><n-icon :component="RefreshOutline" /></template>
          恢复默认
        </n-button>
      </n-space>
      <n-input
        v-model:value="keyword"
        placeholder="搜索标签…"
        clearable
        size="small"
        style="width: 220px"
      >
        <template #prefix>
          <n-icon :component="SearchOutline" />
        </template>
      </n-input>
    </div>

    <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 10px">
      勾选的一级标签会作为侧栏 chip 聚合维度，建议控制在 <n-text strong>16 个以内</n-text>。
    </n-text>

    <div v-if="loading" class="loading-hint">
      <n-text depth="3" style="font-size: 12px">加载中…</n-text>
    </div>
    <div v-else class="tag-dict-body">
      <!-- 左：标签清单 + 一级复选 -->
      <div class="pane pane-tags">
        <div class="pane-header">
          <n-text strong style="font-size: 13px">标签</n-text>
          <n-text depth="3" style="font-size: 12px">
            共 {{ totalTags }}，已选 <n-text strong>{{ primaryCount }}</n-text> 个一级
            <span v-if="isDirty" style="color: var(--n-color-warning, #f0a020)">·有未保存改动</span>
          </n-text>
        </div>
        <n-data-table
          class="pane-table"
          :columns="tagColumns"
          :data="filteredStats"
          :pagination="{ pageSize: 100 }"
          :flex-height="true"
          :row-class-name="(row: any) => isPrimary(row.tag) ? 'row-primary' : ''"
          :row-props="tagRowProps"
          size="small"
          :bordered="false"
          virtual-scroll
        />
      </div>

      <!-- 右：站点表 -->
      <div class="pane pane-sites">
        <div class="pane-header">
          <n-text strong style="font-size: 13px">站点</n-text>
          <n-text depth="3" style="font-size: 12px">共 {{ siteEntries.length }} 条</n-text>
          <n-input
            v-model:value="siteKeyword"
            placeholder="搜索域名 / 名称 / 标签…"
            clearable
            size="small"
            style="width: 240px; margin-left: auto"
          >
            <template #prefix>
              <n-icon :component="SearchOutline" />
            </template>
          </n-input>
        </div>
        <n-data-table
          class="pane-table"
          :columns="siteColumns"
          :data="filteredSites"
          :pagination="{ pageSize: 100 }"
          :flex-height="true"
          size="small"
          :bordered="false"
          virtual-scroll
        />
      </div>
    </div>

    <!-- 导出词典筛选 Modal -->
    <n-modal
      v-model:show="showExportFilter"
      preset="card"
      title="导出词典 — 标签筛选"
      style="width: 520px"
    >
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
        可按标签筛选要导出的内容，也可排除不想导出的标签分类。两者都不选则导出全部。
      </n-text>
      <n-form-item label="导出范围" label-placement="left" :show-feedback="false" style="margin-bottom: 12px">
        <n-radio-group v-model:value="exportScope" size="small">
          <n-radio-button value="all">全部</n-radio-button>
          <n-radio-button value="used">仅密库已用</n-radio-button>
          <n-radio-button value="unused">仅密库未用</n-radio-button>
        </n-radio-group>
      </n-form-item>
      <n-form-item label="只包含（可选）" label-placement="left" :show-feedback="false" style="margin-bottom: 12px">
        <n-select
          v-model:value="selectedExportTags"
          multiple
          filterable
          placeholder="不选 = 不限；可搜索/多选"
          :options="allCatalogTags.filter(t => !excludedExportTags.includes(t)).map(t => ({ label: t, value: t }))"
          max-tag-count="responsive"
        />
      </n-form-item>
      <n-form-item label="排除标签" label-placement="left" :show-feedback="false" style="margin-bottom: 12px">
        <n-select
          v-model:value="excludedExportTags"
          multiple
          filterable
          placeholder="选择要排除的标签"
          :options="allCatalogTags.filter(t => !selectedExportTags.includes(t)).map(t => ({ label: t, value: t }))"
          max-tag-count="responsive"
        />
      </n-form-item>
      <n-form-item label="密库覆盖" label-placement="left" :show-feedback="false" style="margin-bottom: 12px">
        <n-space align="center" :size="8" style="width: 100%">
          <n-switch v-model:value="vaultOverrideExport" size="small" />
          <n-text depth="3" style="font-size: 12px">
            开启后：词典中已存在的 host，用密码库当前的标签覆盖
          </n-text>
        </n-space>
      </n-form-item>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showExportFilter = false">取消</n-button>
          <n-button type="primary" :loading="catalogBusy" @click="confirmExportCatalog">
            确认导出
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- 导入预制词典 Modal -->
    <n-modal
      v-model:show="showPresetDialog"
      preset="card"
      title="导入预制词典"
      style="width: 560px"
    >
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
        请选择一个身份。导入后，<n-text strong>该身份的一级标签会自动勾选</n-text>，sites.json 全集站点会并入你的词典。
      </n-text>

      <div class="persona-list">
        <div
          v-for="p in presetPersonas"
          :key="p.persona"
          class="persona-item"
          :class="{ active: selectedPersona === p.persona }"
          @click="selectedPersona = p.persona"
        >
          <n-radio
            :checked="selectedPersona === p.persona"
            @update:checked="selectedPersona = p.persona"
            @click.stop
          />
          <div class="persona-body">
            <n-space :size="6" align="center" style="flex-wrap: wrap">
              <n-text strong>{{ personaLabels[p.persona] ?? p.persona }}</n-text>
              <n-text depth="3" style="font-size: 12px">
                {{ p.primary_tags.length }} 个一级标签
              </n-text>
              <n-tag v-if="p.community" size="tiny" type="warning" round>社区版</n-tag>
            </n-space>
            <n-text depth="3" style="font-size: 12px; display: block; line-height: 1.5; margin-top: 4px">
              {{ p.description }}
            </n-text>
            <n-space :size="4" style="margin-top: 4px; flex-wrap: wrap">
              <n-tag
                v-for="t in p.primary_tags"
                :key="t"
                size="tiny"
                :bordered="false"
              >{{ t }}</n-tag>
            </n-space>
          </div>
        </div>
      </div>

      <n-divider style="margin: 14px 0 10px" />

      <n-space align="center" :size="8">
        <n-switch v-model:value="overrideExistingTags" size="small" />
        <n-text style="font-size: 13px">同 host 已有标签时，用预制词典覆盖</n-text>
      </n-space>
      <n-text depth="3" style="font-size: 12px; display: block; margin-top: 4px; line-height: 1.6">
        关闭（推荐）：仅新增预制词典中尚未收录的 host，保留你当前的 tags 不变。<br />
        开启：用预制词典的 name + tags 覆盖你已有的同 host 条目。
      </n-text>

      <template #footer>
        <n-space justify="end">
          <n-button @click="showPresetDialog = false">取消</n-button>
          <n-button type="primary" :loading="catalogBusy" @click="confirmImportPreset">
            确认导入
          </n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.tag-dict-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.catalog-toolbar {
  flex: none;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.tag-table {
  flex: 1 1 auto;
  min-height: 0;
}

.tag-dict-body {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  gap: 12px;
  overflow: hidden;
}

.pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
  min-width: 0;
  border: 1px solid var(--app-border-color);
  border-radius: 8px;
  background: var(--app-card-bg);
  overflow: hidden;
}

.pane-tags {
  flex: 0 0 360px;
}

.pane-sites {
  flex: 1 1 auto;
}

.pane-header {
  flex: none;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--app-border-color);
}

.pane-table {
  flex: 1 1 auto;
  min-height: 0;
}

.loading-hint {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 0;
}

:deep(.row-primary td) {
  background-color: var(--app-active-bg, rgba(24, 160, 88, 0.06)) !important;
}

/* ====== 预制词典多选卡片 ====== */
.persona-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 380px;
  overflow-y: auto;
  padding-right: 4px;
}

.persona-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--app-border-color);
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.12s, border-color 0.12s;
}

.persona-item:hover {
  background-color: var(--app-hover-bg, rgba(0, 0, 0, 0.03));
}

.persona-item.active {
  border-color: var(--n-color-primary, #18a058);
  background-color: var(--app-active-bg, rgba(24, 160, 88, 0.06));
}

.persona-body {
  flex: 1;
  min-width: 0;
}
</style>
