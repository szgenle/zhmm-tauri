<script setup lang="ts">
/**
 * 分类管理视图
 *
 * 顶部按 role（分类，身份维度）切换；下方按 tags（用途维度）分组渲染多个列表。
 * - 同条目带多个标签时，会在每个相关分组里出现一次
 * - 没有标签的条目归入"未分类"分组
 * - 每个标签分组的可见列独立持久化（localStorage: zhmm_role_mgmt_columns_v1）
 * - 双击行进入编辑（复用 PasswordEditDialog）
 */
import { computed, h, onMounted, ref } from "vue";
import { useMessage } from "naive-ui";
import { SettingsOutline } from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  formatUtime,
  type PasswordEntry,
  type PasswordSummary,
} from "../api";
import PasswordEditDialog from "../components/PasswordEditDialog.vue";
import TotpCell from "../components/TotpCell.vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";

const message = useMessage();

const data = ref<PasswordSummary[]>([]);
const loading = ref(false);
const selectedRole = ref<string>(""); // "" = 全部

const UNTAGGED_KEY = "__untagged__";
const UNTAGGED_LABEL = "未分类";

// --- 列配置 ---

interface ColumnConfig {
  key: string;
  label: string;
  fixed?: boolean; // fixed 列不可隐藏
}

const allColumnConfigs: ColumnConfig[] = [
  { key: "role", label: "分类" },
  { key: "userID", label: "账号", fixed: true },
  { key: "url", label: "网址" },
  { key: "email", label: "邮箱" },
  { key: "phone", label: "手机" },
  { key: "desc", label: "备注" },
  { key: "totp", label: "2FA" },
  { key: "utime", label: "更新时间" },
];

const STORAGE_KEY = "zhmm_role_mgmt_columns_v1";
const DEFAULT_VISIBLE_KEYS = ["userID", "url", "desc"];

function loadGroupColumnPrefs(): Record<string, string[]> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const obj = JSON.parse(stored);
      if (obj && typeof obj === "object" && !Array.isArray(obj)) return obj;
    }
  } catch {
    /* ignore */
  }
  return {};
}

const groupColumnPrefs = ref<Record<string, string[]>>(loadGroupColumnPrefs());

function getVisibleKeysFor(tagKey: string): string[] {
  return groupColumnPrefs.value[tagKey] ?? [...DEFAULT_VISIBLE_KEYS];
}

function toggleColumnFor(tagKey: string, colKey: string) {
  const cfg = allColumnConfigs.find((c) => c.key === colKey);
  if (cfg?.fixed) return;
  const cur = [...getVisibleKeysFor(tagKey)];
  const idx = cur.indexOf(colKey);
  if (idx >= 0) cur.splice(idx, 1);
  else cur.push(colKey);
  groupColumnPrefs.value = { ...groupColumnPrefs.value, [tagKey]: cur };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(groupColumnPrefs.value));
}

// --- role 选项 ---

const roleOptions = computed<{ label: string; value: string }[]>(() => {
  const set = new Set<string>();
  for (const e of data.value) {
    if (e.role) set.add(e.role);
  }
  const list: { label: string; value: string }[] = [{ label: "全部", value: "" }];
  // 稳定排序：按字母序
  const sorted = [...set].sort((a, b) => a.localeCompare(b));
  for (const r of sorted) list.push({ label: r, value: r });
  return list;
});

const filteredByRole = computed<PasswordSummary[]>(() => {
  if (!selectedRole.value) return data.value;
  return data.value.filter((e) => e.role === selectedRole.value);
});

// --- 联系方式概览（手机/邮箱） ---

interface ContactItem {
  value: string; // 手机号或邮箱
  labels: string[]; // 去重后该 value 关联的所有账号显示名
  rows: PasswordSummary[]; // 关联原始记录，双击取第一条进入编辑
}

function aggregateContacts(
  pick: (e: PasswordSummary) => string
): ContactItem[] {
  const map = new Map<string, ContactItem>();
  for (const e of filteredByRole.value) {
    const v = (pick(e) || "").trim();
    if (!v) continue;
    const exist = map.get(v);
    const label = displayName(e);
    if (exist) {
      if (!exist.labels.includes(label)) exist.labels.push(label);
      exist.rows.push(e);
    } else {
      map.set(v, { value: v, labels: [label], rows: [e] });
    }
  }
  return [...map.values()].sort((a, b) => a.value.localeCompare(b.value));
}

const phoneList = computed<ContactItem[]>(() => aggregateContacts((e) => e.phone));
const emailList = computed<ContactItem[]>(() => aggregateContacts((e) => e.email));

async function copyContact(value: string, kind: "手机号" | "邮箱") {
  if (!value) return;
  try {
    await writeText(value);
    message.success(`${kind}已复制：${value}`);
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

// --- 按 tag 分组 ---

interface TagGroup {
  key: string;
  label: string;
  entries: PasswordSummary[];
}

const groups = computed<TagGroup[]>(() => {
  const tagMap = new Map<string, PasswordSummary[]>();
  const untagged: PasswordSummary[] = [];
  for (const e of filteredByRole.value) {
    const tags = (e.tags || []).filter(Boolean);
    if (tags.length === 0) {
      untagged.push(e);
      continue;
    }
    for (const t of tags) {
      if (!tagMap.has(t)) tagMap.set(t, []);
      tagMap.get(t)!.push(e);
    }
  }
  // 频次倒序 + 字母序稳定
  const sorted = [...tagMap.entries()].sort((a, b) => {
    if (b[1].length !== a[1].length) return b[1].length - a[1].length;
    return a[0].localeCompare(b[0]);
  });
  const result: TagGroup[] = sorted.map(([tag, entries]) => ({
    key: tag,
    label: `#${tag}`,
    entries,
  }));
  if (untagged.length) {
    result.push({ key: UNTAGGED_KEY, label: UNTAGGED_LABEL, entries: untagged });
  }
  return result;
});

// --- 行操作 ---

function displayName(row: PasswordSummary): string {
  if (row.userID) return row.userID;
  if (row.url) return row.url;
  return `#${row.id}`;
}

async function handleCopyUsername(row: PasswordSummary) {
  if (!row.userID) return;
  try {
    await writeText(row.userID);
    message.success("账号已复制到剪贴板");
  } catch (e: any) {
    message.error(`复制失败: ${e}`);
  }
}

async function handleOpenUrl(row: PasswordSummary) {
  if (!row.url) return;
  try {
    const raw = row.url.trim();
    const normalized = /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(raw) ? raw : `https://${raw}`;
    await openUrl(normalized);
  } catch (e: any) {
    message.error(`打开失败: ${e}`);
  }
}

// --- 列定义 ---

function buildColumns(visibleKeys: string[]): DataTableColumns<PasswordSummary> {
  const all: DataTableColumns<PasswordSummary> = [
    { title: "分类", key: "role", width: 80 },
    {
      title: "账号",
      key: "userID",
      width: 200,
      render(row) {
        return h(
          "span",
          {
            style: "cursor: pointer;",
            title: "点击复制账号",
            onClick: (e: MouseEvent) => {
              e.stopPropagation();
              handleCopyUsername(row);
            },
          },
          displayName(row)
        );
      },
    },
    {
      title: "网址",
      key: "url",
      width: 220,
      ellipsis: { tooltip: true },
      render(row) {
        if (!row.url) return "";
        return h(
          "span",
          {
            style: "cursor: pointer; color: var(--n-color-primary, #18a058);",
            title: "点击打开",
            onClick: (e: MouseEvent) => {
              e.stopPropagation();
              handleOpenUrl(row);
            },
          },
          row.url
        );
      },
    },
    { title: "邮箱", key: "email", width: 180, ellipsis: { tooltip: true } },
    { title: "手机", key: "phone", width: 130, ellipsis: { tooltip: true } },
    { title: "备注", key: "desc", ellipsis: { tooltip: true } },
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
      key: "utime",
      width: 170,
      render: (row) => formatUtime(row.utime),
    },
  ];
  return all.filter((c: any) => visibleKeys.includes(c.key));
}

function rowProps(row: PasswordSummary) {
  return {
    onDblclick: (e: MouseEvent) => {
      const sel = window.getSelection();
      if (sel) sel.removeAllRanges();
      e.preventDefault();
      openEdit(row);
    },
    style: "cursor: default; user-select: none;",
  };
}

// --- 编辑对话框 ---

const showEditDialog = ref(false);
const editEntry = ref<PasswordEntry | null>(null);

async function openEdit(row: PasswordSummary) {
  try {
    editEntry.value = await api.getPassword(row.id);
    showEditDialog.value = true;
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  }
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

onMounted(loadData);
</script>

<template>
  <div class="role-mgmt">
    <div class="toolbar">
      <n-radio-group v-model:value="selectedRole" size="small">
        <n-radio-button
          v-for="opt in roleOptions"
          :key="opt.value"
          :value="opt.value"
        >
          {{ opt.label }}
        </n-radio-button>
      </n-radio-group>
      <span class="hint">按标签分组浏览（双击进入编辑）</span>
    </div>

    <div
      v-if="phoneList.length || emailList.length"
      class="contact-overview"
    >
      <div class="contact-card">
        <div class="group-header">
          <span class="group-title">📱 手机</span>
          <span class="group-count">{{ phoneList.length }} 个</span>
        </div>
        <div v-if="phoneList.length" class="contact-list">
          <div
            v-for="item in phoneList"
            :key="`p-${item.value}`"
            class="contact-row"
            @click="copyContact(item.value, '手机号')"
            @dblclick="openEdit(item.rows[0])"
            :title="`点击复制，双击编辑账号${item.labels.length > 1 ? `\n共 ${item.labels.length} 个账号使用：${item.labels.join('、')}` : ''}`"
          >
            <span class="contact-value">{{ item.value }}</span>
            <span class="contact-label">
              {{ item.labels[0] }}<span
                v-if="item.labels.length > 1"
                class="contact-extra"
              >
                +{{ item.labels.length - 1 }}
              </span>
            </span>
          </div>
        </div>
        <div v-else class="contact-empty">无手机号记录</div>
      </div>
      <div class="contact-card">
        <div class="group-header">
          <span class="group-title">✉️ 邮箱</span>
          <span class="group-count">{{ emailList.length }} 个</span>
        </div>
        <div v-if="emailList.length" class="contact-list">
          <div
            v-for="item in emailList"
            :key="`e-${item.value}`"
            class="contact-row"
            @click="copyContact(item.value, '邮箱')"
            @dblclick="openEdit(item.rows[0])"
            :title="`点击复制，双击编辑账号${item.labels.length > 1 ? `\n共 ${item.labels.length} 个账号使用：${item.labels.join('、')}` : ''}`"
          >
            <span class="contact-value">{{ item.value }}</span>
            <span class="contact-label">
              {{ item.labels[0] }}<span
                v-if="item.labels.length > 1"
                class="contact-extra"
              >
                +{{ item.labels.length - 1 }}
              </span>
            </span>
          </div>
        </div>
        <div v-else class="contact-empty">无邮箱记录</div>
      </div>
    </div>

    <div v-if="groups.length" class="groups">
      <div v-for="g in groups" :key="g.key" class="group-card">
        <div class="group-header">
          <span class="group-title">{{ g.label }}</span>
          <span class="group-count">{{ g.entries.length }} 条</span>
          <n-popover trigger="click" placement="bottom-end">
            <template #trigger>
              <n-button quaternary circle size="small" title="列设置">
                <template #icon>
                  <n-icon><SettingsOutline /></n-icon>
                </template>
              </n-button>
            </template>
            <div style="min-width: 120px">
              <div
                v-for="cfg in allColumnConfigs"
                :key="cfg.key"
                style="padding: 4px 0"
              >
                <n-checkbox
                  :checked="getVisibleKeysFor(g.key).includes(cfg.key)"
                  :disabled="cfg.fixed"
                  @update:checked="toggleColumnFor(g.key, cfg.key)"
                >
                  {{ cfg.label }}
                </n-checkbox>
              </div>
            </div>
          </n-popover>
        </div>
        <n-data-table
          :columns="buildColumns(getVisibleKeysFor(g.key))"
          :data="g.entries"
          :bordered="false"
          size="small"
          :row-props="rowProps"
        />
      </div>
    </div>

    <n-empty
      v-else-if="!loading"
      description="当前分类下暂无数据"
      style="padding: 60px 0"
    />

    <PasswordEditDialog
      :show="showEditDialog"
      :edit-entry="editEntry"
      @update:show="showEditDialog = $event"
      @saved="loadData"
    />
  </div>
</template>

<style scoped>
.role-mgmt {
  height: calc(100vh - 92px);
  overflow-y: auto;
  padding: 0 4px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  margin-bottom: 16px;
  background: var(--app-card-bg);
  border: 1px solid var(--app-card-border);
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm);
  backdrop-filter: blur(8px);
  position: sticky;
  top: 0;
  z-index: 10;
}

.toolbar .hint {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  opacity: 0.8;
}

.groups {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.contact-overview {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  margin-bottom: 14px;
}

.contact-card {
  background: var(--app-card-bg);
  border: 1px solid var(--app-card-border);
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm);
  backdrop-filter: blur(8px);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.contact-list {
  max-height: 220px;
  overflow-y: auto;
  padding: 4px 0;
}

.contact-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  user-select: none;
  transition: background 0.15s ease;
}

.contact-row:hover {
  background: var(--n-color-target, rgba(0, 0, 0, 0.04));
}

.contact-value {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.contact-label {
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 50%;
}

.contact-extra {
  margin-left: 4px;
  padding: 0 6px;
  border-radius: 8px;
  background: var(--app-border-color, rgba(0, 0, 0, 0.08));
  font-size: 11px;
}

.contact-empty {
  padding: 18px 14px;
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  text-align: center;
}

@media (max-width: 720px) {
  .contact-overview {
    grid-template-columns: 1fr;
  }
}

.group-card {
  background: var(--app-card-bg);
  border: 1px solid var(--app-card-border);
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm);
  backdrop-filter: blur(8px);
  overflow: hidden;
  transition: box-shadow 0.25s ease;
}

.group-card:hover {
  box-shadow: var(--app-shadow-md);
}

.group-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--app-border-color);
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.group-count {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  margin-right: auto;
}
</style>
