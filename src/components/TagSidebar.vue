<script lang="ts">
export const UNCATEGORIZED_TAG = "__uncategorized__";
</script>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { AddOutline, CreateOutline, TrashOutline } from "@vicons/ionicons5";
import { useDialog } from "naive-ui";
import type { PasswordSummary } from "../api";
import {
  useCustomTagRules,
  matchCustomRule,
  customTagSentinel,
  type CustomTagRule,
} from "../composables/useCustomTagRules";

const props = withDefaults(defineProps<{
  entries: PasswordSummary[];
  selectedTags: string[];
  /** 选择模式：single 单选（点同项取消）/ multi 多选 */
  mode?: "single" | "multi";
  /** 一级标签排序模式：按频次降序 / 按最近使用 */
  sortMode?: "frequency" | "recent";
  /** sortMode='recent' 时用于持久化最近使用顺序的 localStorage key */
  recentTagsKey?: string;
}>(), {
  mode: "single",
  sortMode: "frequency",
});

const emit = defineEmits<{
  (e: "update:selectedTags", value: string[]): void;
}>();

/** 子标签最低出现次数阈值 */
const CHILD_MIN_COUNT = 5;
/** 最近使用最多保留多少条 */
const MAX_RECENT = 50;

/** 加载最近使用列表 */
function loadRecent(): string[] {
  if (!props.recentTagsKey) return [];
  try {
    const stored = localStorage.getItem(props.recentTagsKey);
    if (stored) {
      const arr = JSON.parse(stored);
      if (Array.isArray(arr)) return arr.filter((x) => typeof x === "string");
    }
  } catch { /* ignore */ }
  return [];
}

const recentTags = ref<string[]>(loadRecent());

watch(() => props.recentTagsKey, () => {
  recentTags.value = loadRecent();
});

function touchRecent(tag: string) {
  if (props.sortMode !== "recent" || !props.recentTagsKey) return;
  if (!tag || tag === UNCATEGORIZED_TAG) return;
  const list = recentTags.value.filter((t) => t !== tag);
  list.unshift(tag);
  if (list.length > MAX_RECENT) list.length = MAX_RECENT;
  recentTags.value = list;
  try {
    localStorage.setItem(props.recentTagsKey, JSON.stringify(list));
  } catch { /* ignore */ }
}

interface TagNode {
  tag: string;
  label: string;
  count: number;
  checked: boolean;
  children: TagChild[];
  expanded: boolean;
}

interface TagChild {
  tag: string;
  label: string;
  count: number;
  checked: boolean;
}

// 展开状态缓存
const expandedMap = ref<Record<string, boolean>>({});

function toggleExpand(tag: string) {
  expandedMap.value[tag] = !(expandedMap.value[tag] ?? true);
}

const tagTree = computed<{ nodes: TagNode[]; uncategorizedCount: number }>(() => {
  // 统计一级标签(tags[0])计数
  const primaryCounter = new Map<string, number>();
  // 统计每个一级标签下的子标签(tags[1:])计数
  const childrenCounter = new Map<string, Map<string, number>>();
  let uncategorizedCount = 0;

  for (const entry of props.entries) {
    const tags = (entry.tags || []).filter((t) => !!t);
    if (tags.length === 0) {
      uncategorizedCount += 1;
      continue;
    }
    const primary = tags[0];
    primaryCounter.set(primary, (primaryCounter.get(primary) || 0) + 1);

    // 收集子标签
    for (let i = 1; i < tags.length; i++) {
      const child = tags[i];
      if (!child) continue;
      if (!childrenCounter.has(primary)) {
        childrenCounter.set(primary, new Map());
      }
      const cm = childrenCounter.get(primary)!;
      cm.set(child, (cm.get(child) || 0) + 1);
    }
  }

  // 一级标签排序：频次降序 / 最近使用
  let sorted: [string, number][];
  if (props.sortMode === "recent") {
    const seen = new Set<string>();
    sorted = [];
    // 1. 先按 recentTags 顺序
    for (const r of recentTags.value) {
      if (primaryCounter.has(r) && !seen.has(r)) {
        sorted.push([r, primaryCounter.get(r)!]);
        seen.add(r);
      }
    }
    // 2. 剩余按字母序补充
    const rest = [...primaryCounter.entries()]
      .filter(([t]) => !seen.has(t))
      .sort((a, b) => a[0].localeCompare(b[0]));
    sorted.push(...rest);
  } else {
    sorted = [...primaryCounter.entries()].sort((a, b) => {
      if (b[1] !== a[1]) return b[1] - a[1];
      return a[0].localeCompare(b[0]);
    });
  }

  const selected = new Set(props.selectedTags);

  const nodes: TagNode[] = sorted.map(([tag, count]) => {
    const cm = childrenCounter.get(tag);
    let children: TagChild[] = [];
    if (cm) {
      children = [...cm.entries()]
        .filter(([, c]) => c >= CHILD_MIN_COUNT)
        .sort((a, b) => b[1] - a[1])
        .map(([childTag, childCount]) => ({
          tag: childTag,
          label: childTag,
          count: childCount,
          checked: selected.has(childTag),
        }));
    }
    return {
      tag,
      label: tag,
      count,
      checked: selected.has(tag),
      children,
      expanded: expandedMap.value[tag] ?? true,
    };
  });

  return { nodes, uncategorizedCount };
});

const uncategorizedChecked = computed(() =>
  props.selectedTags.includes(UNCATEGORIZED_TAG),
);

// ========== 自定义标签（关键字筛选）==========
const { rules, addRule, updateRule, removeRule } = useCustomTagRules();
const dialog = useDialog();

interface CustomRuleView {
  rule: CustomTagRule;
  sentinel: string;
  count: number;
  checked: boolean;
}

const customRuleViews = computed<CustomRuleView[]>(() => {
  const selected = new Set(props.selectedTags);
  return rules.value.map((rule) => {
    const sentinel = customTagSentinel(rule);
    let count = 0;
    for (const entry of props.entries) {
      if (matchCustomRule(rule, entry.url)) count += 1;
    }
    return { rule, sentinel, count, checked: selected.has(sentinel) };
  });
});

// 编辑弹窗状态
const showEditDialog = ref(false);
const editingId = ref<string | null>(null); // null = 新建
const editName = ref("");
const editKeywords = ref<string[]>([]);

function openCreate() {
  editingId.value = null;
  editName.value = "";
  editKeywords.value = [];
  showEditDialog.value = true;
}

function openEdit(rule: CustomTagRule) {
  editingId.value = rule.id;
  editName.value = rule.name;
  editKeywords.value = [...rule.keywords];
  showEditDialog.value = true;
}

const canSaveEdit = computed(() => {
  return editName.value.trim().length > 0 && editKeywords.value.some((k) => k.trim());
});

function saveEdit() {
  if (!canSaveEdit.value) return;
  if (editingId.value) {
    updateRule(editingId.value, {
      name: editName.value,
      keywords: editKeywords.value,
    });
  } else {
    addRule(editName.value, editKeywords.value);
  }
  showEditDialog.value = false;
}

function confirmDelete(rule: CustomTagRule) {
  dialog.warning({
    title: "删除自定义标签",
    content: `确定要删除自定义标签「${rule.name}」吗？`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: () => {
      const sentinel = customTagSentinel(rule);
      // 当前若选中该标签，先清除选中
      if (props.selectedTags.includes(sentinel)) {
        emit(
          "update:selectedTags",
          props.selectedTags.filter((t) => t !== sentinel),
        );
      }
      removeRule(rule.id);
    },
  });
}

function toggle(tag: string) {
  if (props.mode === "single") {
    if (props.selectedTags.includes(tag)) {
      // 点击同项 -> 取消选中
      emit("update:selectedTags", []);
    } else {
      emit("update:selectedTags", [tag]);
      touchRecent(tag);
    }
    return;
  }
  // 多选
  const set = new Set(props.selectedTags);
  if (set.has(tag)) {
    set.delete(tag);
  } else {
    set.add(tag);
    touchRecent(tag);
  }
  emit("update:selectedTags", [...set]);
}

function clearSelection() {
  emit("update:selectedTags", []);
}

const collapsed = ref(false);
</script>

<template>
  <div class="tag-sidebar" :class="{ collapsed }">
    <div class="sidebar-header">
      <span class="sidebar-title" v-if="!collapsed">标签</span>
      <n-button text size="tiny" @click="collapsed = !collapsed" :title="collapsed ? '展开' : '收起'">
        {{ collapsed ? "»" : "«" }}
      </n-button>
      <n-button v-if="!collapsed && props.selectedTags.length" text size="tiny" @click="clearSelection" style="margin-left: auto">
        清空
      </n-button>
    </div>
    <div v-if="!collapsed" class="sidebar-body">
      <div class="tag-tree">
        <!-- 全部 -->
        <div
          class="tree-item all-item"
          :class="{ active: props.selectedTags.length === 0 }"
          @click="clearSelection"
        >
          <span class="item-label">全部</span>
          <span class="item-count">{{ props.entries.length }}</span>
        </div>

        <!-- ====== 自定义筛选 ====== -->
        <div class="section-header">
          <span>自定义筛选</span>
          <n-button
            text
            size="tiny"
            class="add-btn"
            title="新建自定义标签"
            @click="openCreate"
          >
            <template #icon>
              <n-icon :component="AddOutline" />
            </template>
          </n-button>
        </div>
        <div v-if="customRuleViews.length === 0" class="custom-empty">
          点 + 按关键字筛选网址
        </div>
        <template v-else>
          <div
            v-for="view in customRuleViews"
            :key="view.sentinel"
            class="tree-item custom-item"
            :class="{ active: view.checked }"
            @click="toggle(view.sentinel)"
          >
            <span class="primary-marker">◈</span>
            <span class="item-label">{{ view.rule.name }}</span>
            <span class="item-count">{{ view.count }}</span>
            <span class="custom-actions" @click.stop>
              <n-button
                text
                size="tiny"
                title="编辑"
                @click="openEdit(view.rule)"
              >
                <template #icon>
                  <n-icon :component="CreateOutline" />
                </template>
              </n-button>
              <n-button
                text
                size="tiny"
                title="删除"
                @click="confirmDelete(view.rule)"
              >
                <template #icon>
                  <n-icon :component="TrashOutline" />
                </template>
              </n-button>
            </span>
          </div>
        </template>

        <!-- ====== 标签 ====== -->
        <div
          v-if="tagTree.nodes.length || tagTree.uncategorizedCount > 0"
          class="section-header"
        >
          <span>标签</span>
        </div>
        <div
          v-if="!tagTree.nodes.length && tagTree.uncategorizedCount === 0 && customRuleViews.length === 0"
          class="empty-hint"
        >
          暂无标签。<br />编辑条目时添加标签即可在此筛选。
        </div>

        <!-- 一级标签 -->
        <template v-for="node in tagTree.nodes" :key="node.tag">
          <div
            class="tree-item primary-item"
            :class="{ active: node.checked }"
            @click="toggle(node.tag)"
          >
            <span
              v-if="node.children.length > 0"
              class="expand-btn"
              @click.stop="toggleExpand(node.tag)"
            >{{ (expandedMap[node.tag] ?? true) ? '▼' : '▶' }}</span>
            <span v-else class="primary-marker">◆</span>
            <span class="item-label">{{ node.label }}</span>
            <span class="item-count">{{ node.count }}</span>
          </div>
          <!-- 子标签 -->
          <template v-if="(expandedMap[node.tag] ?? true) && node.children.length > 0">
            <div
              v-for="child in node.children"
              :key="child.tag"
              class="tree-item child-item"
              :class="{ active: child.checked }"
              @click="toggle(child.tag)"
            >
              <span class="item-label">{{ child.label }}</span>
              <span class="item-count">{{ child.count }}</span>
            </div>
          </template>
        </template>

        <!-- 未分类 -->
        <div
          v-if="tagTree.uncategorizedCount > 0"
          class="tree-item uncategorized-item"
          :class="{ active: uncategorizedChecked }"
          @click="toggle(UNCATEGORIZED_TAG)"
        >
          <span class="primary-marker">◇</span>
          <span class="item-label">未分类</span>
          <span class="item-count">{{ tagTree.uncategorizedCount }}</span>
        </div>
      </div>
    </div>

    <!-- 自定义标签 编辑 / 新建 弹窗 -->
    <n-modal
      v-model:show="showEditDialog"
      preset="card"
      :title="editingId ? '编辑自定义标签' : '新建自定义标签'"
      style="width: 420px"
      :mask-closable="false"
    >
      <n-form-item label="名称" :show-feedback="false" style="margin-bottom: 12px">
        <n-input v-model:value="editName" placeholder="如：工作邮箱" maxlength="20" show-count />
      </n-form-item>
      <n-form-item label="关键字" :show-feedback="false">
        <div style="width: 100%">
          <n-dynamic-tags v-model:value="editKeywords" />
          <div class="hint-text">
            匹配条目的<strong>网址 / 域名</strong>，任一关键字命中即算入此标签；
            不区分大小写，可填如 <code>icbc.com</code>、<code>github</code>。
          </div>
        </div>
      </n-form-item>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showEditDialog = false">取消</n-button>
          <n-button type="primary" :disabled="!canSaveEdit" @click="saveEdit">
            保存
          </n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.tag-sidebar {
  width: 200px;
  min-width: 200px;
  background: var(--app-sidebar-bg);
  border-right: 1px solid var(--app-border-color);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
  height: 100%;
  transition: width 0.25s ease, min-width 0.25s ease, background 0.3s ease;
  backdrop-filter: blur(8px);
}
.tag-sidebar.collapsed {
  width: 36px;
  min-width: 36px;
}
.sidebar-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 12px 8px;
  font-weight: 600;
  font-size: 13px;
  letter-spacing: -0.01em;
}
.sidebar-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
}
.empty-hint {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  padding: 8px 0;
  opacity: 0.8;
}

/* ====== 区段标题 ====== */
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 8px 4px;
  margin-top: 6px;
  font-size: 11px;
  font-weight: 500;
  color: var(--n-text-color-3, #999);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  user-select: none;
}
.section-header .add-btn {
  opacity: 0.7;
}
.section-header .add-btn:hover {
  opacity: 1;
}

.custom-empty {
  font-size: 11px;
  color: var(--n-text-color-3, #aaa);
  padding: 4px 10px 6px;
  opacity: 0.75;
}

/* ====== 树形列表 ====== */
.tag-tree {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.15s ease;
  user-select: none;
}
.tree-item:hover {
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
}
.tree-item.active {
  background: var(--n-color-primary, #18a058);
  color: #fff;
}

.tree-item.child-item {
  padding-left: 26px;
  font-size: 12px;
  opacity: 0.85;
}
.tree-item.child-item.active {
  opacity: 1;
}

.tree-item.uncategorized-item {
  font-style: italic;
  opacity: 0.7;
}
.tree-item.uncategorized-item.active {
  font-style: normal;
  opacity: 1;
}

.all-item {
  font-weight: 500;
  margin-bottom: 4px;
}

/* ====== 自定义标签项 ====== */
.tree-item.custom-item .custom-actions {
  display: none;
  flex: none;
  align-items: center;
  gap: 2px;
  margin-left: 4px;
}
.tree-item.custom-item:hover .custom-actions {
  display: inline-flex;
}
.tree-item.custom-item:hover .item-count {
  display: none;
}
.tree-item.custom-item.active .custom-actions :deep(.n-button) {
  color: rgba(255, 255, 255, 0.85);
}

.expand-btn {
  flex: none;
  width: 14px;
  font-size: 9px;
  text-align: center;
  color: var(--n-text-color-3, #888);
  cursor: pointer;
  user-select: none;
}
.expand-btn.placeholder {
  cursor: default;
  opacity: 0.4;
}
.primary-marker {
  flex: none;
  width: 14px;
  font-size: 8px;
  text-align: center;
  color: var(--n-color-primary, #18a058);
  user-select: none;
  opacity: 0.7;
}
.tree-item.active .expand-btn {
  color: rgba(255, 255, 255, 0.7);
}
.tree-item.active .primary-marker {
  color: rgba(255, 255, 255, 0.85);
  opacity: 1;
}

.item-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-count {
  flex: none;
  font-size: 11px;
  opacity: 0.6;
}
.tree-item.active .item-count {
  opacity: 0.85;
}

.hint-text {
  margin-top: 6px;
  font-size: 12px;
  color: var(--n-text-color-3, #888);
  line-height: 1.5;
}
.hint-text code {
  background: var(--n-action-color, rgba(0, 0, 0, 0.05));
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
}

/* 深色主题适配 */
:global(html[data-theme="dark"]) .tree-item:hover {
  background: rgba(255, 255, 255, 0.06);
}
</style>
