<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useMessage } from "naive-ui";
import { api, type RecentEntry } from "../api";

/**
 * 解锁某个账号库的对话框。
 *
 * 两种使用场景：
 *   1. 点击最近访问条目：传入 path/account/hashpw，account 不可改，
 *      提交时先 bcryptVerify(密码, hashpw) 做 UI 层快速校验，再 unlockWithPath。
 *   2. 通过 "打开外部 .zhmm" 选了一个不在最近访问中的文件：传入 path，
 *      account 由用户输入，hashpw 不传，跳过 bcrypt 预校验直接 unlockWithPath。
 */
const props = defineProps<{
  show: boolean;
  path: string;
  account?: string;
  hashpw?: string;
}>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "success", entry: RecentEntry): void;
}>();

const message = useMessage();

const accountInput = ref("");
const password = ref("");
const busy = ref(false);

const accountReadonly = computed(() => !!props.account);

watch(
  () => props.show,
  (v) => {
    if (v) {
      accountInput.value = props.account ?? "";
      password.value = "";
      busy.value = false;
    }
  }
);

const canSubmit = computed(() => {
  return !busy.value && accountInput.value.trim().length > 0 && password.value.length > 0;
});

function nowString(): string {
  // Python 版用 "%Y-%m-%d %H:%M:%S"，保持一致以便将来交叉迁移
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  );
}

function handleClose() {
  if (busy.value) return;
  emit("update:show", false);
}

async function handleSubmit() {
  if (!canSubmit.value) return;
  const account = accountInput.value.trim();
  busy.value = true;
  try {
    // 1. 若有 hashpw，先 bcrypt 预校验（快速反馈，避免 Argon2id 慢解密）
    if (props.hashpw) {
      const ok = await api.bcryptVerify(password.value, props.hashpw);
      if (!ok) {
        message.error("主密码错误");
        busy.value = false;
        return;
      }
    }

    // 2. 实际解密落地
    await api.unlockWithPath(props.path, account, password.value);

    // 3. 计算 / 复用 hashpw 并写入最近访问列表
    let hashpw = props.hashpw ?? "";
    if (!hashpw) {
      hashpw = await api.bcryptHash(password.value);
    }
    const entry: RecentEntry = {
      path: props.path,
      account,
      hashpw,
      last_access_time: nowString(),
    };
    await api.upsertRecent(entry);

    emit("success", entry);
    emit("update:show", false);
  } catch (e: any) {
    message.error(`解锁失败: ${e}`);
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
    title="解锁账号库"
    style="width: 460px"
    :mask-closable="false"
    :closable="!busy"
  >
    <n-space vertical size="medium">
      <n-form label-placement="top" :show-feedback="false">
        <n-form-item label="文件路径">
          <n-input :value="path" readonly />
        </n-form-item>
        <n-form-item label="账号名">
          <n-input
            v-model:value="accountInput"
            :readonly="accountReadonly"
            placeholder="请输入账号名（与创建时一致）"
            :disabled="busy"
          />
        </n-form-item>
        <n-form-item label="主密码">
          <n-input
            v-model:value="password"
            type="password"
            show-password-on="click"
            placeholder="请输入主密码"
            :disabled="busy"
            @keyup.enter="handleSubmit"
          />
        </n-form-item>
      </n-form>

      <n-alert type="info" :show-icon="false" style="padding: 8px 12px; font-size: 12px">
        账号名作为密钥派生的常量盐参与解密；账号或主密码错误均无法解锁。
      </n-alert>
    </n-space>

    <template #footer>
      <n-space justify="end">
        <n-button :disabled="busy" @click="handleClose">取消</n-button>
        <n-button type="primary" :disabled="!canSubmit" :loading="busy" @click="handleSubmit">
          解锁
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>
