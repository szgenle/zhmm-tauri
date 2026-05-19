<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useMessage, useDialog } from "naive-ui";
import { api, type BackupInfo } from "../api";

const message = useMessage();
const dialog = useDialog();

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
}>();

const backups = ref<BackupInfo[]>([]);
const loading = ref(false);

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

const totalSize = computed(() => {
  const total = backups.value.reduce((sum, b) => sum + b.size, 0);
  return formatSize(total);
});

async function loadBackups() {
  loading.value = true;
  try {
    backups.value = await api.listLocalBackups();
  } catch (e: any) {
    message.error(`加载备份列表失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

async function handleCreate() {
  loading.value = true;
  try {
    const name = await api.createLocalBackup();
    message.success(`已创建备份: ${name}`);
    await loadBackups();
  } catch (e: any) {
    message.error(`创建备份失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

function handleRestore(item: BackupInfo) {
  dialog.warning({
    title: "确认恢复",
    content: `确定要恢复备份"${item.name}"吗？\n\n当前数据将被完全覆盖，此操作不可撤销！`,
    positiveText: "恢复",
    negativeText: "取消",
    onPositiveClick: async () => {
      loading.value = true;
      try {
        await api.restoreLocalBackup(item.name);
        message.success("已从备份恢复");
      } catch (e: any) {
        message.error(`恢复失败: ${e}`);
      } finally {
        loading.value = false;
      }
    },
  });
}

function handleDelete(item: BackupInfo) {
  dialog.warning({
    title: "确认删除",
    content: `确定要删除备份"${item.name}"吗？此操作不可恢复。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.deleteLocalBackup(item.name);
        message.success("已删除");
        await loadBackups();
      } catch (e: any) {
        message.error(`删除失败: ${e}`);
      }
    },
  });
}

async function handleCleanup() {
  dialog.info({
    title: "清理备份",
    content: "将保留最新的 5 个备份，删除其余的。继续？",
    positiveText: "确定",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        const removed = await api.cleanupBackups(5);
        if (removed > 0) {
          message.success(`已清理 ${removed} 个旧备份`);
          await loadBackups();
        } else {
          message.info("无需清理");
        }
      } catch (e: any) {
        message.error(`清理失败: ${e}`);
      }
    },
  });
}

onMounted(loadBackups);
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="备份管理"
    style="width: 640px; max-height: 80vh"
    @update:show="emit('update:show', $event)"
  >
    <n-space vertical size="medium">
      <n-space justify="space-between" align="center">
        <n-space>
          <n-button type="primary" :loading="loading" @click="handleCreate">
            创建备份
          </n-button>
          <n-button :disabled="backups.length === 0" @click="handleCleanup">
            清理旧备份
          </n-button>
        </n-space>
        <n-text depth="3" style="font-size: 12px">
          共 {{ backups.length }} 个备份，总大小 {{ totalSize }}
        </n-text>
      </n-space>

      <n-text depth="3" style="font-size: 12px">
        本地备份使用主密码加密，存储在应用数据目录中。恢复时需要当前已解锁的主密码。
      </n-text>

      <n-empty v-if="!backups.length && !loading" description="暂无备份" />

      <n-list v-else bordered hoverable>
        <n-list-item v-for="item in backups" :key="item.name">
          <n-space justify="space-between" align="center" style="width: 100%">
            <div>
              <div style="font-weight: 500">{{ item.name }}</div>
              <n-space size="small" style="margin-top: 4px">
                <n-text depth="3" style="font-size: 12px">
                  {{ item.created_at }}
                </n-text>
                <n-text depth="3" style="font-size: 12px">
                  {{ formatSize(item.size) }}
                </n-text>
              </n-space>
            </div>
            <n-space size="small">
              <n-button size="small" quaternary type="primary" @click="handleRestore(item)">
                恢复
              </n-button>
              <n-button size="small" quaternary type="error" @click="handleDelete(item)">
                删除
              </n-button>
            </n-space>
          </n-space>
        </n-list-item>
      </n-list>
    </n-space>
  </n-modal>
</template>
