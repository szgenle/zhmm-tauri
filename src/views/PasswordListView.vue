<script setup lang="ts">
import { computed, h, nextTick, onMounted, ref } from "vue";
import { NButton, NIcon, NInput, NTag, NSpace, NCheckbox, NPopover, useMessage, useDialog } from "naive-ui";
import {
  AddOutline,
  CopyOutline,
  EyeOutline,
  SearchOutline,
  SettingsOutline,
} from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  formatUtime,
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
const selectedRole = ref("");  // 空字符串表示"全部"
const data = ref<PasswordSummary[]>([]);
const loading = ref(false);
const selectedTags = ref<string[]>([]);

/** 从当前数据中提取所有分类，用于下拉选项 */
const roleOptions = computed(() => {
  const roles = new Set<string>();
  for (const entry of data.value) {
    if (entry.role) roles.add(entry.role);
  }
  const opts = [{ label: "全部分类", value: "" }];
  for (const r of roles) {
    opts.push({ label: r, value: r });
  }
  return opts;
});

// 密码明文定时隐藏
const { revealedPasswords, revealPassword } = usePasswordReveal(message);

/**
 * 字符串规范化：去首尾空白 + NFKC + 转小写
 */
function normalize(s: string): string {
  return (s || "").trim().normalize("NFKC").toLowerCase();
}

/** 列表展示用的名称：账号 → 网址 → #id */
function displayName(row: PasswordSummary): string {
  if (row.userID) return row.userID;
  if (row.url) return row.url;
  return `#${row.id}`;
}

const filtered = computed<PasswordSummary[]>(() => {
  let result = data.value;
  // 按分类筛选
  if (selectedRole.value) {
    result = result.filter((row) => row.role === selectedRole.value);
  }
  // 按标签筛选
  if (selectedTags.value.length > 0) {
    result = result.filter((row) =>
      selectedTags.value.every((t) => (row.tags || []).includes(t))
    );
  }
  // 按搜索关键词筛选
  const q = normalize(searchQuery.value);
  if (!q) return result;
  return result.filter((row) => {
    const fields = [
      row.role, row.userID, row.phone, row.email, row.url, row.desc,
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
const historyEntryId = ref<number>(0);

// 右键菜单
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextRow = ref<PasswordSummary | null>(null);

const contextMenuOptions = computed(() => {
  const row = contextRow.value;
  if (!row) return [] as Array<{ label: string; key: string; disabled?: boolean; type?: string }>;
  const opts: Array<{ label?: string; key: string; disabled?: boolean; type?: string }> = [];
  opts.push({ label: "复制账号", key: "copy-user", disabled: !row.userID });
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
    onDblclick: (e: MouseEvent) => {
      // 避免双击选中文本
      const sel = window.getSelection();
      if (sel) sel.removeAllRanges();
      e.preventDefault();
      openEdit(row);
    },
    style: 'cursor: default; user-select: none;',
  };
}

// --- 操作处理 ---

async function handleCopyUsername(row: PasswordSummary) {
  if (!row.userID) return;
  try {
    await writeText(row.userID);
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
    await copyAndScheduleClear(entry.pwd);
    message.success("密码已复制到剪贴板");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

async function handleOpenUrl(row: PasswordSummary) {
  if (!row.url) return;
  try {
    const raw = row.url.trim();
    // opener 插件默认仅允许 http(s)/mailto/tel 等带协议的 URL，这里为缺少协议的网址自动补上 https://
    const normalized = /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(raw) ? raw : `https://${raw}`;
    await openUrl(normalized);
  } catch (e: any) {
    message.error(`打开失败: ${e}`);
  }
}

function handleDelete(row: PasswordSummary) {
  dialog.warning({
    title: "确认删除",
    content: `确定要删除"${displayName(row)}"吗？此操作不可恢复。`,
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
  historyTitle.value = displayName(row);
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

// --- 列可见性配置 ---

interface ColumnConfig {
  key: string;
  label: string;
  fixed?: boolean; // fixed 的列不允许隐藏
}

const allColumnConfigs: ColumnConfig[] = [
  { key: "role", label: "分类" },
  { key: "userID", label: "账号", fixed: true },
  { key: "url", label: "网址" },
  { key: "email", label: "邮箱" },
  { key: "phone", label: "手机" },
  { key: "tags", label: "标签" },
  { key: "desc", label: "备注" },
  { key: "totp", label: "2FA" },
  { key: "pwd_utime", label: "密码更新时间" },
  { key: "actions", label: "操作", fixed: true },
];

const STORAGE_KEY = "zhmm_visible_columns_v4";

// 默认勾选：分类、账号(fixed)、网址、邮箱、手机、标签、备注、操作(fixed)
// 2FA 和 密码更新时间 默认隐藏
const DEFAULT_VISIBLE_KEYS = [
  "role", "userID", "url", "email", "phone", "tags", "desc", "actions",
];

function loadVisibleColumns(): string[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const arr = JSON.parse(stored);
      if (Array.isArray(arr) && arr.length > 0) return arr;
    }
  } catch {}
  return [...DEFAULT_VISIBLE_KEYS];
}

const visibleColumnKeys = ref<string[]>(loadVisibleColumns());

function toggleColumn(key: string) {
  const cfg = allColumnConfigs.find((c) => c.key === key);
  if (cfg?.fixed) return;
  const idx = visibleColumnKeys.value.indexOf(key);
  if (idx >= 0) {
    visibleColumnKeys.value.splice(idx, 1);
  } else {
    visibleColumnKeys.value.push(key);
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(visibleColumnKeys.value));
}

// --- 表格列定义 ---

const allColumns: DataTableColumns<PasswordSummary> = [
  { title: "分类", key: "role", width: 80 },
  {
    title: "账号",
    key: "userID",
    width: 200,
    render(row) {
      const pwd = revealedPasswords.value.get(row.id);
      const name = displayName(row);
      const nameNode = h(
        'span',
        {
          class: 'username-copy',
          title: '点击复制账号',
          onClick: (e: MouseEvent) => {
            e.stopPropagation();
            handleCopyUsername(row);
          },
        },
        [
          h('span', { class: 'username-text' }, name),
          h(NIcon, { size: 14, class: 'username-copy-icon' }, { default: () => h(CopyOutline) }),
        ]
      );
      const children: any[] = [nameNode];
      if (pwd) {
        children.push(
          h('div', { style: 'font-family: monospace; font-size: 12px; color: var(--n-text-color-2); margin-top: 2px' }, pwd)
        );
      }
      if (row.has_totp) {
        children.push(
          h('div', { style: 'margin-top: 2px' }, [h(TotpCell, { id: row.id })])
        );
      }
      if (children.length === 1) return nameNode;
      return h('div', {}, children);
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
    title: "网址",
    key: "url",
    width: 200,
    ellipsis: { tooltip: true },
    render(row) {
      if (!row.url) return "";
      return h(
        'span',
        {
          style: 'cursor: pointer; color: var(--n-color-primary, #18a058);',
          title: '点击打开',
          onClick: (e: MouseEvent) => {
            e.stopPropagation();
            handleOpenUrl(row);
          },
        },
        row.url
      );
    },
  },
  {
    title: "邮箱",
    key: "email",
    width: 180,
    ellipsis: { tooltip: true },
  },
  {
    title: "手机",
    key: "phone",
    width: 130,
    ellipsis: { tooltip: true },
  },
  {
    title: "备注",
    key: "desc",
    ellipsis: { tooltip: true },
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
    title: "密码更新时间",
    key: "pwd_utime",
    width: 170,
    render: (row) => formatUtime(row.pwd_utime),
  },
  {
    title: "操作",
    key: "actions",
    width: 150,
    render(row) {
      return h(NSpace, { size: 4, wrap: false, wrapItem: false }, {
        default: () => [
          h(NButton, { size: "small", quaternary: true, onClick: () => handleCopy(row) },
            { default: () => "复制", icon: () => h(NIcon, null, { default: () => h(CopyOutline) }) }),
          h(NButton, { size: "small", quaternary: true, type: revealedPasswords.value.has(row.id) ? 'warning' : 'default', onClick: () => revealPassword(row) },
            { default: () => revealedPasswords.value.has(row.id) ? "隐藏" : "显示", icon: () => h(NIcon, null, { default: () => h(EyeOutline) }) }),
        ],
      });
    },
  },
];

const columns = computed<DataTableColumns<PasswordSummary>>(() => {
  return allColumns.filter((col: any) => visibleColumnKeys.value.includes(col.key));
});

const searchInputRef = ref<InstanceType<typeof NInput> | null>(null);

onMounted(async () => {
  await loadData();
  nextTick(() => {
    searchInputRef.value?.focus();
  });
});
</script>

<template>
  <div class="pwd-page">
    <TagSidebar :entries="data" :selected-tags="selectedTags" @update:selected-tags="v => selectedTags = v" />
    <div class="pwd-main">
    <div class="toolbar">
      <div class="toolbar-left">
        <n-select
          v-model:value="selectedRole"
          :options="roleOptions"
          style="width: 130px"
        />
        <n-input
          ref="searchInputRef"
          v-model:value="searchQuery"
          placeholder="搜索账号..."
          clearable
          style="width: 260px"
        >
          <template #prefix>
            <n-icon :size="16" style="opacity: 0.5"><SearchOutline /></n-icon>
          </template>
        </n-input>
      </div>
      <div class="toolbar-right">
        <n-popover trigger="click" placement="bottom-end">
          <template #trigger>
            <n-button quaternary circle>
              <template #icon>
                <n-icon><SettingsOutline /></n-icon>
              </template>
            </n-button>
          </template>
          <div style="min-width: 120px">
            <div v-for="cfg in allColumnConfigs" :key="cfg.key" style="padding: 4px 0">
              <n-checkbox
                :checked="visibleColumnKeys.includes(cfg.key)"
                :disabled="cfg.fixed"
                @update:checked="toggleColumn(cfg.key)"
              >
                {{ cfg.label }}
              </n-checkbox>
            </div>
          </div>
        </n-popover>
        <n-button type="primary" @click="openAdd">
          <template #icon>
            <n-icon><AddOutline /></n-icon>
          </template>
          添加
        </n-button>
      </div>
    </div>

    <!-- 空库欢迎页 -->
    <WelcomeWidget
      v-if="!loading && data.length === 0 && !searchQuery.trim() && selectedTags.length === 0 && !selectedRole"
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
  /* header 52px + content padding 20px*2 = 92px */
  height: calc(100vh - 92px);
  overflow: hidden;
  gap: 0;
}
.pwd-main {
  flex: 1;
  overflow-y: auto;
  padding: 0;
  min-height: 0;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  padding: 10px 14px;
  background: var(--app-card-bg);
  border: 1px solid var(--app-border-color);
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm);
  backdrop-filter: blur(8px);
  gap: 12px;
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
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
