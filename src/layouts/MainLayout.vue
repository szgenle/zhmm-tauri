<script setup lang="ts">
import { computed, h, ref } from "vue";
import { NIcon } from "naive-ui";
import { useRoute, useRouter, RouterLink } from "vue-router";
import {
  KeyOutline,
  SettingsOutline,
  LockClosedOutline,
} from "@vicons/ionicons5";
import type { MenuOption } from "naive-ui";
import { api } from "../api";

const route = useRoute();
const router = useRouter();
const message = useMessage();

const collapsed = ref(false);

const renderIcon = (icon: any) => () => h(NIcon, null, { default: () => h(icon) });

const menuOptions: MenuOption[] = [
  {
    label: () => h(RouterLink, { to: "/" }, { default: () => "密码库" }),
    key: "passwords",
    icon: renderIcon(KeyOutline),
  },
  {
    label: () => h(RouterLink, { to: "/settings" }, { default: () => "设置" }),
    key: "settings",
    icon: renderIcon(SettingsOutline),
  },
];

const activeKey = computed<string>(() => (route.name as string) ?? "passwords");
const pageTitle = computed(() => (route.meta.title as string) ?? "");

async function handleLock() {
  try {
    await api.lockVault();
    router.push("/login");
  } catch (e: any) {
    message.error(`锁定失败: ${e}`);
  }
}
</script>

<template>
  <n-layout has-sider style="height: 100vh">
    <n-layout-sider
      bordered
      collapse-mode="width"
      :collapsed-width="64"
      :width="200"
      :collapsed="collapsed"
      show-trigger
      @collapse="collapsed = true"
      @expand="collapsed = false"
    >
      <div class="brand">
        <n-icon size="24"><KeyOutline /></n-icon>
        <span v-if="!collapsed" class="brand-text">智慧密码</span>
      </div>
      <n-menu
        :collapsed="collapsed"
        :collapsed-width="64"
        :collapsed-icon-size="22"
        :options="menuOptions"
        :value="activeKey"
      />
    </n-layout-sider>
    <n-layout>
      <n-layout-header bordered class="header">
        <span class="title">{{ pageTitle }}</span>
        <n-button quaternary size="small" @click="handleLock">
          <template #icon>
            <n-icon><LockClosedOutline /></n-icon>
          </template>
          锁定
        </n-button>
      </n-layout-header>
      <n-layout-content content-style="padding: 24px;">
        <router-view />
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<style scoped>
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 20px;
  font-size: 16px;
  font-weight: 600;
}
.brand-text {
  white-space: nowrap;
}
.header {
  height: 56px;
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.title {
  font-size: 16px;
  font-weight: 600;
}
</style>
