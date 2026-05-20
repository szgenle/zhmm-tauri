<script setup lang="ts">
import { darkTheme, lightTheme, zhCN, dateZhCN } from "naive-ui";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { settings, loadSettings } from "./settings";
import { api } from "./api";

const sysIsDark = ref(false);

const isDark = computed(() => {
  if (settings.theme === "dark") return true;
  if (settings.theme === "light") return false;
  return sysIsDark.value; // auto
});
const theme = computed(() => (isDark.value ? darkTheme : lightTheme));

const mql = window.matchMedia("(prefers-color-scheme: dark)");
const handleChange = (e: MediaQueryListEvent) => {
  sysIsDark.value = e.matches;
};

onMounted(async () => {
  sysIsDark.value = mql.matches;
  mql.addEventListener("change", handleChange);
  await loadSettings();
  // 启动后按设置应用防截屏
  try {
    await api.applyAntiCapture(settings.anti_screenshot ?? true);
  } catch {
    // 平台不支持时忽略
  }
});

// 设置切换时实时应用
watch(
  () => settings.anti_screenshot,
  async (v) => {
    try {
      await api.applyAntiCapture(v ?? true);
    } catch {
      // 忽略
    }
  }
);

onUnmounted(() => {
  mql.removeEventListener("change", handleChange);
});
</script>

<template>
  <n-config-provider :theme="theme" :locale="zhCN" :date-locale="dateZhCN">
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <n-loading-bar-provider>
            <router-view />
          </n-loading-bar-provider>
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
