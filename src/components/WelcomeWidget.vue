<script setup lang="ts">
import { ShieldCheckmarkOutline, AddOutline } from "@vicons/ionicons5";

const emit = defineEmits<{
  (e: "add"): void;
}>();
</script>

<template>
  <div class="welcome">
    <n-icon size="64" :depth="3"><ShieldCheckmarkOutline /></n-icon>
    <h1 class="title">欢迎使用账号小本本</h1>
    <n-divider style="width: 320px" />
    <p class="info">
      这是一个安全的本地密码管理工具，可以帮助您管理各种账号密码。<br />
      点击下方按钮开始添加你的第一条密码。
    </p>
    <n-button type="primary" size="large" @click="emit('add')">
      <template #icon>
        <n-icon><AddOutline /></n-icon>
      </template>
      添加密码
    </n-button>
    <n-text depth="3" class="hint">
      数据全部加密保存在本地（SM4-CBC + HMAC-SM3 + Argon2id），不会上传任何服务器。
    </n-text>
  </div>
</template>

<style scoped>
.welcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px 40px;
  text-align: center;
}
.title {
  margin: 16px 0 4px;
  font-size: 26px;
  font-weight: 600;
  color: var(--n-text-color);
}
.info {
  color: var(--n-text-color-3);
  font-size: 14px;
  line-height: 1.8;
  margin: 0 0 28px;
}
.hint {
  margin-top: 24px;
  font-size: 12px;
  max-width: 460px;
}
</style>
