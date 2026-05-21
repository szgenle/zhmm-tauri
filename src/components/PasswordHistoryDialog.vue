<script setup lang="ts">
import { ref, watch } from "vue";
import { useMessage, useDialog } from "naive-ui";
import { api, type PasswordHistoryItem } from "../api";
import { copyAndScheduleClear } from "../settings";

const props = defineProps<{
  show: boolean;
  title: string;
  entryId: string;
}>();

const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "rolledBack"): void;
}>();

const message = useMessage();
const dialog = useDialog();

const historyItems = ref<PasswordHistoryItem[]>([]);
const historyRevealed = ref<Set<number>>(new Set());

// 当对话框打开时加载历史
watch(
  () => props.show,
  async (visible) => {
    if (!visible || !props.entryId) return;
    historyRevealed.value = new Set();
    try {
      historyItems.value = await api.getPasswordHistory(props.entryId);
    } catch (e: any) {
      message.error(`加载历史失败: ${e}`);
    }
  }
);

function toggleHistoryReveal(idx: number) {
  const s = new Set(historyRevealed.value);
  if (s.has(idx)) {
    s.delete(idx);
  } else {
    s.add(idx);
  }
  historyRevealed.value = s;
}

async function copyHistory(item: PasswordHistoryItem) {
  await copyAndScheduleClear(item.pwd);
  message.success("已复制历史密码");
}

function rollbackHistory(idx: number) {
  dialog.warning({
    title: "确认恢复",
    content: "将该历史密码恢复为当前密码？\n当前密码会被压回历史栈顶，可再次回滚。",
    positiveText: "恢复",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.rollbackPassword(props.entryId, idx);
        message.success("密码已回滚到历史版本");
        emit("update:show", false);
        emit("rolledBack");
      } catch (e: any) {
        message.error(`回滚失败: ${e}`);
      }
    },
  });
}
</script>

<template>
  <n-modal
    :show="show"
    preset="card"
    :title="`历史密码 - ${title}`"
    style="width: 560px"
    @update:show="emit('update:show', $event)"
  >
    <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 12px">
      以下为该条目的历史密码版本（最新在前，最多 5 条）。
    </n-text>
    <n-empty v-if="!historyItems.length" description="暂无历史密码" />
    <n-list v-else>
      <n-list-item v-for="(item, idx) in historyItems" :key="idx">
        <n-space justify="space-between" align="center" style="width: 100%">
          <div>
            <div style="font-family: monospace">
              {{ historyRevealed.has(idx) ? item.pwd : '••••••••' }}
            </div>
            <div style="font-size: 12px; color: var(--n-text-color-3)">
              替换于 {{ new Date(item.replaced_at).toLocaleString() }}
            </div>
          </div>
          <n-space :size="4">
            <n-button size="small" quaternary @click="toggleHistoryReveal(idx)">
              {{ historyRevealed.has(idx) ? '隐藏' : '显示' }}
            </n-button>
            <n-button size="small" quaternary @click="copyHistory(item)">
              复制
            </n-button>
            <n-button size="small" quaternary type="warning" @click="rollbackHistory(idx)">
              恢复
            </n-button>
          </n-space>
        </n-space>
      </n-list-item>
    </n-list>
  </n-modal>
</template>
