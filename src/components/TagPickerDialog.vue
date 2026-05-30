<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import { api } from "../api";

const props = defineProps<{
  show: boolean;
  /** 当前条目已选中的标签 */
  selectedTags: string[];
  /** 用于按账号库隔离最近使用列表的 key（通常是 vault path） */
  vaultKey: string;
}>();

const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "update:selectedTags", v: string[]): void;
}>();

const message = useMessage();

interface TagItem {
  tag: string;
  count: number;
  recent: boolean;
}

const allTags = ref<{ tag: string; count: number }[]>([]);
const recentList = ref<string[]>([]);
const keyword = ref("");
const newTagInput = ref("");
const loading = ref(false);

function recentStorageKey(): string {
  // 空 key 时退化为全局，避免崩溃
  const k = props.vaultKey || "__default__";
  return `zhmm:recent-tags:${k}`;
}

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(recentStorageKey());
    if (!raw) return [];
    const arr = JSON.parse(raw);
    if (!Array.isArray(arr)) return [];
    return arr.filter((x: unknown): x is string => typeof x === "string");
  } catch {
    return [];
  }
}

async function refresh() {
  loading.value = true;
  try {
    const counts = await api.collectTagCounts();
    allTags.value = counts.map((c) => ({ tag: c.tag, count: c.count }));
    recentList.value = loadRecent();
  } catch (e: any) {
    message.error(`加载标签失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.show,
  (v: boolean) => {
    if (v) {
      keyword.value = "";
      newTagInput.value = "";
      refresh();
    }
  }
);

/** 排序：最近使用 > 频次 > 字母 */
const orderedTags = computed<TagItem[]>(() => {
  const map = new Map<string, number>();
  for (const t of allTags.value) map.set(t.tag, t.count);
  const recentOrder = new Map<string, number>();
  recentList.value.forEach((t: string, i: number) => recentOrder.set(t, i));

  const all = new Set<string>([...map.keys(), ...recentList.value]);
  const items: TagItem[] = [...all].map((tag) => ({
    tag,
    count: map.get(tag) ?? 0,
    recent: recentOrder.has(tag),
  }));
  items.sort((a, b) => {
    const ai = recentOrder.has(a.tag) ? recentOrder.get(a.tag)! : Infinity;
    const bi = recentOrder.has(b.tag) ? recentOrder.get(b.tag)! : Infinity;
    if (ai !== bi) return ai - bi;
    if (b.count !== a.count) return b.count - a.count;
    return a.tag.localeCompare(b.tag);
  });
  return items;
});

const filteredTags = computed<TagItem[]>(() => {
  const kw = keyword.value.trim().toLowerCase();
  if (!kw) return orderedTags.value;
  return orderedTags.value.filter((t: TagItem) => t.tag.toLowerCase().includes(kw));
});

const selectedSet = computed(() => new Set(props.selectedTags));

function toggle(tag: string) {
  const set = new Set(props.selectedTags);
  if (set.has(tag)) {
    set.delete(tag);
  } else {
    set.add(tag);
  }
  emit("update:selectedTags", [...set]);
}

function addCustomTag() {
  const t = newTagInput.value.trim();
  if (!t) return;
  if (!selectedSet.value.has(t)) {
    emit("update:selectedTags", [...props.selectedTags, t]);
  }
  newTagInput.value = "";
}
</script>

<template>
  <n-modal
    :show="show"
    preset="card"
    title="选择标签"
    style="width: 480px"
    @update:show="emit('update:show', $event)"
  >
    <div class="picker-toolbar">
      <n-input
        v-model:value="keyword"
        placeholder="搜索标签…"
        clearable
        size="small"
        style="flex: 1"
      />
      <n-tag size="small" :bordered="false" type="info">
        已选 {{ selectedTags.length }}
      </n-tag>
    </div>

    <n-spin :show="loading">
      <div class="picker-body">
        <div v-if="!filteredTags.length" class="empty-hint">
          {{ keyword ? "没有匹配的标签" : "暂无标签，先在下方输入新建一个吧" }}
        </div>
        <div
          v-for="item in filteredTags"
          :key="item.tag"
          class="tag-row"
          :class="{ selected: selectedSet.has(item.tag) }"
          @click="toggle(item.tag)"
        >
          <span class="tag-name">#{{ item.tag }}</span>
          <span v-if="item.recent" class="badge badge-recent">最近</span>
          <span v-if="item.count > 0" class="badge badge-count">{{ item.count }}</span>
          <span class="state">
            {{ selectedSet.has(item.tag) ? "✓" : "" }}
          </span>
        </div>
      </div>
    </n-spin>

    <div class="picker-new">
      <n-input
        v-model:value="newTagInput"
        placeholder="输入新标签名按回车添加"
        size="small"
        @keyup.enter="addCustomTag"
        style="flex: 1"
      />
      <n-button size="small" @click="addCustomTag">添加</n-button>
    </div>

    <template #footer>
      <n-space justify="end">
        <n-button type="primary" @click="emit('update:show', false)">完成</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.picker-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.picker-body {
  max-height: 360px;
  min-height: 120px;
  overflow-y: auto;
  padding: 2px;
}
.empty-hint {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  padding: 24px 0;
  text-align: center;
  opacity: 0.8;
}
.tag-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.tag-row:hover {
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
}
.tag-row.selected {
  background: var(--n-color-pressed, rgba(24, 160, 88, 0.12));
}
.tag-name {
  flex: 1;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.badge {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 8px;
  line-height: 1.4;
}
.badge-recent {
  background: rgba(24, 160, 88, 0.15);
  color: #18a058;
}
.badge-count {
  background: rgba(0, 0, 0, 0.06);
  color: var(--n-text-color-3, #888);
}
.state {
  width: 14px;
  text-align: right;
  color: #18a058;
  font-weight: 600;
}
.picker-new {
  display: flex;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--app-border-color, #e0e0e6);
}
</style>
