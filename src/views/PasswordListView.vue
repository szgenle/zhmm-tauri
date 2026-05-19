<script setup lang="ts">
import { computed, h, onMounted, reactive, ref } from "vue";
import { NButton, NIcon, NTag, NSpace, useMessage, useDialog } from "naive-ui";
import {
  AddOutline,
  CopyOutline,
  CreateOutline,
  DiceOutline,
  OpenOutline,
  TimeOutline,
  TrashOutline,
} from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  type PasswordEntry,
  type PasswordHistoryItem,
  type PasswordInput,
  type PasswordSummary,
} from "../api";
import TotpCell from "../components/TotpCell.vue";
import PasswordStrengthBar from "../components/PasswordStrengthBar.vue";
import RandomPasswordDialog from "../components/RandomPasswordDialog.vue";
import TagSidebar from "../components/TagSidebar.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { copyAndScheduleClear } from "../settings";

const message = useMessage();
const dialog = useDialog();

const searchQuery = ref("");
const data = ref<PasswordSummary[]>([]);
const loading = ref(false);
const selectedTags = ref<string[]>([]);

/**
 * 字符串规范化：去首尾空白 + NFKC + 转小写
 * 与 Python 版 unicodedata.normalize('NFKC') + casefold() 一致
 */
function normalize(s: string): string {
  return (s || "").trim().normalize("NFKC").toLowerCase();
}

const filtered = computed<PasswordSummary[]>(() => {
  let result = data.value;
  // 标签 AND 筛选
  if (selectedTags.value.length > 0) {
    result = result.filter((row) =>
      selectedTags.value.every((t) => (row.tags || []).includes(t))
    );
  }
  // 关键词搜索
  const q = normalize(searchQuery.value);
  if (!q) return result;
  return result.filter((row) => {
    const fields = [
      row.title,
      row.role,
      row.username,
      row.phone,
      row.email,
      row.url,
      ...(row.tags || []),
    ];
    return fields.some((f) => normalize(String(f || "")).includes(q));
  });
});

// 编辑对话框
const showEditDialog = ref(false);
const editing = ref(false);
const editingId = ref<string | null>(null); // null=新增
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

// 历史对话框
const showHistoryDialog = ref(false);
const historyTitle = ref("");
const historyItems = ref<PasswordHistoryItem[]>([]);

// 随机密码对话框
const showRandomPwdDialog = ref(false);

function onRandomPwdConfirm(pwd: string) {
  form.password = pwd;
  showRandomPwdDialog.value = false;
}

async function loadData() {
  loading.value = true;
  try {
    data.value = await api.listPasswords();
    await loadRoles();
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

async function handleCopy(row: PasswordSummary) {
  try {
    const entry = await api.getPassword(row.id);
    await copyAndScheduleClear(entry.password);
    message.success("密码已复制到剪贴板");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

async function handleOpenUrl(row: PasswordSummary) {
  if (!row.url) return;
  try {
    await openUrl(row.url);
  } catch (e: any) {
    message.error(`打开失败: ${e}`);
  }
}

function handleDelete(row: PasswordSummary) {
  dialog.warning({
    title: "确认删除",
    content: `确定要删除"${row.title}"吗？此操作不可恢复。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.deletePassword(row.id);
        message.success("已删除");
        loadData();
      } catch (e: any) {
        message.error(`删除失败: ${e}`);
      }
    },
  });
}

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
}

function openAdd() {
  resetForm();
  editingId.value = null;
  showEditDialog.value = true;
}

async function openEdit(row: PasswordSummary) {
  try {
    const e: PasswordEntry = await api.getPassword(row.id);
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
    editingId.value = e.id;
    showEditDialog.value = true;
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  }
}

async function handleSave() {
  if (!form.title?.trim()) {
    message.error("名称不能为空");
    return;
  }
  editing.value = true;
  try {
    const payload: PasswordInput = { ...form };
    if (editingId.value) {
      await api.updatePassword(editingId.value, payload);
      message.success("已更新");
    } else {
      await api.addPassword(payload);
      message.success("已添加");
    }
    showEditDialog.value = false;
    loadData();
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  } finally {
    editing.value = false;
  }
}

async function openHistory(row: PasswordSummary) {
  try {
    historyItems.value = await api.getPasswordHistory(row.id);
    historyTitle.value = row.title;
    showHistoryDialog.value = true;
  } catch (e: any) {
    message.error(`加载历史失败: ${e}`);
  }
}

async function copyHistory(item: PasswordHistoryItem) {
  await copyAndScheduleClear(item.pwd);
  message.success("已复制历史密码");
}

// 粘贴 otpauth URI 自动填充
const otpauthUri = ref("");
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
    if (!form.title) form.title = p.issuer || p.label || form.title;
    otpauthUri.value = "";
    message.success("已导入 TOTP 配置");
  } catch (e: any) {
    message.error(`解析失败: ${e}`);
  }
}

const columns: DataTableColumns<PasswordSummary> = [
  { title: "名称", key: "title", width: 180 },
  { title: "分类", key: "role", width: 80 },
  { title: "用户名", key: "username", width: 160 },
  {
    title: "标签",
    key: "tags",
    render(row) {
      if (!row.tags?.length) return "";
      return h(
        NSpace,
        { size: 4 },
        {
          default: () =>
            row.tags.map((t) =>
              h(NTag, { size: "small", round: true }, { default: () => t })
            ),
        }
      );
    },
  },
  {
    title: "2FA",
    key: "totp",
    width: 120,
    render(row) {
      if (!row.has_totp) return "";
      return h(TotpCell, { id: row.id });
    },
  },
  {
    title: "更新时间",
    key: "updated_at",
    width: 170,
    render: (row) => new Date(row.updated_at).toLocaleString(),
  },
  {
    title: "操作",
    key: "actions",
    width: 280,
    render(row) {
      return h(NSpace, { size: 0 }, {
        default: () => [
          h(NButton, { size: "small", quaternary: true, onClick: () => handleCopy(row) },
            { default: () => "复制", icon: () => h(NIcon, null, { default: () => h(CopyOutline) }) }),
          h(NButton, { size: "small", quaternary: true, onClick: () => openEdit(row) },
            { default: () => "编辑", icon: () => h(NIcon, null, { default: () => h(CreateOutline) }) }),
          ...(row.url ? [h(NButton, { size: "small", quaternary: true, onClick: () => handleOpenUrl(row) },
            { default: () => "打开", icon: () => h(NIcon, null, { default: () => h(OpenOutline) }) })] : []),
          h(NButton, { size: "small", quaternary: true, onClick: () => openHistory(row) },
            { default: () => "历史", icon: () => h(NIcon, null, { default: () => h(TimeOutline) }) }),
          h(NButton, { size: "small", quaternary: true, type: "error", onClick: () => handleDelete(row) },
            { default: () => "删除", icon: () => h(NIcon, null, { default: () => h(TrashOutline) }) }),
        ],
      });
    },
  },
];

const roleOptions = ref<{ label: string; value: string }[]>([
  { label: "个人", value: "个人" },
  { label: "工作", value: "工作" },
  { label: "其它", value: "其它" },
]);

async function loadRoles() {
  try {
    const roles = await api.listRoles();
    roleOptions.value = roles.map((r) => ({ label: r, value: r }));
  } catch {
    // 失败时保留默认
  }
}

onMounted(loadData);
</script>

<template>
  <div class="pwd-page">
    <TagSidebar :entries="data" :selected-tags="selectedTags" @update:selected-tags="v => selectedTags = v" />
    <div class="pwd-main">
    <n-space justify="space-between" style="margin-bottom: 16px">
      <n-input
        v-model:value="searchQuery"
        placeholder="搜索密码"
        clearable
        style="width: 280px"
      />
      <n-button type="primary" @click="openAdd">
        <template #icon>
          <n-icon><AddOutline /></n-icon>
        </template>
        添加密码
      </n-button>
    </n-space>
    <n-data-table
      :columns="columns"
      :data="filtered"
      :loading="loading"
      :bordered="false"
      :pagination="{ pageSize: 20 }"
    />

    <!-- 新增/编辑对话框 -->
    <n-modal
      v-model:show="showEditDialog"
      preset="card"
      :title="editingId ? '编辑密码' : '添加密码'"
      style="width: 560px"
    >
      <n-form label-placement="left" label-width="72">
        <n-form-item label="名称" required>
          <n-input v-model:value="form.title" placeholder="例如：GitHub" />
        </n-form-item>
        <n-form-item label="分类">
          <n-select
            v-model:value="form.role"
            :options="roleOptions"
            filterable
            tag
            placeholder="选择或输入新类别"
          />
        </n-form-item>
        <n-form-item label="用户名">
          <n-input v-model:value="form.username" />
        </n-form-item>
        <n-form-item label="密码">
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
        <n-divider style="margin: 12px 0 8px">两步验证 (TOTP)</n-divider>
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
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showEditDialog = false">取消</n-button>
          <n-button type="primary" :loading="editing" @click="handleSave">
            保存
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- 历史密码对话框 -->
    <n-modal
      v-model:show="showHistoryDialog"
      preset="card"
      :title="`历史密码 - ${historyTitle}`"
      style="width: 480px"
    >
      <n-empty v-if="!historyItems.length" description="暂无历史密码" />
      <n-list v-else>
        <n-list-item v-for="(item, idx) in historyItems" :key="idx">
          <n-space justify="space-between" align="center" style="width: 100%">
            <div>
              <div style="font-family: monospace">
                {{ item.pwd }}
              </div>
              <div style="font-size: 12px; color: var(--n-text-color-3)">
                替换于 {{ new Date(item.replaced_at).toLocaleString() }}
              </div>
            </div>
            <n-button size="small" quaternary @click="copyHistory(item)">
              复制
            </n-button>
          </n-space>
        </n-list-item>
      </n-list>
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
    </div>
  </div>
</template>

<style scoped>
.pwd-page {
  display: flex;
  height: 100%;
}
.pwd-main {
  flex: 1;
  overflow: auto;
  padding: 0;
}
</style>
