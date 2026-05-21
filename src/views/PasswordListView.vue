<script setup lang="ts">
import { computed, h, nextTick, onMounted, ref } from "vue";
import { NButton, NIcon, NTag, NSpace, useMessage, useDialog } from "naive-ui";
import {
  AddOutline,
  CopyOutline,
  CreateOutline,
  EyeOutline,
  OpenOutline,
  TimeOutline,
  TrashOutline,
} from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  type PasswordEntry,
  type PasswordSummary,
} from "../api";
import TotpCell from "../components/TotpCell.vue";
import PasswordEditDialog from "../components/PasswordEditDialog.vue";
import PasswordHistoryDialog from "../components/PasswordHistoryDialog.vue";
import TagSidebar from "../components/TagSidebar.vue";
import WelcomeWidget from "../components/WelcomeWidget.vue";
import { usePasswordReveal } from "../composables/usePasswordReveal";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { copyAndScheduleClear } from "../settings";

const message = useMessage();
const dialog = useDialog();

const searchQuery = ref("");
const data = ref<PasswordSummary[]>([]);
const loading = ref(false);
const selectedTags = ref<string[]>([]);

// 密码明文定时隐藏
const { revealedPasswords, revealPassword } = usePasswordReveal(message);

/**
 * 字符串规范化：去首尾空白 + NFKC + 转小写
 */
function normalize(s: string): string {
  return (s || "").trim().normalize("NFKC").toLowerCase();
}

const filtered = computed<PasswordSummary[]>(() => {
  let result = data.value;
  if (selectedTags.value.length > 0) {
    result = result.filter((row) =>
      selectedTags.value.every((t) => (row.tags || []).includes(t))
    );
  }
  const q = normalize(searchQuery.value);
  if (!q) return result;
  return result.filter((row) => {
    const fields = [
      row.title, row.role, row.username, row.phone, row.email, row.url,
      ...(row.tags || []),
    ];
    return fields.some((f) => normalize(String(f || "")).includes(q));
  });
});

// 编辑对话框
const showEditDialog = ref(false);
const editEntry = ref<PasswordEntry | null>(null);

// 历史对话框
const showHistoryDialog = ref(false);
const historyTitle = ref("");
const historyEntryId = ref("");

// 右键菜单
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextRow = ref<PasswordSummary | null>(null);

const contextMenuOptions = computed(() => {
  const row = contextRow.value;
  if (!row) return [] as Array<{ label: string; key: string; disabled?: boolean; type?: string }>;
  const opts: Array<{ label?: string; key: string; disabled?: boolean; type?: string }> = [];
  opts.push({ label: "复制账号", key: "copy-user", disabled: !row.username });
  opts.push({ label: "复制密码", key: "copy" });
  if (row.has_totp) opts.push({ label: "复制动态码", key: "copy-totp" });
  if (row.url) opts.push({ label: "复制网址", key: "copy-url" });
  if (row.url) opts.push({ label: "打开网址", key: "open" });
  opts.push({ key: "d1", type: "divider" });
  opts.push({ label: revealedPasswords.value.has(row.id) ? "隐藏密码" : "显示密码", key: "reveal" });
  opts.push({ label: "编辑", key: "edit" });
  opts.push({ label: "历史密码", key: "history" });
  opts.push({ key: "d2", type: "divider" });
  opts.push({ label: "删除", key: "delete" });
  return opts as Array<{ label: string; key: string; disabled?: boolean; type?: string }>;
});

function onRowContextMenu(e: MouseEvent, row: PasswordSummary) {
  e.preventDefault();
  contextRow.value = row;
  showContextMenu.value = false;
  nextTick(() => {
    contextMenuX.value = e.clientX;
    contextMenuY.value = e.clientY;
    showContextMenu.value = true;
  });
}

function handleContextSelect(key: string) {
  showContextMenu.value = false;
  const row = contextRow.value;
  if (!row) return;
  switch (key) {
    case "copy-user": handleCopyUsername(row); break;
    case "copy": handleCopy(row); break;
    case "copy-totp": handleCopyTotp(row); break;
    case "copy-url": handleCopyUrl(row); break;
    case "reveal": revealPassword(row); break;
    case "edit": openEdit(row); break;
    case "open": handleOpenUrl(row); break;
    case "history": openHistory(row); break;
    case "delete": handleDelete(row); break;
  }
}

function rowProps(row: PasswordSummary) {
  return {
    onContextmenu: (e: MouseEvent) => onRowContextMenu(e, row),
  };
}

// --- 操作处理 ---

async function handleCopyUsername(row: PasswordSummary) {
  if (!row.username) return;
  try {
    await writeText(row.username);
    message.success("账号已复制到剪贴板");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

async function handleCopyUrl(row: PasswordSummary) {
  if (!row.url) return;
  try {
    await writeText(row.url);
    message.success("网址已复制到剪贴板");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

async function handleCopyTotp(row: PasswordSummary) {
  if (!row.has_totp) return;
  try {
    const code = await api.generateTotp(row.id);
    await copyAndScheduleClear(code.code);
    message.success(`动态码已复制（剩余 ${code.remaining_seconds}s）`);
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
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

function openAdd() {
  editEntry.value = null;
  showEditDialog.value = true;
}

async function openEdit(row: PasswordSummary) {
  try {
    editEntry.value = await api.getPassword(row.id);
    showEditDialog.value = true;
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  }
}

function openHistory(row: PasswordSummary) {
  historyTitle.value = row.title;
  historyEntryId.value = row.id;
  showHistoryDialog.value = true;
}

async function loadData() {
  loading.value = true;
  try {
    data.value = await api.listPasswords();
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

// --- 表格列定义 ---

const columns: DataTableColumns<PasswordSummary> = [
  {
    title: "名称",
    key: "title",
    width: 180,
    render(row) {
      const pwd = revealedPasswords.value.get(row.id);
      if (pwd) {
        return h('div', {}, [
          h('div', {}, row.title),
          h('div', { style: 'font-family: monospace; font-size: 12px; color: var(--n-text-color-2); margin-top: 2px' }, pwd),
        ]);
      }
      return row.title;
    },
  },
  { title: "分类", key: "role", width: 80 },
  {
    title: "用户名",
    key: "username",
    width: 160,
    render(row) {
      if (!row.username) return "";
      return h(
        'span',
        {
          class: 'username-copy',
          title: '点击复制用户名',
          onClick: (e: MouseEvent) => {
            e.stopPropagation();
            handleCopyUsername(row);
          },
        },
        [
          h('span', { class: 'username-text' }, row.username),
          h(NIcon, { size: 14, class: 'username-copy-icon' }, { default: () => h(CopyOutline) }),
        ]
      );
    },
  },
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
    width: 340,
    render(row) {
      return h(NSpace, { size: 0 }, {
        default: () => [
          h(NButton, { size: "small", quaternary: true, onClick: () => handleCopy(row) },
            { default: () => "复制", icon: () => h(NIcon, null, { default: () => h(CopyOutline) }) }),
          h(NButton, { size: "small", quaternary: true, type: revealedPasswords.value.has(row.id) ? 'warning' : 'default', onClick: () => revealPassword(row) },
            { default: () => revealedPasswords.value.has(row.id) ? "隐藏" : "显示", icon: () => h(NIcon, null, { default: () => h(EyeOutline) }) }),
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

    <!-- 空库欢迎页 -->
    <WelcomeWidget
      v-if="!loading && data.length === 0 && !searchQuery.trim() && selectedTags.length === 0"
      @add="openAdd"
    />
    <n-data-table
      v-else
      :columns="columns"
      :data="filtered"
      :loading="loading"
      :bordered="false"
      :pagination="{ pageSize: 20 }"
      :row-props="rowProps"
    />

    <!-- 右键上下文菜单 -->
    <n-dropdown
      placement="bottom-start"
      trigger="manual"
      :show="showContextMenu"
      :options="contextMenuOptions"
      :x="contextMenuX"
      :y="contextMenuY"
      @clickoutside="showContextMenu = false"
      @select="handleContextSelect"
    />

    <!-- 新增/编辑对话框 -->
    <PasswordEditDialog
      :show="showEditDialog"
      :edit-entry="editEntry"
      @update:show="showEditDialog = $event"
      @saved="loadData"
    />

    <!-- 历史密码对话框 -->
    <PasswordHistoryDialog
      :show="showHistoryDialog"
      :title="historyTitle"
      :entry-id="historyEntryId"
      @update:show="showHistoryDialog = $event"
      @rolled-back="loadData"
    />
    </div>
  </div>
</template>

<style scoped>
.pwd-page {
  display: flex;
  /* header 48px + content padding 16px*2 = 80px */
  height: calc(100vh - 80px);
  overflow: hidden;
}
.pwd-main {
  flex: 1;
  overflow-y: auto;
  padding: 0;
  min-height: 0;
}
/* 用户名单击复制 */
.username-copy {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  padding: 2px 6px;
  margin: -2px -6px;
  border-radius: 4px;
  transition: background-color 0.15s;
  max-width: 100%;
}
.username-copy:hover {
  background-color: var(--n-action-color, rgba(0, 0, 0, 0.05));
}
.username-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.username-copy-icon {
  opacity: 0;
  flex-shrink: 0;
  color: var(--n-text-color-3, #999);
  transition: opacity 0.15s;
}
.username-copy:hover .username-copy-icon {
  opacity: 1;
}
</style>
