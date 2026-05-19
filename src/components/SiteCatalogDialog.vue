<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useMessage } from "naive-ui";
import { api, type SiteCatalogEntry } from "../api";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const message = useMessage();

const allEntries = ref<SiteCatalogEntry[]>([]);
const searchQuery = ref("");
const loading = ref(false);

// 防抖搜索
let searchTimer: ReturnType<typeof setTimeout> | null = null;
const debouncedQuery = ref("");

watch(searchQuery, (val) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    debouncedQuery.value = val.trim().toLowerCase();
  }, 250);
});

const filtered = computed(() => {
  const q = debouncedQuery.value;
  if (!q) return allEntries.value;
  return allEntries.value.filter((entry) => {
    if (entry.host.toLowerCase().includes(q)) return true;
    if (entry.name.toLowerCase().includes(q)) return true;
    if (entry.tags.some((t) => t.toLowerCase().includes(q))) return true;
    return false;
  });
});

const countSummary = computed(() => {
  const total = allEntries.value.length;
  const shown = filtered.value.length;
  if (total === shown) return `共 ${total} 条`;
  return `共 ${total} 条（显示 ${shown} 条）`;
});

async function loadCatalog() {
  loading.value = true;
  try {
    allEntries.value = await api.listSiteCatalog();
  } catch (e: any) {
    message.error(`加载站点词典失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

// 只在首次 show 时加载
watch(
  () => props.show,
  (val) => {
    if (val && allEntries.value.length === 0) {
      loadCatalog();
    }
  }
);
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="网站词典"
    style="width: 680px; max-height: 85vh"
    @update:show="(v) => emit('update:show', v)"
  >
    <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
      内置离线站点词典，可根据网址自动建议名称和标签。仅用于辅助填写，不联网。
    </n-text>

    <n-space justify="space-between" align="center" style="margin-bottom: 12px">
      <n-input
        v-model:value="searchQuery"
        placeholder="搜索域名 / 名称 / 标签"
        clearable
        style="width: 320px"
      />
      <n-text depth="3" style="font-size: 12px">{{ countSummary }}</n-text>
    </n-space>

    <n-spin :show="loading">
      <div
        v-if="!loading && allEntries.length === 0"
        style="text-align: center; padding: 40px 0; color: #888"
      >
        词典为空
      </div>
      <div
        v-else-if="!loading && filtered.length === 0"
        style="text-align: center; padding: 24px 0; color: #888"
      >
        无匹配结果
      </div>
      <n-data-table
        v-else
        :columns="[
          { title: '域名', key: 'host', width: 200 },
          { title: '名称', key: 'name', width: 150 },
          {
            title: '标签',
            key: 'tags',
            render: (row: any) => row.tags?.join('、') || '',
          },
        ]"
        :data="filtered"
        :bordered="false"
        :max-height="400"
        :pagination="{ pageSize: 50 }"
        virtual-scroll
        size="small"
      />
    </n-spin>

    <template #footer>
      <n-space justify="end">
        <n-button size="small" @click="emit('update:show', false)">关闭</n-button>
      </n-space>
    </template>
  </n-modal>
</template>
