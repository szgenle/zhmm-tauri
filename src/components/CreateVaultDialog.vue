<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useMessage } from "naive-ui";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type RecentEntry } from "../api";
import PasswordStrengthBar from "./PasswordStrengthBar.vue";

/**
 * 创建新账号库对话框：
 *   1. 用户选保存路径（必须以 .zhmm 结尾）
 *   2. 输入账号名（参与 KDF，遗忘后无法解密）
 *   3. 输入两次主密码
 *   4. 调 createVaultAt 落地，bcryptHash 后写入最近访问
 */
const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "success", entry: RecentEntry): void;
}>();

const message = useMessage();

const filePath = ref("");
const account = ref("");
const password = ref("");
const confirmPassword = ref("");
const busy = ref(false);

watch(
  () => props.show,
  (v) => {
    if (v) {
      filePath.value = "";
      account.value = "";
      password.value = "";
      confirmPassword.value = "";
      busy.value = false;
    }
  }
);

const errorHint = computed(() => {
  if (!password.value) return "";
  if (password.value.length < 4) return "主密码至少 4 个字符";
  if (confirmPassword.value && confirmPassword.value !== password.value)
    return "两次输入的主密码不一致";
  return "";
});

const canSubmit = computed(() => {
  return (
    !busy.value &&
    filePath.value.trim().length > 0 &&
    account.value.trim().length > 0 &&
    password.value.length >= 4 &&
    confirmPassword.value === password.value
  );
});

function nowString(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  );
}

async function chooseFile() {
  if (busy.value) return;
  try {
    const selected = await saveDialog({
      title: "保存为",
      defaultPath: "zhmm.zhmm",
      filters: [{ name: "ZHMM 账号库", extensions: ["zhmm"] }],
    });
    if (typeof selected === "string" && selected) {
      filePath.value = selected.endsWith(".zhmm") ? selected : `${selected}.zhmm`;
    }
  } catch (e: any) {
    message.error(`选择文件失败: ${e}`);
  }
}

function handleClose() {
  if (busy.value) return;
  emit("update:show", false);
}

async function handleSubmit() {
  if (!canSubmit.value) return;
  busy.value = true;
  try {
    const path = filePath.value.trim();
    const acc = account.value.trim();

    // 防止覆盖已存在文件
    const exists = await api.pathExists(path);
    if (exists) {
      message.error("目标文件已存在，请换一个路径");
      busy.value = false;
      return;
    }

    await api.createVaultAt(path, acc, password.value);
    const hashpw = await api.bcryptHash(password.value);
    const entry: RecentEntry = {
      path,
      account: acc,
      hashpw,
      last_access_time: nowString(),
    };
    await api.upsertRecent(entry);

    message.success("账号库已创建");
    emit("success", entry);
    emit("update:show", false);
  } catch (e: any) {
    message.error(`创建失败: ${e}`);
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
    title="新建账号库"
    style="width: 520px"
    :mask-closable="false"
    :closable="!busy"
  >
    <n-space vertical size="medium">
      <n-form label-placement="top" :show-feedback="false">
        <n-form-item label="保存路径">
          <n-input-group>
            <n-input
              v-model:value="filePath"
              placeholder="点击右侧选择保存位置"
              :disabled="busy"
            />
            <n-button :disabled="busy" @click="chooseFile">选择...</n-button>
          </n-input-group>
        </n-form-item>
        <n-form-item label="账号名">
          <n-input
            v-model:value="account"
            placeholder="账号名作为常量盐参与密钥派生，请妥善记录"
            :disabled="busy"
          />
        </n-form-item>
        <n-form-item label="主密码">
          <n-input
            v-model:value="password"
            type="password"
            show-password-on="click"
            placeholder="至少 4 个字符，建议使用强密码"
            :disabled="busy"
          />
        </n-form-item>
        <PasswordStrengthBar :password="password" />
        <n-form-item label="确认主密码" style="margin-top: 12px">
          <n-input
            v-model:value="confirmPassword"
            type="password"
            show-password-on="click"
            placeholder="再次输入主密码"
            :disabled="busy"
            @keyup.enter="handleSubmit"
          />
        </n-form-item>
      </n-form>

      <n-alert v-if="errorHint" type="warning" :show-icon="false" style="padding: 6px 10px">
        {{ errorHint }}
      </n-alert>

      <n-alert type="warning" :show-icon="false" style="padding: 8px 12px; font-size: 12px">
        账号名与主密码均参与密钥派生，遗忘其中任何一项都将导致密码库永久无法解密。
      </n-alert>
    </n-space>

    <template #footer>
      <n-space justify="end">
        <n-button :disabled="busy" @click="handleClose">取消</n-button>
        <n-button type="primary" :disabled="!canSubmit" :loading="busy" @click="handleSubmit">
          创建
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>
