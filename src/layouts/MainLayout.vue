<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { LockClosedOutline } from "@vicons/ionicons5";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "../api";
import { settings } from "../settings";

const route = useRoute();
const router = useRouter();
const message = useMessage();

// 路由切换时重置滚动偏移，防止 WKWebView 渲染残留
watch(() => route.name, () => {
  document.documentElement.scrollTop = 0;
  document.body.scrollTop = 0;
});

// 平台检测：macOS 下需要给红绿灯按钮让位
const isMacOS = ref(/Mac/i.test(navigator.platform || navigator.userAgent || ""));

// 手动触发窗口拖动：左键按下时调用，双击时最大化（macOS 标准行为）
async function onDragMouseDown(e: MouseEvent) {
  if (e.buttons !== 1) return;
  e.preventDefault();
  try {
    const win = getCurrentWindow();
    if (e.detail === 2) {
      await win.toggleMaximize();
    } else {
      await win.startDragging();
    }
  } catch {
    // 忽略：非 Tauri 环境或权限不足
  }
}

const tabValue = computed<string>({
  get: () => (route.name as string) ?? "passwords",
  set: (val: string) => {
    const map: Record<string, string> = {
      passwords: "/",
      "role-management": "/role-management",
      "template-management": "/template-management",
      "data-management": "/data-management",
      "site-nav": "/site-nav",
      "tag-dictionary": "/tag-dictionary",
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
  <div class="app-shell">
    <header class="title-bar" @mousedown="onDragMouseDown">
      <div v-if="isMacOS" class="macos-spacer"></div>
      <span class="brand">账号小本本</span>
      <div class="title-drag-fill"></div>
      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button quaternary circle size="small" class="no-drag" @mousedown.stop @click="handleLock">
            <template #icon>
              <n-icon :size="18"><LockClosedOutline /></n-icon>
            </template>
          </n-button>
        </template>
        锁定密码库
      </n-tooltip>
    </header>
    <div class="tab-bar">
      <n-tabs :value="tabValue" type="line" @update:value="tabValue = $event" class="nav-tabs">
        <n-tab name="passwords">账号管理</n-tab>
        <n-tab name="site-nav">网站导航</n-tab>
        <n-tab name="role-management">分组浏览</n-tab>
        <n-tab name="tag-dictionary">标签词典</n-tab>
        <n-tab name="template-management">模板管理</n-tab>
        <n-tab name="data-management">数据管理</n-tab>
        <n-tab name="settings">系统设置</n-tab>
      </n-tabs>
    </div>
    <main class="app-content">
      <router-view />
    </main>
  </div>
</template>

<style scoped>
/* 应用主壳：固定定位覆盖整个窗口，避免 100vh 在 macOS Overlay 模式下不等于实际窗口高度 */
.app-shell {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--app-bg);
  color: var(--n-text-color, inherit);
}

/* 顶部标题栏：整条作为窗口拖动区 */
.title-bar {
  height: 38px;
  flex: none;
  padding: 0 12px;
  display: flex;
  align-items: center;
  background: var(--app-header-bg);
  z-index: 100;
  user-select: none;
}

/* macOS 红绿灯按钮区让位（同时是拖动区） */
.macos-spacer {
  width: 78px;
  height: 100%;
  flex: none;
}

/* 中部空白拖动填充区 */
.title-drag-fill {
  flex: 1 1 auto;
  align-self: stretch;
  min-width: 16px;
}

.brand {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.01em;
  opacity: 0.7;
  white-space: nowrap;
  cursor: default;
}

/* 标签栏 */
.tab-bar {
  flex: none;
  background: var(--app-header-bg);
  padding: 0 16px;
  border-bottom: 1px solid var(--app-border-color);
  z-index: 99;
}

/* 内容区：占据剩余高度的固定容器；自身不滚动，子视图内部自行管理滚动 */
.app-content {
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
  padding: 20px 24px;
  box-sizing: border-box;
  background: var(--app-bg);
}

/* 让 n-tabs 内部所有 tab 整体靠右排列，并以基线对齐文字底部 */
.nav-tabs :deep(.n-tabs-nav-scroll-content) {
  justify-content: flex-end;
  align-items: baseline;
}

/* 隐藏 naive-ui 自带的指示条（JS 定位在 flex-end 下不准确） */
.nav-tabs :deep(.n-tabs-bar) {
  display: none !important;
}

/* 用激活标签自身的 border-bottom 作为指示线 */
.nav-tabs :deep(.n-tabs-tab--active) {
  border-bottom: 2px solid var(--n-tab-text-color-active, currentColor) !important;
}

/* 后四个标签字号小一号、整体淡化、字重更轻，弱化视觉权重 */
.nav-tabs :deep(.n-tabs-tab[data-name="tag-dictionary"]),
.nav-tabs :deep(.n-tabs-tab[data-name="template-management"]),
.nav-tabs :deep(.n-tabs-tab[data-name="data-management"]),
.nav-tabs :deep(.n-tabs-tab[data-name="settings"]) {
  font-size: 12px;
  font-weight: 400 !important;
  opacity: 0.55;
  transition: opacity 0.2s ease;
}
.nav-tabs :deep(.n-tabs-tab[data-name="tag-dictionary"]:hover),
.nav-tabs :deep(.n-tabs-tab[data-name="template-management"]:hover),
.nav-tabs :deep(.n-tabs-tab[data-name="data-management"]:hover),
.nav-tabs :deep(.n-tabs-tab[data-name="settings"]:hover) {
  opacity: 0.85;
}
.nav-tabs :deep(.n-tabs-tab[data-name="tag-dictionary"].n-tabs-tab--active),
.nav-tabs :deep(.n-tabs-tab[data-name="template-management"].n-tabs-tab--active),
.nav-tabs :deep(.n-tabs-tab[data-name="data-management"].n-tabs-tab--active),
.nav-tabs :deep(.n-tabs-tab[data-name="settings"].n-tabs-tab--active) {
  opacity: 1;
}

/* 紧凑化：缩小标签内边距 */
.nav-tabs :deep(.n-tabs-nav) {
  --n-tab-padding: 6px 10px;
}
.nav-tabs :deep(.n-tabs-tab) {
  padding: 6px 10px !important;
}

.nav-tabs :deep(.n-tabs-tab-pad),
.nav-tabs :deep(.n-tabs-tab-wrapper) {
  border-bottom: none !important;
}

.nav-tabs :deep(.n-tabs-rail) {
  display: none !important;
}

.nav-tabs :deep(.n-tabs-tab) {
  font-weight: 500;
  transition: color 0.2s ease, opacity 0.2s ease;
}

.nav-tabs :deep(.n-tabs-tab--active) {
  font-weight: 600;
}
</style>
