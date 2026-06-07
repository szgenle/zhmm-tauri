<script setup lang="ts">
/**
 * 分类管理视图
 *
 * 顶部按 role（分类，身份维度）切换；下方按 tags（用途维度）分组渲染多个列表。
 * - 默认不展开标签分组：顶部显示标签芯片栏，最近查看的排前面
 * - 点击标签芯片后才展示对应分组内容
 * - 若分组内所有条目使用同一模板，自动并排显示模板字段
 * - 双击行进入编辑（复用 PasswordEditDialog）
 */
import { computed, h, onMounted, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import { SettingsOutline, CloseCircleOutline } from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  formatUtime,
  type AccountTemplate,
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

// --- 模板数据 ---
const templateMap = ref<Map<string, AccountTemplate>>(new Map());

async function loadTemplates() {
  try {
    const list = await api.listTemplates();
    const map = new Map<string, AccountTemplate>();
    for (const t of list) map.set(t.id, t);
    templateMap.value = map;
  } catch { /* 静默 */ }
}

// --- 标签展开状态 ---
const expandedTags = ref<Set<string>>(new Set());

// 最近查看标签（localStorage 持久化）
const RECENT_TAGS_KEY = "ajot_role_mgmt_recent_tags";
const MAX_RECENT_TAGS = 50;

function loadRecentTags(): string[] {
  try {
    const stored = localStorage.getItem(RECENT_TAGS_KEY);
    if (stored) return JSON.parse(stored);
  } catch { /* ignore */ }
  return [];
}

const recentTags = ref<string[]>(loadRecentTags());

function touchTag(tag: string) {
  const list = recentTags.value.filter((t) => t !== tag);
  list.unshift(tag);
  if (list.length > MAX_RECENT_TAGS) list.length = MAX_RECENT_TAGS;
  recentTags.value = list;
  localStorage.setItem(RECENT_TAGS_KEY, JSON.stringify(list));
}

function toggleTagExpand(tag: string) {
  const s = new Set(expandedTags.value);
  if (s.has(tag)) {
    s.delete(tag);
  } else {
    s.add(tag);
    touchTag(tag); // 记录最近查看
  }
  expandedTags.value = s;
}

function collapseTag(tag: string) {
  const s = new Set(expandedTags.value);
  s.delete(tag);
  expandedTags.value = s;
}

// --- 列配置 ---

interface ColumnConfig {
  key: string;
  label: string;
  fixed?: boolean; // fixed 列不可隐藏
}

const allColumnConfigs: ColumnConfig[] = [
  { key: "role", label: "分类" },
  { key: "name", label: "名称" },
  { key: "userID", label: "账号", fixed: true },
  { key: "url", label: "网址" },
  { key: "email", label: "邮箱" },
  { key: "phone", label: "手机" },
  { key: "desc", label: "备注" },
  { key: "totp", label: "2FA" },
  { key: "utime", label: "更新时间" },
];

const STORAGE_KEY = "ajot_role_mgmt_columns_v2";
const DEFAULT_VISIBLE_KEYS = ["name", "userID", "url", "desc"];

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
  /** 展开时显示的全部记录：所有 tags 含该 tag 的记录（无论位置） */
  entries: PasswordSummary[];
  /** chip 上显示的计数：仅按记录的第一个标签（一级标签）聚合 */
  primaryCount: number;
  /** 若分组内所有条目使用同一模板，记录该模板 */
  soleTemplate?: AccountTemplate;
}

/**
 * 所有标签分组：
 * - chip 出现条件 + count：仅按 tags[0]（一级标签）聚合
 * - 分组展开内容：包含所有 tags 含该标签的记录（一级或细分位置均可）
 */
const allTagGroups = computed<TagGroup[]>(() => {
  // 1) 仅按 primary 计数，决定哪些 tag 作为 chip 出现
  const primaryCount = new Map<string, number>();
  let untaggedCount = 0;
  for (const e of filteredByRole.value) {
    const primary = (e.tags || []).find(Boolean);
    if (!primary) {
      untaggedCount += 1;
      continue;
    }
    primaryCount.set(primary, (primaryCount.get(primary) || 0) + 1);
  }
  // 2) 展开内容：每条记录可在多个 tag 组中出现（用于 chip 选中后展示全部相关）
  const fullEntries = new Map<string, PasswordSummary[]>();
  const untagged: PasswordSummary[] = [];
  for (const e of filteredByRole.value) {
    const tags = (e.tags || []).filter(Boolean);
    if (tags.length === 0) {
      untagged.push(e);
      continue;
    }
    for (const t of tags) {
      // 仅为 primary 出现过的 tag 建组（与 chip 列表对齐）
      if (!primaryCount.has(t)) continue;
      if (!fullEntries.has(t)) fullEntries.set(t, []);
      fullEntries.get(t)!.push(e);
    }
  }
  const result: TagGroup[] = [];
  for (const [tag, count] of primaryCount.entries()) {
    const entries = fullEntries.get(tag) || [];
    const sole = detectSoleTemplate(entries);
    result.push({
      key: tag,
      label: `#${tag}`,
      entries,
      primaryCount: count,
      soleTemplate: sole,
    });
  }
  if (untaggedCount) {
    const sole = detectSoleTemplate(untagged);
    result.push({
      key: UNTAGGED_KEY,
      label: UNTAGGED_LABEL,
      entries: untagged,
      primaryCount: untaggedCount,
      soleTemplate: sole,
    });
  }
  return result;
});

/** 检测分组内是否只使用一种模板（忽略无模板的条目） */
function detectSoleTemplate(entries: PasswordSummary[]): AccountTemplate | undefined {
  if (entries.length === 0) return undefined;
  // 收集所有非空 template_id
  const ids = new Set<string>();
  for (const e of entries) {
    if (e.template_id) ids.add(e.template_id);
  }
  // 只有恰好一种模板时才返回
  if (ids.size !== 1) return undefined;
  const [soleId] = ids;
  return templateMap.value.get(soleId);
}

/** 标签芯片列表（最近查看的排前面），count 按一级聚合显示 */
const sortedTagChips = computed<{ key: string; label: string; count: number; expanded: boolean }[]>(() => {
  const groups = allTagGroups.value;
  const recentOrder = recentTags.value;
  // 建立 tag -> group 映射
  const groupMap = new Map(groups.map((g) => [g.key, g]));
  const result: { key: string; label: string; count: number; expanded: boolean }[] = [];
  const added = new Set<string>();
  // 先按最近顺序
  for (const tag of recentOrder) {
    const g = groupMap.get(tag);
    if (g && !added.has(tag)) {
      result.push({ key: g.key, label: g.label, count: g.primaryCount, expanded: expandedTags.value.has(g.key) });
      added.add(tag);
    }
  }
  // 再按 primary 计数倒序补充未出现过的
  const remaining = groups.filter((g) => !added.has(g.key)).sort((a, b) => {
    if (b.primaryCount !== a.primaryCount) return b.primaryCount - a.primaryCount;
    return a.key.localeCompare(b.key);
  });
  for (const g of remaining) {
    result.push({ key: g.key, label: g.label, count: g.primaryCount, expanded: expandedTags.value.has(g.key) });
  }
  return result;
});

/** 当前展开的分组（仅已选中的标签） */
const visibleGroups = computed<TagGroup[]>(() => {
  return allTagGroups.value.filter((g) => expandedTags.value.has(g.key));
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

function buildColumns(visibleKeys: string[], tpl?: AccountTemplate): DataTableColumns<PasswordSummary> {
  const all: DataTableColumns<PasswordSummary> = [
    { title: "分类", key: "role", width: 80 },
    { title: "名称", key: "name", width: 160, ellipsis: { tooltip: true } },
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
  let cols = all.filter((c: any) => visibleKeys.includes(c.key));
  // 若分组使用单一模板，追加模板字段列
  if (tpl && tpl.fields.length) {
    const tplCols: DataTableColumns<PasswordSummary> = tpl.fields.map((f) => ({
      title: f.label,
      key: `_tpl_${f.key}`,
      width: 140,
      ellipsis: { tooltip: true },
      render(row: PasswordSummary) {
        // 无模板关联的条目直接显示空
        if (!row.template_id) return "";
        // 需要从缓存中读取扩展字段
        const cached = customFieldsCache.value.get(row.id);
        if (cached === undefined) return h("span", { style: "color: var(--n-text-color-3); font-size: 12px;" }, "…");
        const val = cached[f.key] || "";
        if (f.field_type === "secret" && val) return "••••";
        return val;
      },
    }));
    cols = [...cols, ...tplCols];
  }
  return cols;
}

// --- 扩展字段缓存（用于模板列渲染） ---
const customFieldsCache = ref<Map<number, Record<string, string>>>(new Map());

async function loadCustomFieldsForGroup(entries: PasswordSummary[], tpl?: AccountTemplate) {
  // 加载有 custom_fields 或有模板关联的条目（模板字段可能已填写但 has_custom_fields 未标记）
  const toLoad = entries.filter(
    (e) => !customFieldsCache.value.has(e.id) && (e.has_custom_fields || (tpl && e.template_id))
  );
  if (!toLoad.length) return;
  // 并发加载（限制并发数避免爆栈）
  const BATCH = 10;
  for (let i = 0; i < toLoad.length; i += BATCH) {
    const batch = toLoad.slice(i, i + BATCH);
    const results = await Promise.all(
      batch.map((e) => api.getPassword(e.id).catch(() => null))
    );
    const newMap = new Map(customFieldsCache.value);
    for (let j = 0; j < batch.length; j++) {
      const entry = results[j];
      // 无论是否有 custom_fields 都缓存，避免反复请求
      newMap.set(batch[j].id, entry?.custom_fields ?? {});
    }
    customFieldsCache.value = newMap;
  }
}

// 当展开的分组变化时，加载对应的扩展字段
watch(visibleGroups, (groups) => {
  for (const g of groups) {
    if (g.soleTemplate) {
      loadCustomFieldsForGroup(g.entries, g.soleTemplate);
    }
  }
}, { immediate: true });

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

onMounted(async () => {
  await Promise.all([loadData(), loadTemplates()]);
});
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
      <span class="hint">点击标签查看分组（双击进入编辑）</span>
    </div>

    <!-- 标签芯片栏 -->
    <div v-if="sortedTagChips.length" class="tag-chips-bar">
      <div class="tag-chips-scroll">
        <span
          v-for="chip in sortedTagChips"
          :key="chip.key"
          class="tag-chip"
          :class="{ active: chip.expanded }"
          @click="toggleTagExpand(chip.key)"
        >
          {{ chip.label }}
          <span class="chip-count">{{ chip.count }}</span>
          <span v-if="chip.expanded" class="chip-close" @click.stop="collapseTag(chip.key)">×</span>
        </span>
      </div>
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

    <div v-if="visibleGroups.length" class="groups">
      <div v-for="g in visibleGroups" :key="g.key" class="group-card">
        <div class="group-header">
          <span class="group-title">{{ g.label }}</span>
          <span class="group-count">{{ g.entries.length }} 条</span>
          <span v-if="g.soleTemplate" class="group-tpl-badge">📋 {{ g.soleTemplate.name }}</span>
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
          <n-button text size="tiny" @click="collapseTag(g.key)" title="收起">
            <template #icon>
              <n-icon><CloseCircleOutline /></n-icon>
            </template>
          </n-button>
        </div>
        <n-data-table
          :columns="buildColumns(getVisibleKeysFor(g.key), g.soleTemplate)"
          :data="g.entries"
          :bordered="false"
          size="small"
          :row-props="rowProps"
        />
      </div>
    </div>

    <n-empty
      v-else-if="!loading && !sortedTagChips.length"
      description="当前分类下暂无数据"
      style="padding: 60px 0"
    />
    <div v-else-if="!visibleGroups.length && sortedTagChips.length" class="expand-hint">
      <span>👆 点击上方标签查看对应分组内容</span>
    </div>

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
  height: 100%;
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

.tag-chips-bar {
  margin-bottom: 14px;
  padding: 10px 14px;
  background: var(--app-card-bg);
  border: 1px solid var(--app-card-border);
  border-radius: 10px;
  box-shadow: var(--app-shadow-sm);
  backdrop-filter: blur(8px);
}

.tag-chips-scroll {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  border-radius: 16px;
  font-size: 13px;
  cursor: pointer;
  user-select: none;
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
  border: 1px solid transparent;
  transition: all 0.2s ease;
}

.tag-chip:hover {
  background: var(--n-color-pressed, rgba(0, 0, 0, 0.08));
  transform: translateY(-1px);
}

.tag-chip.active {
  background: var(--n-color-primary, #18a058);
  color: #fff;
  border-color: var(--n-color-primary, #18a058);
  box-shadow: 0 2px 8px rgba(24, 160, 88, 0.25);
}

.chip-count {
  font-size: 11px;
  opacity: 0.7;
  margin-left: 2px;
}

.tag-chip.active .chip-count {
  opacity: 0.85;
}

.chip-close {
  margin-left: 2px;
  font-size: 14px;
  line-height: 1;
  opacity: 0.7;
  cursor: pointer;
}

.chip-close:hover {
  opacity: 1;
}

.expand-hint {
  text-align: center;
  padding: 48px 0;
  color: var(--n-text-color-3, #999);
  font-size: 14px;
}

.group-tpl-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
  color: var(--n-text-color-2, #666);
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
