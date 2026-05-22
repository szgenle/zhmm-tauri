<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useDialog, useMessage } from "naive-ui";
import {
  ShieldCheckmarkOutline,
  AddCircleOutline,
  FolderOpenOutline,
  TrashOutline,
  RefreshOutline,
} from "@vicons/ionicons5";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, type RecentEntry } from "../api";
import UnlockDialog from "../components/UnlockDialog.vue";
import CreateVaultDialog from "../components/CreateVaultDialog.vue";

const router = useRouter();
const message = useMessage();
const dialog = useDialog();

const recents = ref<RecentEntry[]>([]);
const loading = ref(false);
const showLegacyBanner = ref(false);

const showUnlock = ref(false);
const unlockPath = ref("");
const unlockAccount = ref<string | undefined>(undefined);
const unlockHashpw = ref<string | undefined>(undefined);

const showCreate = ref(false);

async function refresh() {
  loading.value = true;
  try {
    recents.value = await api.listRecent();
  } catch (e: any) {
    message.error(`加载最近访问失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  // 若已经处于解锁状态（例如热重载），直接进入主界面
  try {
    const status = await api.vaultStatus();
    if (status.unlocked) {
      router.push("/");
      return;
    }
  } catch {
    // 忽略
  }
  // 检测旧版 v1 vault.zmb 是否存在（不兼容提示）
  try {
    showLegacyBanner.value = await api.legacyVaultExists();
  } catch {
    showLegacyBanner.value = false;
  }
  await refresh();
});

function openRecent(item: RecentEntry) {
  unlockPath.value = item.path;
  unlockAccount.value = item.account;
  unlockHashpw.value = item.hashpw || undefined;
  showUnlock.value = true;
}

async function openExternal() {
  try {
    const selected = await openDialog({
      title: "打开账号库",
      multiple: false,
      filters: [{ name: "ZMB 账号库", extensions: ["zmb"] }],
    });
    if (typeof selected !== "string" || !selected) return;

    // 若已在最近访问列表中，复用其 account/hashpw
    const exist = recents.value.find((r) => r.path === selected);
    unlockPath.value = selected;
    unlockAccount.value = exist?.account;
    unlockHashpw.value = exist?.hashpw || undefined;
    showUnlock.value = true;
  } catch (e: any) {
    message.error(`选择文件失败: ${e}`);
  }
}

function handleUnlocked(_entry: RecentEntry) {
  router.push("/");
}

function handleCreated(_entry: RecentEntry) {
  router.push("/");
}

function handleRemove(item: RecentEntry, e: Event) {
  e.stopPropagation();
  dialog.warning({
    title: "从最近访问中移除",
    content: `仅从列表中移除 "${item.path}"，不会删除实际文件。`,
    positiveText: "移除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.removeRecent(item.path);
        await refresh();
      } catch (err: any) {
        message.error(`移除失败: ${err}`);
      }
    },
  });
}

function fileName(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}
</script>

<template>
  <n-layout class="filelist-page">
    <div class="filelist-container">
      <div class="logo">
        <n-icon size="44" :depth="3"><ShieldCheckmarkOutline /></n-icon>
        <h1>账号小本本</h1>
        <p class="subtitle">选择一个账号库以解锁，或新建/打开任意 .zmb 文件</p>
      </div>

      <n-alert
        v-if="showLegacyBanner"
        type="warning"
        title="检测到旧版数据"
        closable
        @close="showLegacyBanner = false"
        style="margin-bottom: 16px"
      >
        发现旧版本遗留的
        <n-text code>vault.zmb</n-text>
        文件（AES-GCM 单口令格式），新版本采用 SM4-GCM 双因子加密，
        <b>无法直接读取</b>。请使用旧版本启动并通过"导出 xlsx"备份数据，
        然后在新版本中"新建账号库"并通过"导入 xlsx"恢复。
      </n-alert>

      <div class="actions">
        <n-space>
          <n-button type="primary" @click="showCreate = true">
            <template #icon>
              <n-icon><AddCircleOutline /></n-icon>
            </template>
            新建账号库
          </n-button>
          <n-button @click="openExternal">
            <template #icon>
              <n-icon><FolderOpenOutline /></n-icon>
            </template>
            打开 .zmb 文件
          </n-button>
          <n-button quaternary @click="refresh" :loading="loading">
            <template #icon>
              <n-icon><RefreshOutline /></n-icon>
            </template>
            刷新
          </n-button>
        </n-space>
      </div>

      <n-divider style="margin: 16px 0">最近访问</n-divider>

      <n-spin :show="loading">
        <n-empty
          v-if="recents.length === 0 && !loading"
          description="暂无最近访问的账号库"
          style="margin: 32px 0"
        />
        <div v-else class="recent-list">
          <div
            v-for="item in recents"
            :key="item.path"
            class="recent-item"
            @click="openRecent(item)"
          >
            <div class="item-main">
              <div class="item-name">{{ fileName(item.path) }}</div>
              <div class="item-path" :title="item.path">{{ item.path }}</div>
              <div class="item-meta">
                <span class="meta-account">账号：{{ item.account || "-" }}</span>
                <span class="meta-time">{{ item.last_access_time }}</span>
              </div>
            </div>
            <div class="item-actions">
              <n-button quaternary circle size="small" @click="(e) => handleRemove(item, e)">
                <template #icon>
                  <n-icon><TrashOutline /></n-icon>
                </template>
              </n-button>
            </div>
          </div>
        </div>
      </n-spin>
    </div>

    <UnlockDialog
      v-model:show="showUnlock"
      :path="unlockPath"
      :account="unlockAccount"
      :hashpw="unlockHashpw"
      @success="handleUnlocked"
    />
    <CreateVaultDialog v-model:show="showCreate" @success="handleCreated" />
  </n-layout>
</template>

<style scoped>
.filelist-page {
  min-height: 100vh;
  width: 100vw;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 48px 24px;
  box-sizing: border-box;
}
.filelist-container {
  width: 100%;
  max-width: 720px;
}
.logo {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 24px;
}
.logo h1 {
  margin: 12px 0 4px;
  font-size: 22px;
}
.subtitle {
  color: var(--n-text-color-3);
  font-size: 13px;
  margin: 0;
}
.actions {
  display: flex;
  justify-content: center;
  margin: 16px 0;
}
.recent-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.recent-item {
  display: flex;
  align-items: center;
  padding: 12px 14px;
  border: 1px solid var(--n-border-color, #e0e0e0);
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s;
}
.recent-item:hover {
  background-color: var(--n-color-hover, rgba(0, 0, 0, 0.04));
  border-color: var(--n-primary-color, #18a058);
}
.item-main {
  flex: 1;
  min-width: 0;
}
.item-name {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 2px;
}
.item-path {
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}
.item-meta {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--n-text-color-2);
}
.item-actions {
  margin-left: 12px;
}
</style>
