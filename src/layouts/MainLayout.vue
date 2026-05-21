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
    router.push("/login");
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
      router.push("/login");
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
    <n-layout-header bordered class="header">
      <n-tabs :value="tabValue" type="line" @update:value="tabValue = $event">
        <n-tab name="passwords">账号管理</n-tab>
        <n-tab name="data-management">数据管理</n-tab>
        <n-tab name="settings">系统设置</n-tab>
      </n-tabs>
      <n-button quaternary size="small" @click="handleLock">
        <template #icon>
          <n-icon><LockClosedOutline /></n-icon>
        </template>
        锁定
      </n-button>
    </n-layout-header>
    <n-layout-content content-style="padding: 16px;">
      <router-view />
    </n-layout-content>
  </n-layout>
</template>

<style scoped>
.header {
  height: 48px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
</style>
