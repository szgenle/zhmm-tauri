<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useMessage, useDialog } from "naive-ui";
import {
  AddOutline,
  TrashOutline,
  SwapHorizontalOutline,
  DownloadOutline,
  SearchOutline,
  CloudUploadOutline,
  RefreshOutline,
} from "@vicons/ionicons5";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type TagDefinition, type TagStats, type TagHierarchyNode, type SiteCatalogEntry } from "../api";

const message = useMessage();
const dialog = useDialog();

// ========== 页面 tab ==========
const activeTab = ref<"tags" | "catalog">("tags");

// ========== 数据 ==========
const registry = ref<TagDefinition[]>([]);
const stats = ref<TagStats[]>([]);
const hierarchy = ref<TagHierarchyNode[]>([]);
const catalogEntries = ref<SiteCatalogEntry[]>([]);
const loading = ref(false);
const selectedTag = ref<string | null>(null);
const keyword = ref("");

// ========== 加载 ==========
async function loadAll() {
  loading.value = true;
  try {
    const [reg, st, hier, catalog] = await Promise.all([
      api.listTagRegistry(),
      api.getTagStats(),
      api.getTagHierarchy(),
      api.listSiteCatalog(),
    ]);
    registry.value = reg;
    stats.value = st;
    hierarchy.value = hier;
    catalogEntries.value = catalog;
  } catch (e: any) {
    message.error(`加载标签数据失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

onMounted(loadAll);

// ========== 统计 ==========
const totalTags = computed(() => {
  const all = new Set<string>();
  for (const s of stats.value) all.add(s.name);
  for (const r of registry.value) all.add(r.name);
  return all.size;
});

const registeredCount = computed(() => registry.value.length);

const wildTags = computed(() => {
  const registered = new Set(registry.value.map((t) => t.name));
  return stats.value.filter((s) => !registered.has(s.name)).length;
});

const catalogTagCount = computed(() => {
  return stats.value.filter((s) => s.in_catalog).length;
});

// ========== 标签树（一级 + 子标签） ==========
interface TagItem {
  name: string;
  def: TagDefinition | null;
  stat: TagStats | null;
}

interface TreeNode {
  primary: TagItem;
  children: TagItem[];
  expanded: boolean;
}

// 展开状态缓存
const expandedMap = ref<Record<string, boolean>>({});

function toggleExpand(name: string) {
  expandedMap.value[name] = !expandedMap.value[name];
}

const tagTree = computed<TreeNode[]>(() => {
  const regMap = new Map(registry.value.map((r) => [r.name, r]));
  const statMap = new Map(stats.value.map((s) => [s.name, s]));

  const makeItem = (name: string): TagItem => ({
    name,
    def: regMap.get(name) || null,
    stat: statMap.get(name) || null,
  });

  // 从 hierarchy 构建树
  const tree: TreeNode[] = [];
  const childrenUsed = new Set<string>(); // 已作为子标签出现的

  for (const node of hierarchy.value) {
    for (const child of node.children) {
      childrenUsed.add(child);
    }
  }

  for (const node of hierarchy.value) {
    tree.push({
      primary: makeItem(node.name),
      children: node.children.map(makeItem),
      expanded: expandedMap.value[node.name] ?? true,
    });
  }

  // 补充注册表中有但不在 hierarchy 中的标签（没被任何条目使用的已注册标签）
  const hierNames = new Set(hierarchy.value.map((h) => h.name));
  for (const r of registry.value) {
    if (!hierNames.has(r.name) && !childrenUsed.has(r.name)) {
      tree.push({
        primary: makeItem(r.name),
        children: [],
        expanded: false,
      });
    }
  }
  // 补充 stats 中有但既不在 hierarchy 也不在 childrenUsed 中的野生标签
  for (const s of stats.value) {
    if (!hierNames.has(s.name) && !childrenUsed.has(s.name) && !registry.value.some((r) => r.name === s.name)) {
      tree.push({
        primary: makeItem(s.name),
        children: [],
        expanded: false,
      });
    }
  }

  return tree;
});

const filteredTree = computed<TreeNode[]>(() => {
  const kw = keyword.value.trim().toLowerCase();
  if (!kw) return tagTree.value;
  return tagTree.value.filter((node) => {
    if (node.primary.name.toLowerCase().includes(kw)) return true;
    return node.children.some((c) => c.name.toLowerCase().includes(kw));
  });
});

// ========== 选中详情 ==========
const selectedDef = computed(() => {
  if (!selectedTag.value) return null;
  return registry.value.find((r) => r.name === selectedTag.value) || null;
});

const selectedStat = computed(() => {
  if (!selectedTag.value) return null;
  return stats.value.find((s) => s.name === selectedTag.value) || null;
});

const selectedCatalogEntries = computed(() => {
  if (!selectedTag.value) return [];
  const tag = selectedTag.value;
  return catalogEntries.value.filter((e) => e.tags.includes(tag));
});

const isRegistered = computed(() => selectedDef.value !== null);

// ========== 编辑表单 ==========
const editForm = ref({
  color: "",
  icon: "",
  order: 0,
});

watch(selectedDef, (def) => {
  if (def) {
    editForm.value = {
      color: def.color,
      icon: def.icon,
      order: def.order,
    };
  } else {
    editForm.value = { color: "", icon: "", order: 0 };
  }
});

// ========== 标签操作 ==========
async function handleSave() {
  if (!selectedTag.value) return;
  try {
    await api.upsertTagDef({
      name: selectedTag.value,
      color: editForm.value.color,
      icon: editForm.value.icon,
      order: editForm.value.order,
      source: "",
    });
    message.success("已保存");
    await loadAll();
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  }
}

async function handleRegister() {
  if (!selectedTag.value) return;
  try {
    await api.upsertTagDef({
      name: selectedTag.value,
      color: "",
      icon: "",
      order: 0,
      source: "manual",
    });
    message.success(`已注册标签「${selectedTag.value}」`);
    await loadAll();
  } catch (e: any) {
    message.error(`注册失败: ${e}`);
  }
}

async function handleRemoveFromRegistry() {
  if (!selectedTag.value) return;
  const tag = selectedTag.value;
  dialog.warning({
    title: "从注册表移除",
    content: `从注册表移除「${tag}」？已有条目的标签不会被删除。`,
    positiveText: "移除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.removeTagDef(tag);
        message.success("已移除");
        selectedTag.value = null;
        await loadAll();
      } catch (e: any) {
        message.error(`移除失败: ${e}`);
      }
    },
  });
}

async function handleDelete() {
  if (!selectedTag.value) return;
  const tag = selectedTag.value;
  const count = selectedStat.value?.count || 0;
  dialog.warning({
    title: "彻底删除标签",
    content: `确定要彻底删除标签「${tag}」吗？将从注册表移除，并从 ${count} 条记录中清除。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.removeTagDef(tag);
        await api.deleteTag(tag);
        message.success(`已删除标签「${tag}」`);
        selectedTag.value = null;
        await loadAll();
      } catch (e: any) {
        message.error(`删除失败: ${e}`);
      }
    },
  });
}

async function handleRename() {
  if (!selectedTag.value) return;
  const old = selectedTag.value;
  const newName = window.prompt(`将「${old}」重命名为:`, old);
  if (!newName || newName.trim() === "" || newName.trim() === old) return;
  try {
    const affected = await api.renameTag(old, newName.trim());
    if (selectedDef.value) {
      await api.removeTagDef(old);
      await api.upsertTagDef({
        ...selectedDef.value,
        name: newName.trim(),
      });
    }
    message.success(`已重命名，影响 ${affected} 条记录`);
    selectedTag.value = newName.trim();
    await loadAll();
  } catch (e: any) {
    message.error(`重命名失败: ${e}`);
  }
}

// 合并标签
const mergeTarget = ref("");
async function handleMerge() {
  if (!selectedTag.value || !mergeTarget.value) return;
  const source = selectedTag.value;
  const target = mergeTarget.value.trim();
  if (!target || target === source) return;

  dialog.warning({
    title: "合并标签",
    content: `将「${source}」合并到「${target}」？所有使用「${source}」的条目将改为使用「${target}」。`,
    positiveText: "合并",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        const affected = await api.mergeTags([source], target);
        message.success(`合并完成，影响 ${affected} 条记录`);
        mergeTarget.value = "";
        selectedTag.value = target;
        await loadAll();
      } catch (e: any) {
        message.error(`合并失败: ${e}`);
      }
    },
  });
}

// 新增标签
const newTagName = ref("");
async function handleAddTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  try {
    await api.upsertTagDef({
      name,
      color: "",
      icon: "",
      order: registry.value.length,
      source: "manual",
    });
    message.success(`已添加标签「${name}」`);
    newTagName.value = "";
    await loadAll();
    selectedTag.value = name;
  } catch (e: any) {
    message.error(`添加失败: ${e}`);
  }
}

// 批量导入未注册标签
async function handleImportWild() {
  const registered = new Set(registry.value.map((t) => t.name));
  const wild = stats.value.filter((s) => !registered.has(s.name));
  if (wild.length === 0) {
    message.info("没有需要导入的未注册标签");
    return;
  }
  dialog.info({
    title: "批量注册",
    content: `将 ${wild.length} 个未注册标签全部加入注册表？`,
    positiveText: "确认",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        const items: TagDefinition[] = [
          ...registry.value,
          ...wild.map((w, i) => ({
            name: w.name,
            color: "",
            icon: "",
            order: registry.value.length + i,
            source: "import",
          })),
        ];
        await api.saveTagRegistry(items);
        message.success(`已导入 ${wild.length} 个标签`);
        await loadAll();
      } catch (e: any) {
        message.error(`导入失败: ${e}`);
      }
    },
  });
}

// 预设颜色板
const presetColors = [
  "#18a058", "#2080f0", "#f0a020", "#d03050",
  "#8a2be2", "#36ad6a", "#4098fc", "#e88080",
  "#9b59b6", "#1abc9c", "#e67e22", "#607d8b",
];

// ========== 站点词典管理 ==========
const catalogBusy = ref(false);
const hasUserCatalog = ref(false);
const catalogKeyword = ref("");

async function checkUserCatalog() {
  try {
    hasUserCatalog.value = await api.hasUserCatalog();
  } catch {
    hasUserCatalog.value = false;
  }
}
checkUserCatalog();

const filteredCatalog = computed(() => {
  const kw = catalogKeyword.value.trim().toLowerCase();
  if (!kw) return catalogEntries.value;
  return catalogEntries.value.filter(
    (e) =>
      e.name.toLowerCase().includes(kw) ||
      e.host.toLowerCase().includes(kw) ||
      e.tags.some((t) => t.toLowerCase().includes(kw)),
  );
});

// 导出词典标签筛选
const showExportFilter = ref(false);
const allCatalogTags = ref<string[]>([]);
const selectedExportTags = ref<string[]>([]);
const excludedExportTags = ref<string[]>([]);

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
    const count = await api.exportSiteCatalog(path as string, selectedExportTags.value, excludedExportTags.value);
    const hints: string[] = [];
    if (selectedExportTags.value.length > 0) hints.push(`包含 ${selectedExportTags.value.length} 个标签`);
    if (excludedExportTags.value.length > 0) hints.push(`排除 ${excludedExportTags.value.length} 个标签`);
    const hint = hints.length > 0
      ? `已导出 ${count} 个站点（${hints.join("，")}）`
      : `已导出 ${count} 个站点（含密码库中的网站）`;
    message.success(hint);
    showExportFilter.value = false;
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

async function handleImportCatalog() {
  const path = await openDialog({
    title: "选择词典 JSON 文件",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  catalogBusy.value = true;
  try {
    const count = await api.importSiteCatalog(path as string);
    message.success(`已导入 ${count} 个站点`);
    hasUserCatalog.value = true;
    await loadAll();
  } catch (e: any) {
    message.error(`导入失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

function handleResetCatalog() {
  dialog.warning({
    title: "确认恢复默认",
    content: "将清除所有自定义词典数据，恢复为内置词典。此操作不可撤销。",
    positiveText: "确认恢复",
    negativeText: "取消",
    onPositiveClick: async () => {
      catalogBusy.value = true;
      try {
        await api.resetSiteCatalog();
        message.success("已恢复为默认词典");
        hasUserCatalog.value = false;
        await loadAll();
      } catch (e: any) {
        message.error(`重置失败: ${e}`);
      } finally {
        catalogBusy.value = false;
      }
    },
  });
}
</script>

<template>
  <div class="tag-dict-page">
    <!-- 顶部统计 + Tab 切换 -->
    <div class="stats-bar">
      <n-space :size="24" align="center">
        <n-statistic label="总标签数" :value="totalTags" />
        <n-statistic label="已注册" :value="registeredCount" />
        <n-statistic label="未注册" :value="wildTags" />
        <n-statistic label="词典引用" :value="catalogTagCount" />
      </n-space>
    </div>

    <n-tabs v-model:value="activeTab" type="segment" size="small" style="margin-bottom: 12px">
      <n-tab name="tags">标签管理</n-tab>
      <n-tab name="catalog">站点词典</n-tab>
    </n-tabs>

    <!-- ====== 标签管理 Tab ====== -->
    <div v-show="activeTab === 'tags'" class="main-body">
      <!-- 左侧：标签列表 -->
      <div class="left-panel">
        <div class="panel-toolbar">
          <n-input
            v-model:value="keyword"
            placeholder="搜索标签…"
            clearable
            size="small"
            style="flex: 1"
          >
            <template #prefix>
              <n-icon :component="SearchOutline" />
            </template>
          </n-input>
        </div>

        <div class="panel-actions">
          <n-input
            v-model:value="newTagName"
            placeholder="新标签名"
            size="small"
            style="flex: 1"
            @keyup.enter="handleAddTag"
          />
          <n-button size="small" @click="handleAddTag" :disabled="!newTagName.trim()">
            <template #icon><n-icon :component="AddOutline" /></template>
          </n-button>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button size="small" quaternary @click="handleImportWild" :disabled="wildTags === 0">
                <template #icon><n-icon :component="DownloadOutline" /></template>
              </n-button>
            </template>
            批量注册所有未注册标签
          </n-tooltip>
        </div>

        <n-spin :show="loading" class="list-spin">
          <div class="tag-list">
            <div v-if="filteredTree.length === 0" class="empty-hint">
              {{ keyword ? "没有匹配的标签" : "暂无标签" }}
            </div>
            <template v-for="node in filteredTree" :key="node.primary.name">
              <!-- 一级标签 -->
              <div
                class="tag-item primary-item"
                :class="{ active: selectedTag === node.primary.name, wild: !node.primary.def }"
                @click="selectedTag = node.primary.name"
              >
                <span
                  v-if="node.children.length > 0"
                  class="expand-icon"
                  @click.stop="toggleExpand(node.primary.name)"
                >{{ (expandedMap[node.primary.name] ?? true) ? '▼' : '▶' }}</span>
                <span v-else class="expand-icon placeholder">·</span>
                <span v-if="node.primary.def?.icon" class="item-icon">{{ node.primary.def.icon }}</span>
                <span
                  class="item-name"
                  :style="node.primary.def?.color ? { color: node.primary.def.color } : {}"
                >{{ node.primary.name }}</span>
                <span class="item-count" v-if="node.primary.stat">{{ node.primary.stat.count }}</span>
                <n-tag v-if="node.primary.def?.source === 'manual'" size="tiny" type="success" :bordered="false">手动</n-tag>
                <n-tag v-if="!node.primary.def" size="tiny" type="warning" :bordered="false">未注册</n-tag>
              </div>
              <!-- 子标签 -->
              <template v-if="(expandedMap[node.primary.name] ?? true) && node.children.length > 0">
                <div
                  v-for="child in node.children"
                  :key="child.name"
                  class="tag-item child-item"
                  :class="{ active: selectedTag === child.name, wild: !child.def }"
                  @click="selectedTag = child.name"
                >
                  <span v-if="child.def?.icon" class="item-icon">{{ child.def.icon }}</span>
                  <span
                    class="item-name"
                    :style="child.def?.color ? { color: child.def.color } : {}"
                  >{{ child.name }}</span>
                  <span class="item-count" v-if="child.stat">{{ child.stat.count }}</span>
                  <n-tag v-if="child.def?.source === 'manual'" size="tiny" type="success" :bordered="false">手动</n-tag>
                  <n-tag v-if="!child.def" size="tiny" type="warning" :bordered="false">未注册</n-tag>
                </div>
              </template>
            </template>
          </div>
        </n-spin>
      </div>

      <!-- 右侧：详情面板 -->
      <div class="right-panel">
        <div v-if="!selectedTag" class="empty-detail">
          <n-text depth="3">选择左侧标签查看详情</n-text>
        </div>
        <template v-else>
          <div class="detail-header">
            <h3 class="detail-title">
              <span v-if="selectedDef?.icon">{{ selectedDef.icon }} </span>{{ selectedTag }}
            </h3>
            <n-space :size="8">
              <n-button size="small" quaternary @click="handleRename">重命名</n-button>
              <n-button v-if="!isRegistered" size="small" type="primary" @click="handleRegister">
                注册
              </n-button>
            </n-space>
          </div>

          <!-- 使用统计 -->
          <n-card size="small" style="margin-bottom: 12px" :bordered="false" embedded>
            <n-space :size="16">
              <n-text>使用次数: <strong>{{ selectedStat?.count || 0 }}</strong></n-text>
              <n-text>一级引用: <strong>{{ selectedStat?.primary_count || 0 }}</strong></n-text>
              <n-tag v-if="selectedStat?.in_catalog" size="small" type="info" :bordered="false">
                站点词典
              </n-tag>
              <n-tag v-if="selectedDef?.source === 'manual'" size="small" type="success" :bordered="false">
                手动创建
              </n-tag>
              <n-tag v-else-if="selectedDef?.source === 'import'" size="small" type="warning" :bordered="false">
                导入词典
              </n-tag>
            </n-space>
          </n-card>

          <!-- 属性编辑（仅已注册的标签） -->
          <template v-if="isRegistered">
            <n-card size="small" title="属性" style="margin-bottom: 12px">
              <n-space vertical :size="12">
                <div class="form-row">
                  <label>图标</label>
                  <n-input
                    v-model:value="editForm.icon"
                    placeholder="输入 emoji"
                    size="small"
                    style="width: 80px"
                  />
                </div>
                <div class="form-row">
                  <label>颜色</label>
                  <n-space :size="4" align="center">
                    <div
                      v-for="c in presetColors"
                      :key="c"
                      class="color-swatch"
                      :class="{ active: editForm.color === c }"
                      :style="{ background: c }"
                      @click="editForm.color = editForm.color === c ? '' : c"
                    />
                    <n-input
                      v-model:value="editForm.color"
                      placeholder="#hex"
                      size="small"
                      style="width: 80px"
                    />
                  </n-space>
                </div>
                <div class="form-row">
                  <label>排序</label>
                  <n-input-number
                    v-model:value="editForm.order"
                    size="small"
                    style="width: 100px"
                    :min="-999"
                    :max="9999"
                  />
                </div>
                <n-button size="small" type="primary" @click="handleSave">保存修改</n-button>
              </n-space>
            </n-card>
          </template>

          <!-- 站点词典关联 -->
          <n-card
            v-if="selectedCatalogEntries.length > 0"
            size="small"
            title="站点词典关联"
            style="margin-bottom: 12px"
          >
            <div class="catalog-list">
              <div v-for="entry in selectedCatalogEntries.slice(0, 20)" :key="entry.host" class="catalog-item">
                <n-text style="font-size: 12px">{{ entry.name || entry.host }}</n-text>
                <n-text depth="3" style="font-size: 11px; margin-left: 8px">{{ entry.host }}</n-text>
              </div>
              <n-text v-if="selectedCatalogEntries.length > 20" depth="3" style="font-size: 11px">
                …等 {{ selectedCatalogEntries.length }} 条
              </n-text>
            </div>
          </n-card>

          <!-- 合并操作 -->
          <n-card size="small" title="合并" style="margin-bottom: 12px">
            <n-space align="center" :size="8">
              <n-icon :component="SwapHorizontalOutline" :size="16" />
              <n-text style="font-size: 12px">合并到:</n-text>
              <n-input
                v-model:value="mergeTarget"
                placeholder="目标标签名"
                size="small"
                style="width: 140px"
              />
              <n-button
                size="small"
                :disabled="!mergeTarget.trim() || mergeTarget.trim() === selectedTag"
                @click="handleMerge"
              >合并</n-button>
            </n-space>
          </n-card>

          <!-- 危险操作 -->
          <n-card size="small" style="margin-bottom: 12px">
            <n-space :size="8">
              <n-button
                v-if="isRegistered"
                size="small"
                quaternary
                type="warning"
                @click="handleRemoveFromRegistry"
              >从注册表移除</n-button>
              <n-button size="small" type="error" @click="handleDelete">
                <template #icon><n-icon :component="TrashOutline" /></template>
                彻底删除
              </n-button>
            </n-space>
          </n-card>
        </template>
      </div>
    </div>

    <!-- ====== 站点词典 Tab ====== -->
    <div v-show="activeTab === 'catalog'" class="catalog-body">
      <div class="catalog-toolbar">
        <n-space align="center" :size="8">
          <n-button type="primary" size="small" :loading="catalogBusy" @click="handleExportCatalog">
            <template #icon><n-icon :component="CloudUploadOutline" /></template>
            导出词典
          </n-button>
          <n-button size="small" :loading="catalogBusy" @click="handleImportCatalog">
            <template #icon><n-icon :component="DownloadOutline" /></template>
            导入词典
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
          <n-tag v-if="hasUserCatalog" type="info" size="small">已自定义</n-tag>
        </n-space>
        <n-input
          v-model:value="catalogKeyword"
          placeholder="搜索站点/标签…"
          clearable
          size="small"
          style="width: 200px"
        >
          <template #prefix>
            <n-icon :component="SearchOutline" />
          </template>
        </n-input>
      </div>

      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 10px">
        站点词典共 {{ catalogEntries.length }} 条，筛选结果 {{ filteredCatalog.length }} 条。
        可导出后交给 AI 按行业重新整理标签，再导入生效。
      </n-text>

      <div class="catalog-table-wrap">
        <n-data-table
          :columns="[
            { title: '站点名', key: 'name', width: 160, ellipsis: { tooltip: true } },
            { title: '域名', key: 'host', width: 200, ellipsis: { tooltip: true } },
            { title: '标签', key: 'tags', render: (row: any) => row.tags?.join(', ') || '' },
          ]"
          :data="filteredCatalog"
          :max-height="480"
          size="small"
          :bordered="false"
          virtual-scroll
          flex-height
        />
      </div>
    </div>

    <!-- 导出词典标签筛选 Modal -->
    <n-modal
      v-model:show="showExportFilter"
      preset="card"
      title="导出词典 — 标签筛选"
      style="width: 520px"
    >
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
        可按标签筛选要导出的内容，也可排除不想导出的标签分类。两者都不选则导出全部。
      </n-text>
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
      <n-text v-if="selectedExportTags.length > 0 || excludedExportTags.length > 0" depth="3" style="font-size: 12px">
        <span v-if="selectedExportTags.length > 0">仅导出含「{{ selectedExportTags.join('、') }}」的站点</span>
        <span v-if="selectedExportTags.length > 0 && excludedExportTags.length > 0">，</span>
        <span v-if="excludedExportTags.length > 0">排除含「{{ excludedExportTags.join('、') }}」的站点</span>
      </n-text>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showExportFilter = false">取消</n-button>
          <n-button type="primary" :loading="catalogBusy" @click="confirmExportCatalog">
            确认导出
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

.stats-bar {
  flex: none;
  padding: 0 0 12px;
  border-bottom: 1px solid var(--app-border-color);
  margin-bottom: 12px;
}

.main-body {
  flex: 1;
  display: flex;
  gap: 16px;
  min-height: 0;
  overflow: hidden;
}

.left-panel {
  width: 300px;
  min-width: 240px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--app-border-color);
  padding-right: 16px;
  min-height: 0;
  overflow: hidden;
}

.panel-toolbar {
  flex: none;
  margin-bottom: 8px;
}

.panel-actions {
  flex: none;
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}

.list-spin {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.list-spin :deep(.n-spin-container),
.list-spin :deep(.n-spin-content) {
  height: 100%;
}

.tag-list {
  height: 100%;
  overflow-y: auto;
  padding: 2px 0;
}

.empty-hint {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  padding: 24px 0;
  text-align: center;
  opacity: 0.8;
}

.tag-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s ease;
  font-size: 13px;
}
.tag-item.child-item {
  padding-left: 28px;
  font-size: 12px;
  opacity: 0.85;
}
.tag-item:hover {
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
}
.tag-item.active {
  background: var(--n-color-pressed, rgba(24, 160, 88, 0.12));
}
.tag-item.wild {
  opacity: 0.7;
}

.expand-icon {
  flex: none;
  width: 14px;
  font-size: 10px;
  text-align: center;
  cursor: pointer;
  color: var(--n-text-color-3, #888);
  user-select: none;
}
.expand-icon.placeholder {
  cursor: default;
  opacity: 0.4;
}

.item-icon {
  font-size: 14px;
  flex: none;
}
.item-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-count {
  font-size: 11px;
  color: var(--n-text-color-3, #888);
  flex: none;
}

.right-panel {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
}

.empty-detail {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}
.detail-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.form-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.form-row label {
  width: 40px;
  font-size: 12px;
  color: var(--n-text-color-3, #666);
  flex: none;
}

.color-swatch {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: border-color 0.15s ease, transform 0.15s ease;
}
.color-swatch:hover {
  transform: scale(1.2);
}
.color-swatch.active {
  border-color: var(--n-text-color, #333);
}

.catalog-list {
  max-height: 160px;
  overflow-y: auto;
}
.catalog-item {
  display: flex;
  align-items: center;
  padding: 3px 0;
}

/* ====== 站点词典 Tab ====== */
.catalog-body {
  flex: 1;
  display: flex;
  flex-direction: column;
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

.catalog-table-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>
