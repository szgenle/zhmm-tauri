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

interface TagInfo {
  tag: string;
  label: string;
  count: number;
  checked: boolean;
  uncategorized?: boolean;
}

const tagList = computed<TagInfo[]>(() => {
  // 仅以每条记录的第一个标签（一级标签）为分组键聚合
  const counter = new Map<string, number>();
  let uncategorizedCount = 0;
  for (const entry of props.entries) {
    const primary = (entry.tags || []).find((t) => !!t);
    if (!primary) {
      uncategorizedCount += 1;
      continue;
    }
    counter.set(primary, (counter.get(primary) || 0) + 1);
  }
  // 按频次倒序 + 字母序稳定
  const sorted = [...counter.entries()].sort((a, b) => {
    if (b[1] !== a[1]) return b[1] - a[1];
    return a[0].localeCompare(b[0]);
  });
  const selected = new Set(props.selectedTags);
  const list: TagInfo[] = sorted.map(([tag, count]) => ({
    tag,
    label: tag,
    count,
    checked: selected.has(tag),
  }));
  if (uncategorizedCount > 0) {
    list.push({
      tag: UNCATEGORIZED_TAG,
      label: "未分类",
      count: uncategorizedCount,
      checked: selected.has(UNCATEGORIZED_TAG),
      uncategorized: true,
    });
  }
  return list;
});

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
      <div v-if="!tagList.length" class="empty-hint">
        暂无标签。<br />编辑条目时添加标签即可在此筛选。
      </div>
      <div v-else class="tag-chips-wrap">
        <span
          class="tag-chip"
          :class="{ active: props.selectedTags.length === 0 }"
          @click="clearSelection"
        >
          全部
          <span class="chip-count">{{ props.entries.length }}</span>
        </span>
        <span
          v-for="item in tagList"
          :key="item.tag"
          class="tag-chip"
          :class="{ active: item.checked, uncategorized: item.uncategorized }"
          @click="toggle(item.tag)"
        >
          {{ item.label }}
          <span class="chip-count">{{ item.count }}</span>
        </span>
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
  padding: 4px 10px;
}
.empty-hint {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  padding: 8px 0;
  opacity: 0.8;
}
.tag-chips-wrap {
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
.tag-chip.uncategorized {
  font-style: italic;
  color: var(--n-text-color-3, #888);
}
.tag-chip.uncategorized.active {
  font-style: normal;
  color: #fff;
}

/* 深色主题适配 */
:global(html[data-theme="dark"]) .tag-chip {
  background: rgba(255, 255, 255, 0.05);
}
:global(html[data-theme="dark"]) .tag-chip:hover {
  background: rgba(255, 255, 255, 0.10);
}
</style>
