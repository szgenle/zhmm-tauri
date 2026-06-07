<script lang="ts">
export const UNCATEGORIZED_TAG = "__uncategorized__";
</script>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { PasswordSummary } from "../api";

const props = defineProps<{
  entries: PasswordSummary[];
  selectedTags: string[];
}>();

const emit = defineEmits<{
  (e: "update:selectedTags", value: string[]): void;
}>();

/** 子标签最低出现次数阈值 */
const CHILD_MIN_COUNT = 5;

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

  // 按频次倒序 + 字母序
  const sorted = [...primaryCounter.entries()].sort((a, b) => {
    if (b[1] !== a[1]) return b[1] - a[1];
    return a[0].localeCompare(b[0]);
  });

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

function toggle(tag: string) {
  const set = new Set(props.selectedTags);
  if (set.has(tag)) {
    set.delete(tag);
  } else {
    set.add(tag);
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
      <div v-if="!tagTree.nodes.length && tagTree.uncategorizedCount === 0" class="empty-hint">
        暂无标签。<br />编辑条目时添加标签即可在此筛选。
      </div>
      <div v-else class="tag-tree">
        <!-- 全部 -->
        <div
          class="tree-item all-item"
          :class="{ active: props.selectedTags.length === 0 }"
          @click="clearSelection"
        >
          <span class="item-label">全部</span>
          <span class="item-count">{{ props.entries.length }}</span>
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

/* 深色主题适配 */
:global(html[data-theme="dark"]) .tree-item:hover {
  background: rgba(255, 255, 255, 0.06);
}
</style>
