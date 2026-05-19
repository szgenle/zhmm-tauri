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
  count: number;
  checked: boolean;
}

const tagList = computed<TagInfo[]>(() => {
  const counter = new Map<string, number>();
  for (const entry of props.entries) {
    if (!entry.tags) continue;
    for (const t of entry.tags) {
      if (t) counter.set(t, (counter.get(t) || 0) + 1);
    }
  }
  // 按频次倒序 + 字母序稳定
  const sorted = [...counter.entries()].sort((a, b) => {
    if (b[1] !== a[1]) return b[1] - a[1];
    return a[0].localeCompare(b[0]);
  });
  const selected = new Set(props.selectedTags);
  return sorted.map(([tag, count]) => ({
    tag,
    count,
    checked: selected.has(tag),
  }));
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
      <div
        v-for="item in tagList"
        :key="item.tag"
        class="tag-item"
        @click="toggle(item.tag)"
      >
        <n-checkbox :checked="item.checked" @update:checked="toggle(item.tag)" />
        <span class="tag-label">#{{ item.tag }}</span>
        <span class="tag-count">({{ item.count }})</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tag-sidebar {
  width: 200px;
  min-width: 200px;
  border-right: 1px solid var(--n-border-color, #e0e0e0);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: width 0.2s, min-width 0.2s;
}
.tag-sidebar.collapsed {
  width: 36px;
  min-width: 36px;
}
.sidebar-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 10px 6px;
  font-weight: bold;
  font-size: 13px;
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
}
.tag-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 2px;
  cursor: pointer;
  border-radius: 4px;
}
.tag-item:hover {
  background: var(--n-color-hover, rgba(0,0,0,0.04));
}
.tag-label {
  font-size: 13px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tag-count {
  font-size: 11px;
  color: var(--n-text-color-3, #999);
}
</style>
