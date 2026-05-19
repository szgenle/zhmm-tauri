<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useMessage, useDialog } from "naive-ui";
import { api, type TagCount } from "../api";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "changed"): void;
}>();

const message = useMessage();
const dialog = useDialog();

const tags = ref<TagCount[]>([]);
const loading = ref(false);
const selectedTag = ref<string | null>(null);
const hasChanges = ref(false);

const isEmpty = computed(() => tags.value.length === 0);

async function loadTags() {
  loading.value = true;
  try {
    tags.value = await api.collectTagCounts();
  } catch (e: any) {
    message.error(`加载标签失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

function handleClose() {
  if (hasChanges.value) {
    emit("changed");
  }
  emit("update:show", false);
}

async function handleRename() {
  if (!selectedTag.value) return;
  const old = selectedTag.value;

  const newName = window.prompt(`将"${old}"重命名为:`, old);
  if (!newName || newName.trim() === "" || newName.trim() === old) return;

  try {
    const affected = await api.renameTag(old, newName.trim());
    if (affected > 0) {
      hasChanges.value = true;
      message.success(`已将"${old}"重命名为"${newName.trim()}"，影响 ${affected} 条记录`);
    } else {
      message.info("无变更");
    }
    await loadTags();
    selectedTag.value = newName.trim();
  } catch (e: any) {
    message.error(`重命名失败: ${e}`);
  }
}

async function handleDelete() {
  if (!selectedTag.value) return;
  const tag = selectedTag.value;
  const count = tags.value.find((t) => t.tag === tag)?.count || 0;

  dialog.warning({
    title: "确认删除",
    content: `确定要删除标签"${tag}"吗？将从 ${count} 条记录中移除。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        const affected = await api.deleteTag(tag);
        if (affected > 0) {
          hasChanges.value = true;
          message.success(`已删除标签"${tag}"，影响 ${affected} 条记录`);
        }
        selectedTag.value = null;
        await loadTags();
      } catch (e: any) {
        message.error(`删除失败: ${e}`);
      }
    },
  });
}

onMounted(loadTags);
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="标签管理"
    style="width: 420px; max-height: 80vh"
    @update:show="(v) => emit('update:show', v)"
    @after-leave="handleClose"
  >
    <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
      管理所有已使用的标签。支持重命名（合并到已有标签）和删除操作。
    </n-text>

    <n-spin :show="loading">
      <div v-if="isEmpty && !loading" style="text-align: center; padding: 40px 0; color: #888">
        暂无标签
      </div>
      <n-list v-else hoverable clickable style="max-height: 360px; overflow: auto">
        <n-list-item
          v-for="item in tags"
          :key="item.tag"
          :class="{ 'tag-selected': selectedTag === item.tag }"
          @click="selectedTag = item.tag"
        >
          <n-space justify="space-between" align="center" style="width: 100%">
            <n-tag round>{{ item.tag }}</n-tag>
            <n-text depth="3" style="font-size: 12px">{{ item.count }} 条</n-text>
          </n-space>
        </n-list-item>
      </n-list>
    </n-spin>

    <template #footer>
      <n-space justify="space-between">
        <n-space>
          <n-button :disabled="!selectedTag" size="small" @click="handleRename">
            重命名
          </n-button>
          <n-button
            :disabled="!selectedTag"
            size="small"
            type="error"
            @click="handleDelete"
          >
            删除
          </n-button>
        </n-space>
        <n-button size="small" @click="handleClose">关闭</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.tag-selected {
  background: var(--n-item-color-active, rgba(24, 160, 88, 0.1));
}
</style>
