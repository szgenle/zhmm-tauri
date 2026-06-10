<script lang="ts">
import type { PasswordSummary } from "../api";

export const UNCATEGORIZED_TAG = "__uncategorized__";

/** 子标签最低出现次数阈值（< 阈值的二级标签会聚合到「其它」虚拟子标签） */
export const CHILD_MIN_COUNT = 5;

/** 「其它」虚拟子标签 sentinel 前缀，后接所属一级标签 */
const OTHER_SUB_PREFIX = "__other_sub__::";

export function otherSubSentinel(primary: string): string {
  return OTHER_SUB_PREFIX + primary;
}

export function isOtherSubSentinel(tag: string): boolean {
  return typeof tag === "string" && tag.startsWith(OTHER_SUB_PREFIX);
}

export function primaryOfOtherSub(sentinel: string): string {
  return sentinel.slice(OTHER_SUB_PREFIX.length);
}

/** 计算每个一级标签下「频次 ≥ 阈值」的可见二级标签集合
 *
 * - 当传入 primarySet 时：以「条目 tags ∩ primarySet」决定该条目归属的一级标签，
 *   除该一级外的其它 tags 作为子标签候选；一个条目可同时归属多个一级。
 * - 未传入 primarySet 时：退化为旧行为（tags[0] 作为一级，tags[1:] 作为子标签）。
 */
export function computeVisibleChildrenMap(
  entries: PasswordSummary[],
  threshold: number = CHILD_MIN_COUNT,
  primarySet?: Set<string>,
): Map<string, Set<string>> {
  const counter = new Map<string, Map<string, number>>();
  const useConfig = !!primarySet && primarySet.size > 0;
  for (const entry of entries) {
    const tags = (entry.tags || []).filter((t) => !!t);
    if (tags.length < 2) continue;
    if (useConfig) {
      const myPrimaries = tags.filter((t) => primarySet!.has(t));
      for (const primary of myPrimaries) {
        if (!counter.has(primary)) counter.set(primary, new Map());
        const cm = counter.get(primary)!;
        for (const t of tags) {
          if (!t || t === primary) continue;
          cm.set(t, (cm.get(t) || 0) + 1);
        }
      }
    } else {
      const primary = tags[0];
      if (!counter.has(primary)) counter.set(primary, new Map());
      const cm = counter.get(primary)!;
      for (let i = 1; i < tags.length; i++) {
        const c = tags[i];
        if (!c) continue;
        cm.set(c, (cm.get(c) || 0) + 1);
      }
    }
  }
  const result = new Map<string, Set<string>>();
  for (const [primary, cm] of counter) {
    const visible = new Set<string>();
    for (const [child, count] of cm) {
      if (count >= threshold) visible.add(child);
    }
    result.set(primary, visible);
  }
  return result;
}

/** 判断条目是否属于某「其它」虚拟子标签
 *
 * - 当传入 primarySet 时：tags 含 primary，且条目其它 tags 均不在 visibleSet
 * - 未传入 primarySet 时：tags[0]=primary 且 tags[1:] 均不在 visibleSet
 */
export function entryMatchesOtherSub(
  entry: PasswordSummary,
  sentinel: string,
  visibleMap: Map<string, Set<string>>,
  primarySet?: Set<string>,
): boolean {
  if (!isOtherSubSentinel(sentinel)) return false;
  const primary = primaryOfOtherSub(sentinel);
  const tags = (entry.tags || []).filter((t) => !!t);
  if (tags.length === 0) return false;
  const useConfig = !!primarySet && primarySet.size > 0;
  if (useConfig) {
    if (!tags.includes(primary)) return false;
  } else {
    if (tags[0] !== primary) return false;
  }
  const visible = visibleMap.get(primary);
  if (!visible || visible.size === 0) return true;
  for (const t of tags) {
    if (t === primary) continue;
    if (visible.has(t)) return false;
  }
  return true;
}
</script>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { AddOutline, CreateOutline, TrashOutline } from "@vicons/ionicons5";
import { useDialog } from "naive-ui";
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
  /** 用户在「标签词典」勾选的一级标签集合；非空时按交集分级，空则退化为按 tags[0] 分级 */
  primaryTags?: string[];
}>(), {
  mode: "single",
  sortMode: "frequency",
  primaryTags: () => [],
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
  /** 标记为「其它」虚拟子标签，用于样式区分 */
  isOther?: boolean;
}

// 展开状态缓存
const expandedMap = ref<Record<string, boolean>>({});

function toggleExpand(tag: string) {
  expandedMap.value[tag] = !(expandedMap.value[tag] ?? true);
}

const tagTree = computed<{ nodes: TagNode[]; uncategorizedCount: number }>(() => {
  // 用户在「标签词典」勾选的一级标签集合；非空时按集合分级，空则退化为按 tags[0]
  const primarySet = new Set(props.primaryTags || []);
  const useConfig = primarySet.size > 0;

  // 一级标签条目计数
  const primaryCounter = new Map<string, number>();
  // 每个一级标签下的子标签条目计数
  const childrenCounter = new Map<string, Map<string, number>>();
  // 没有命中任何一级标签的条目数（含 tags 为空、tags 完全不与 primarySet 相交）
  let uncategorizedCount = 0;

  for (const entry of props.entries) {
    const tags = (entry.tags || []).filter((t) => !!t);
    if (tags.length === 0) {
      uncategorizedCount += 1;
      continue;
    }
    if (useConfig) {
      const myPrimaries = tags.filter((t) => primarySet.has(t));
      if (myPrimaries.length === 0) {
        uncategorizedCount += 1;
        continue;
      }
      // 一个条目可同时归属多个一级 chip，每个 chip 都计 +1
      for (const primary of myPrimaries) {
        primaryCounter.set(primary, (primaryCounter.get(primary) || 0) + 1);
        if (!childrenCounter.has(primary)) childrenCounter.set(primary, new Map());
        const cm = childrenCounter.get(primary)!;
        for (const t of tags) {
          if (!t || t === primary) continue;
          cm.set(t, (cm.get(t) || 0) + 1);
        }
      }
    } else {
      // 未配置一级标签：不分级，所有出现过的 tag 平铺为独立 chip（无 children）
      // 一个条目的每个 tag 都计 +1，可同时贡献给多个 chip
      for (const t of tags) {
        primaryCounter.set(t, (primaryCounter.get(t) || 0) + 1);
      }
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
      // 1) 频次 ≥ 阈值的子标签独立显示
      const visibleEntries = [...cm.entries()].filter(([, c]) => c >= CHILD_MIN_COUNT);
      const visibleSet = new Set(visibleEntries.map(([t]) => t));
      children = visibleEntries
        .sort((a, b) => b[1] - a[1])
        .map(([childTag, childCount]) => ({
          tag: childTag,
          label: childTag,
          count: childCount,
          checked: selected.has(childTag),
        }));

      // 2) 计算「其它」虚拟子标签的条目数
      //    - useConfig：tags 含 tag 且其它 tags 均不在 visibleSet
      //    - 旧行为：  tags[0]==tag 且 tags[1:] 均不在 visibleSet
      let otherCount = 0;
      for (const entry of props.entries) {
        const ts = (entry.tags || []).filter((t) => !!t);
        if (ts.length === 0) continue;
        if (useConfig) {
          if (!ts.includes(tag)) continue;
        } else {
          if (ts[0] !== tag) continue;
        }
        let hit = false;
        for (const t of ts) {
          if (t === tag) continue;
          if (visibleSet.has(t)) { hit = true; break; }
        }
        if (!hit) otherCount += 1;
      }
      if (otherCount > 0) {
        const sentinel = otherSubSentinel(tag);
        children.push({
          tag: sentinel,
          label: "其它",
          count: otherCount,
          checked: selected.has(sentinel),
          isOther: true,
        });
      }
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
              :class="{ active: child.checked, 'other-sub': child.isOther }"
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

/* 「其它」虚拟子标签：弱化为斜体灰字，与真实子标签区分 */
.tree-item.child-item.other-sub {
  font-style: italic;
  opacity: 0.6;
}
.tree-item.child-item.other-sub.active {
  font-style: normal;
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
