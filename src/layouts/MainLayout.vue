<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { LockClosedOutline } from "@vicons/ionicons5";
import { api } from "../api";
import { settings } from "../settings";

const route = useRoute();
const router = useRouter();
const message = useMessage();

const tabValue = computed<string>({
  get: () => (route.name as string) ?? "passwords",
  set: (val: string) => {
    const map: Record<string, string> = {
      passwords: "/",
      "data-management": "/data-management",
      settings: "/settings",
    };
    router.push(map[val] ?? "/");
  },
});

async function handleLock() {
  try {
    await api.lockVault();
    router.push("/files");
  } catch (e: any) {
    message.error(`锁定失败: ${e}`);
  }
}

// 空闲自动锁定
let idleTimer: number | null = null;
function resetIdle() {
  if (idleTimer != null) window.clearTimeout(idleTimer);
  const minutes = settings.auto_lock_minutes;
  if (!minutes || minutes <= 0) return;
  idleTimer = window.setTimeout(async () => {
    try {
      await api.lockVault();
      router.push("/files");
    } catch {
      // 已锁定或路由切换下静默
    }
  }, minutes * 60 * 1000);
}
const idleEvents: (keyof DocumentEventMap)[] = [
  "mousemove",
  "mousedown",
  "keydown",
  "wheel",
  "touchstart",
];
function handleVisibility() {
  if (document.visibilityState === "visible") resetIdle();
}
onMounted(() => {
  resetIdle();
  for (const ev of idleEvents) document.addEventListener(ev, resetIdle, { passive: true });
  document.addEventListener("visibilitychange", handleVisibility);
});
onUnmounted(() => {
  if (idleTimer != null) window.clearTimeout(idleTimer);
  for (const ev of idleEvents) document.removeEventListener(ev, resetIdle);
  document.removeEventListener("visibilitychange", handleVisibility);
});
</script>

<template>
  <n-layout style="height: 100vh">
    <n-layout-header class="header">
      <div class="header-left">
        <span class="brand">账号小本本</span>
        <n-tabs :value="tabValue" type="line" @update:value="tabValue = $event" class="nav-tabs">
          <n-tab name="passwords">账号管理</n-tab>
          <n-tab name="data-management">数据管理</n-tab>
          <n-tab name="settings">系统设置</n-tab>
        </n-tabs>
      </div>
      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button quaternary circle size="small" @click="handleLock">
            <template #icon>
              <n-icon :size="18"><LockClosedOutline /></n-icon>
            </template>
          </n-button>
        </template>
        锁定密码库
      </n-tooltip>
    </n-layout-header>
    <n-layout-content content-style="padding: 20px 24px;">
      <router-view />
    </n-layout-content>
  </n-layout>
</template>

<style scoped>
.header {
  height: 52px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--app-header-bg);
  backdrop-filter: blur(var(--app-header-blur));
  -webkit-backdrop-filter: blur(var(--app-header-blur));
  box-shadow: var(--app-header-shadow);
  border-bottom: 1px solid var(--app-border-color);
  position: sticky;
  top: 0;
  z-index: 100;
  transition: background 0.3s ease, box-shadow 0.3s ease;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.brand {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.01em;
  opacity: 0.85;
  white-space: nowrap;
}

.nav-tabs :deep(.n-tabs-tab) {
  font-weight: 500;
  transition: color 0.2s ease, opacity 0.2s ease;
}

.nav-tabs :deep(.n-tabs-tab--active) {
  font-weight: 600;
}
</style>
