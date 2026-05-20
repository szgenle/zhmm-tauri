<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useMessage } from "naive-ui";
import { api } from "../api";
import PasswordStrengthBar from "./PasswordStrengthBar.vue";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "success", backupName: string): void;
}>();

const message = useMessage();

const oldPassword = ref("");
const newPassword = ref("");
const confirmPassword = ref("");
const busy = ref(false);
const stage = ref<"input" | "verifying" | "rekeying" | "done">("input");
const stageText = ref("");

const errorHint = computed(() => {
  if (!newPassword.value) return "";
  if (newPassword.value.length < 4) return "新主密码至少 4 个字符";
  if (newPassword.value === oldPassword.value) return "新主密码不能与旧主密码相同";
  if (confirmPassword.value && confirmPassword.value !== newPassword.value)
    return "两次输入的新密码不一致";
  return "";
});

const canSubmit = computed(() => {
  return (
    !busy.value &&
    oldPassword.value.length > 0 &&
    newPassword.value.length >= 4 &&
    newPassword.value !== oldPassword.value &&
    confirmPassword.value === newPassword.value
  );
});

// 关闭时重置状态
watch(
  () => props.show,
  (v) => {
    if (!v) {
      reset();
    }
  }
);

function reset() {
  oldPassword.value = "";
  newPassword.value = "";
  confirmPassword.value = "";
  busy.value = false;
  stage.value = "input";
  stageText.value = "";
}

function handleClose() {
  if (busy.value) return;
  emit("update:show", false);
}

async function handleSubmit() {
  if (!canSubmit.value) return;
  busy.value = true;
  try {
    // 1. 校验旧主密码
    stage.value = "verifying";
    stageText.value = "正在校验旧主密码...";
    const ok = await api.verifyMasterPassword(oldPassword.value);
    if (!ok) {
      message.error("旧主密码错误");
      busy.value = false;
      stage.value = "input";
      stageText.value = "";
      return;
    }

    // 2. 执行 rekey（创建保险备份 + 重新加密落盘）
    stage.value = "rekeying";
    stageText.value = "正在创建保险备份并重新加密...";
    const backupName = await api.rekeyVault(oldPassword.value, newPassword.value);

    stage.value = "done";
    stageText.value = "";
    message.success(`主密码已更换；保险备份：${backupName}`);
    emit("success", backupName);
    emit("update:show", false);
  } catch (e: any) {
    message.error(`更换失败: ${e}`);
    stage.value = "input";
    stageText.value = "";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <n-modal
    :show="show"
    @update:show="handleClose"
    preset="card"
    title="更换主密码"
    style="width: 480px"
    :mask-closable="false"
    :closable="!busy"
  >
    <n-space vertical size="medium">
      <n-form label-placement="top" :show-feedback="false">
        <n-form-item label="当前主密码">
          <n-input
            v-model:value="oldPassword"
            type="password"
            show-password-on="click"
            placeholder="请输入当前主密码"
            :disabled="busy"
          />
        </n-form-item>
        <n-form-item label="新主密码">
          <n-input
            v-model:value="newPassword"
            type="password"
            show-password-on="click"
            placeholder="至少 4 个字符，建议使用强密码"
            :disabled="busy"
          />
        </n-form-item>
        <PasswordStrengthBar :password="newPassword" />
        <n-form-item label="确认新主密码" style="margin-top: 12px">
          <n-input
            v-model:value="confirmPassword"
            type="password"
            show-password-on="click"
            placeholder="再次输入新主密码"
            :disabled="busy"
            @keyup.enter="handleSubmit"
          />
        </n-form-item>
      </n-form>

      <n-alert v-if="errorHint" type="warning" :show-icon="false" style="padding: 6px 10px">
        {{ errorHint }}
      </n-alert>

      <n-alert type="info" :show-icon="false" style="padding: 8px 12px; font-size: 12px">
        换密前会先在
        <n-text code>.backups/</n-text>
        生成一份保险备份（前缀 <n-text code>rekey_</n-text>），失败时可手动恢复。
      </n-alert>

      <n-spin v-if="busy" :description="stageText" size="small" style="text-align: center" />
    </n-space>

    <template #footer>
      <n-space justify="end">
        <n-button :disabled="busy" @click="handleClose">取消</n-button>
        <n-button type="primary" :disabled="!canSubmit" :loading="busy" @click="handleSubmit">
          确认更换
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>
