<script setup lang="ts">
import { darkTheme, lightTheme, zhCN, dateZhCN } from "naive-ui";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { settings, loadSettings } from "./settings";

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
});

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
