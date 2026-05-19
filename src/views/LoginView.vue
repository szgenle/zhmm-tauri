<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ShieldCheckmarkOutline } from "@vicons/ionicons5";
import { api } from "../api";

const router = useRouter();
const message = useMessage();

const password = ref("");
const confirmPassword = ref("");
const loading = ref(false);
const isCreating = ref(false);
const checking = ref(true);

onMounted(async () => {
  try {
    const status = await api.vaultStatus();
    isCreating.value = !status.exists;
    if (status.unlocked) {
      router.push("/");
      return;
    }
  } catch (e) {
    console.error(e);
  } finally {
    checking.value = false;
  }
});

async function handleSubmit() {
  if (!password.value) return;
  if (isCreating.value && password.value !== confirmPassword.value) {
    message.error("两次输入的密码不一致");
    return;
  }
  loading.value = true;
  try {
    if (isCreating.value) {
      await api.createVault(password.value);
      message.success("密码库已创建");
    } else {
      await api.unlockVault(password.value);
    }
    password.value = "";
    confirmPassword.value = "";
    router.push("/");
  } catch (e: any) {
    message.error(`${e}`);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="login-page">
    <n-card class="login-card" :bordered="false">
      <div class="logo">
        <n-icon size="48" :depth="3"><ShieldCheckmarkOutline /></n-icon>
        <h1>智慧密码</h1>
        <p class="subtitle">
          {{ isCreating ? "创建你的本地密码库" : "输入主密码解锁密码库" }}
        </p>
      </div>
      <n-spin :show="checking">
        <n-form @submit.prevent="handleSubmit">
          <n-form-item label="主密码">
            <n-input
              v-model:value="password"
              type="password"
              show-password-on="click"
              placeholder="请输入主密码"
              :disabled="loading"
              @keyup.enter="handleSubmit"
            />
          </n-form-item>
          <n-form-item v-if="isCreating" label="确认密码">
            <n-input
              v-model:value="confirmPassword"
              type="password"
              show-password-on="click"
              placeholder="请再次输入主密码"
              :disabled="loading"
              @keyup.enter="handleSubmit"
            />
          </n-form-item>
          <n-button
            type="primary"
            block
            :loading="loading"
            :disabled="!password"
            @click="handleSubmit"
          >
            {{ isCreating ? "创建密码库" : "解锁" }}
          </n-button>
        </n-form>
      </n-spin>
    </n-card>
  </div>
</template>

<style scoped>
.login-page {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  width: 100vw;
}
.login-card {
  width: 380px;
  padding: 24px;
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
  margin: 0 0 8px;
}
</style>
