<script setup lang="ts">
import { darkTheme, lightTheme, zhCN, dateZhCN } from "naive-ui";
import { computed, onMounted, onUnmounted, ref } from "vue";

const isDark = ref(false);
const theme = computed(() => (isDark.value ? darkTheme : lightTheme));

const mql = window.matchMedia("(prefers-color-scheme: dark)");
const handleChange = (e: MediaQueryListEvent) => {
  isDark.value = e.matches;
};

onMounted(() => {
  isDark.value = mql.matches;
  mql.addEventListener("change", handleChange);
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
