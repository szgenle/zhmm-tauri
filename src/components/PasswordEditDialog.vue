<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import { DiceOutline } from "@vicons/ionicons5";
import {
  api,
  type PasswordEntry,
  type PasswordInput,
} from "../api";
import PasswordStrengthBar from "./PasswordStrengthBar.vue";
import RandomPasswordDialog from "./RandomPasswordDialog.vue";
import AddRoleDialog from "./AddRoleDialog.vue";

const props = defineProps<{
  show: boolean;
  editEntry: PasswordEntry | null; // null = 新增模式
}>();

const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "saved"): void;
}>();

const message = useMessage();

const editing = ref(false);
const form = reactive<Required<PasswordInput>>({
  title: "",
  role: "个人",
  username: "",
  password: "",
  phone: "",
  email: "",
  url: "",
  notes: "",
  tags: [] as string[],
  totp_secret: "",
  totp_algo: "",
  totp_digits: 6,
  totp_period: 30,
});

// TOTP 区域展开控制
const totpEnabled = ref(false);
const otpauthUri = ref("");

// 随机密码对话框
const showRandomPwdDialog = ref(false);

function onRandomPwdConfirm(pwd: string) {
  form.password = pwd;
  showRandomPwdDialog.value = false;
}

// 新建类别对话框
const showAddRoleDialog = ref(false);
const roleOptions = ref<{ label: string; value: string }[]>([
  { label: "个人", value: "个人" },
  { label: "工作", value: "工作" },
  { label: "其它", value: "其它" },
]);

function onAddRoleSuccess(role: string) {
  if (!roleOptions.value.find((o) => o.value === role)) {
    roleOptions.value = [...roleOptions.value, { label: role, value: role }];
  }
  form.role = role;
  message.success(`已新建类别「${role}」`);
}

// 监听 show 变化，初始化表单
watch(
  () => props.show,
  (visible) => {
    if (!visible) return;
    if (props.editEntry) {
      const e = props.editEntry;
      form.title = e.title;
      form.role = e.role || "个人";
      form.username = e.username;
      form.password = e.password;
      form.phone = e.phone;
      form.email = e.email;
      form.url = e.url;
      form.notes = e.notes;
      form.tags = [...e.tags];
      form.totp_secret = e.totp_secret;
      form.totp_algo = e.totp_algo;
      form.totp_digits = e.totp_digits || 6;
      form.totp_period = e.totp_period || 30;
      totpEnabled.value = !!e.totp_secret;
    } else {
      resetForm();
    }
    loadRoles();
  }
);

watch(totpEnabled, (v) => {
  if (!v) {
    form.totp_secret = "";
    form.totp_algo = "";
    form.totp_digits = 6;
    form.totp_period = 30;
    otpauthUri.value = "";
  }
});

// 网址自动建议标签（防抖）
let urlSuggestTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => form.url,
  (newUrl) => {
    if (urlSuggestTimer) clearTimeout(urlSuggestTimer);
    const trimmed = (newUrl || "").trim();
    if (!trimmed) return;
    urlSuggestTimer = setTimeout(async () => {
      try {
        const suggestion = await api.suggestSite(trimmed);
        if (suggestion.matched && suggestion.tags.length > 0) {
          const existing = new Set(form.tags);
          for (const t of suggestion.tags) {
            if (!existing.has(t)) {
              form.tags.push(t);
              existing.add(t);
            }
          }
          if (!form.title.trim() && suggestion.name) {
            form.title = suggestion.name;
          }
        }
      } catch {
        // 静默忽略建议失败
      }
    }, 600);
  }
);

function resetForm() {
  form.title = "";
  form.role = "个人";
  form.username = "";
  form.password = "";
  form.phone = "";
  form.email = "";
  form.url = "";
  form.notes = "";
  form.tags = [];
  form.totp_secret = "";
  form.totp_algo = "";
  form.totp_digits = 6;
  form.totp_period = 30;
  otpauthUri.value = "";
  totpEnabled.value = false;
}

async function handleSave() {
  if (!form.title?.trim()) {
    message.error("名称不能为空");
    return;
  }
  editing.value = true;
  try {
    const payload: PasswordInput = { ...form };
    if (props.editEntry) {
      await api.updatePassword(props.editEntry.id, payload);
      message.success("已更新");
    } else {
      await api.addPassword(payload);
      message.success("已添加");
    }
    emit("update:show", false);
    emit("saved");
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  } finally {
    editing.value = false;
  }
}

async function importOtpauth() {
  if (!otpauthUri.value.trim()) {
    message.error("请粘贴 otpauth:// URI");
    return;
  }
  try {
    const p = await api.parseOtpauth(otpauthUri.value.trim());
    form.totp_secret = p.secret;
    form.totp_algo = p.algo;
    form.totp_digits = p.digits;
    form.totp_period = p.period;
    totpEnabled.value = true;
    if (!form.title) form.title = p.issuer || p.label || form.title;
    otpauthUri.value = "";
    message.success("已导入 TOTP 配置");
  } catch (e: any) {
    message.error(`解析失败: ${e}`);
  }
}

async function loadRoles() {
  try {
    const roles = await api.listRoles();
    roleOptions.value = roles.map((r) => ({ label: r, value: r }));
  } catch {
    // 失败时保留默认
  }
}
</script>

<template>
  <n-modal
    :show="show"
    preset="card"
    :title="editEntry ? '编辑密码' : '添加密码'"
    style="width: 560px"
    @update:show="emit('update:show', $event)"
  >
    <n-form label-placement="left" label-width="72">
      <n-form-item label="名称" required>
        <n-input v-model:value="form.title" placeholder="例如：GitHub" />
      </n-form-item>
      <n-form-item label="分类">
        <n-input-group>
          <n-select
            v-model:value="form.role"
            :options="roleOptions"
            filterable
            placeholder="选择类别"
            style="flex: 1"
          />
          <n-button @click="showAddRoleDialog = true" title="新建类别">
            + 新建
          </n-button>
        </n-input-group>
      </n-form-item>
      <n-form-item label="用户名">
        <n-input v-model:value="form.username" />
      </n-form-item>
      <n-form-item label="密码">
        <div style="width: 100%">
          <n-input-group>
            <n-input
              v-model:value="form.password"
              type="password"
              show-password-on="click"
              style="flex: 1"
            />
            <n-button @click="showRandomPwdDialog = true" title="生成随机密码">
              <template #icon><n-icon><DiceOutline /></n-icon></template>
            </n-button>
          </n-input-group>
          <PasswordStrengthBar :password="form.password" />
        </div>
      </n-form-item>
      <n-form-item label="手机">
        <n-input v-model:value="form.phone" />
      </n-form-item>
      <n-form-item label="邮箱">
        <n-input v-model:value="form.email" />
      </n-form-item>
      <n-form-item label="网址">
        <n-input v-model:value="form.url" placeholder="https://..." />
      </n-form-item>
      <n-form-item label="标签">
        <n-dynamic-tags v-model:value="form.tags" />
      </n-form-item>
      <n-form-item label="备注">
        <n-input v-model:value="form.notes" type="textarea" :rows="3" />
      </n-form-item>
      <n-divider style="margin: 12px 0 8px">
        <n-checkbox v-model:checked="totpEnabled">两步验证 (TOTP)</n-checkbox>
      </n-divider>
      <template v-if="totpEnabled">
        <n-form-item label="otpauth">
          <n-input-group>
            <n-input
              v-model:value="otpauthUri"
              placeholder="粘贴 otpauth://totp/... 一键导入"
            />
            <n-button @click="importOtpauth">导入</n-button>
          </n-input-group>
        </n-form-item>
        <n-form-item label="密钥">
          <n-input
            v-model:value="form.totp_secret"
            placeholder="Base32 字符串，留空表示不启用 2FA"
          />
        </n-form-item>
        <n-form-item label="算法">
          <n-select
            v-model:value="form.totp_algo"
            :options="[
              { label: '默认 (SHA1)', value: '' },
              { label: 'SHA1', value: 'SHA1' },
              { label: 'SHA256', value: 'SHA256' },
              { label: 'SHA512', value: 'SHA512' },
              { label: 'SM3 (国密扩展，仅 zhmm 互通)', value: 'SM3' },
            ]"
          />
        </n-form-item>
        <n-form-item label="位数">
          <n-input-number v-model:value="form.totp_digits" :min="6" :max="10" />
        </n-form-item>
        <n-form-item label="周期(秒)">
          <n-input-number v-model:value="form.totp_period" :min="1" :max="300" />
        </n-form-item>
      </template>
    </n-form>
    <template #footer>
      <n-space justify="end">
        <n-button @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" :loading="editing" @click="handleSave">
          保存
        </n-button>
      </n-space>
    </template>
  </n-modal>

  <!-- 随机密码生成器 -->
  <n-modal
    v-model:show="showRandomPwdDialog"
    preset="card"
    title="生成随机密码"
    style="width: 480px"
  >
    <RandomPasswordDialog @confirm="onRandomPwdConfirm" />
  </n-modal>

  <!-- 新建类别对话框 -->
  <AddRoleDialog
    v-model:show="showAddRoleDialog"
    :existing-roles="roleOptions.map(o => o.value)"
    @success="onAddRoleSuccess"
  />
</template>
