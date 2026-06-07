<script setup lang="ts">
/**
 * 账号模板管理视图（v2.0+）
 *
 * - 列表展示当前 vault 下所有 AccountTemplate
 * - 新建/编辑模板：可视化定义业务字段（key/label/type/placeholder）
 * - 删除模板（不会清理已引用条目的 template_id）
 * - 一键补装内建模板：旧 .zmb 库或被清空模板后用得上（幂等）
 *
 * 设计细节：
 * - 模板 id 一旦保存即不可改（影响条目里的 custom_fields key 关联）
 * - 字段 key 必须唯一、非空；保存时做最终校验
 * - icon 用单字符 emoji 即可，不强校验
 */
import { computed, h, onMounted, reactive, ref } from "vue";
import { useDialog, useMessage } from "naive-ui";
import {
  AddOutline,
  TrashOutline,
  ReloadOutline,
  DownloadOutline,
  CloudUploadOutline,
  GlobeOutline,
} from "@vicons/ionicons5";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { DataTableColumns } from "naive-ui";
import {
  api,
  type AccountTemplate,
  type TemplateField,
  type TemplateFieldType,
  type TemplateMatchRule,
} from "../api";
import { COMMUNITY_PACKS, type CommunityTemplatePack } from "../utils/communityTemplates";

const message = useMessage();
const dialog = useDialog();

const templates = ref<AccountTemplate[]>([]);
const loading = ref(false);

// ========== 编辑对话框 ==========

const showEditor = ref(false);
const editorMode = ref<"create" | "edit">("create");
const form = reactive<{
  id: string;
  name: string;
  icon: string;
  fields: TemplateField[];
  match_rules: TemplateMatchRule[];
}>({
  id: "",
  name: "",
  icon: "",
  fields: [],
  match_rules: [],
});

const matchRuleKindOptions: { label: string; value: TemplateMatchRule["kind"] }[] = [
  { label: "网址包含", value: "url_contains" },
  { label: "关键词", value: "keyword" },
  { label: "分类等于", value: "role" },
];

const fieldTypeOptions: { label: string; value: TemplateFieldType }[] = [
  { label: "单行文本", value: "text" },
  { label: "密文（隐藏显示）", value: "secret" },
  { label: "多行文本", value: "multiline" },
  { label: "网址", value: "url" },
  { label: "邮箱", value: "email" },
  { label: "电话", value: "phone" },
  { label: "日期", value: "date" },
];

function resetForm() {
  form.id = "";
  form.name = "";
  form.icon = "";
  form.fields = [];
  form.match_rules = [];
}

function openCreate() {
  editorMode.value = "create";
  resetForm();
  showEditor.value = true;
}

function openEdit(t: AccountTemplate) {
  editorMode.value = "edit";
  form.id = t.id;
  form.name = t.name;
  form.icon = t.icon || "";
  form.fields = t.fields.map((f) => ({
    key: f.key,
    label: f.label,
    field_type: f.field_type || "text",
    required: !!f.required,
    placeholder: f.placeholder || "",
  }));
  form.match_rules = (t.match_rules || []).map((r) => ({ ...r }));
  showEditor.value = true;
}

function addField() {
  form.fields.push({
    key: "",
    label: "",
    field_type: "text",
    required: false,
    placeholder: "",
  });
}

function removeField(idx: number) {
  form.fields.splice(idx, 1);
}

function moveField(idx: number, delta: number) {
  const j = idx + delta;
  if (j < 0 || j >= form.fields.length) return;
  const tmp = form.fields[idx];
  form.fields[idx] = form.fields[j];
  form.fields[j] = tmp;
}

function addMatchRule() {
  form.match_rules.push({ kind: "url_contains", value: "" });
}
function removeMatchRule(idx: number) {
  form.match_rules.splice(idx, 1);
}

function validateForm(): string | null {
  const id = form.id.trim();
  const name = form.name.trim();
  if (!id) return "模板 id 不能为空";
  if (!/^[A-Za-z0-9_\-]+$/.test(id))
    return "模板 id 仅支持字母、数字、下划线、短横线";
  if (!name) return "模板名称不能为空";
  const seen = new Set<string>();
  for (const f of form.fields) {
    const k = (f.key || "").trim();
    const lb = (f.label || "").trim();
    if (!k) return "存在未填写 key 的字段";
    if (!lb) return `字段「${k}」缺少显示名`;
    if (!/^[A-Za-z0-9_\-]+$/.test(k))
      return `字段 key「${k}」仅支持字母、数字、下划线、短横线`;
    if (seen.has(k)) return `字段 key「${k}」重复`;
    seen.add(k);
  }
  return null;
}

const saving = ref(false);
async function handleSave() {
  const err = validateForm();
  if (err) {
    message.error(err);
    return;
  }
  saving.value = true;
  try {
    const payload: AccountTemplate = {
      id: form.id.trim(),
      name: form.name.trim(),
      icon: form.icon.trim() || undefined,
      fields: form.fields.map((f) => ({
        key: f.key.trim(),
        label: f.label.trim(),
        field_type: f.field_type || "text",
        required: !!f.required,
        placeholder: f.placeholder?.trim() || undefined,
      })),
      match_rules: form.match_rules
        .map((r) => ({ kind: r.kind, value: (r.value || "").trim() } as TemplateMatchRule))
        .filter((r) => !!r.value),
    };
    await api.upsertTemplate(payload);
    message.success(editorMode.value === "create" ? "模板已创建" : "模板已更新");
    showEditor.value = false;
    await loadData();
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  } finally {
    saving.value = false;
  }
}

// ========== 列表操作 ==========

function handleDelete(t: AccountTemplate) {
  dialog.warning({
    title: "确认删除模板",
    content: `删除「${t.name}」？已使用此模板的账号条目会保留 custom_fields 数据，但不再显示对应字段（可重建同 id 模板恢复）。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.deleteTemplate(t.id);
        message.success("已删除");
        await loadData();
      } catch (e: any) {
        message.error(`删除失败: ${e}`);
      }
    },
  });
}

async function handleSeedDefaults() {
  try {
    const added = await api.seedDefaultTemplates();
    if (added > 0) {
      message.success(`已补装 ${added} 个内建模板`);
    } else {
      message.info("内建模板已齐全，无需补装");
    }
    await loadData();
  } catch (e: any) {
    message.error(`补装失败: ${e}`);
  }
}

// ========== 导入导出（alpha.3） ==========

async function handleExportAll() {
  if (templates.value.length === 0) {
    message.info("当前账号库未配置任何模板");
    return;
  }
  await exportTemplatesToFile(undefined, "templates");
}

async function handleExportOne(t: AccountTemplate) {
  await exportTemplatesToFile([t.id], `template-${t.id}`);
}

/** 双路复用的导出实现：ids 为 undefined 表示全部 */
async function exportTemplatesToFile(
  ids: string[] | undefined,
  defaultStem: string,
) {
  try {
    const path = await saveDialog({
      title: "导出模板包",
      defaultPath: `${defaultStem}.json`,
      filters: [{ name: "模板包 JSON", extensions: ["json"] }],
    });
    if (!path) return;
    const count = await api.exportTemplatesJson(path as string, ids);
    message.success(`已导出 ${count} 个模板到 ${path}`);
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  }
}

async function handleImport() {
  let path: string | null = null;
  try {
    const picked = await openDialog({
      title: "选择模板包 JSON 文件",
      multiple: false,
      filters: [{ name: "模板包 JSON", extensions: ["json"] }],
    });
    if (!picked) return;
    path = Array.isArray(picked) ? picked[0] : picked;
  } catch (e: any) {
    message.error(`打开文件失败: ${e}`);
    return;
  }
  if (!path) return;
  const filePath = path;

  dialog.create({
    title: "导入模板包",
    content: "遇到 id 冲突的模板，你希望如何处理？",
    positiveText: "仅新增（安全）",
    negativeText: "覆盖现有",
    onPositiveClick: () => doImport(filePath, false),
    onNegativeClick: () => doImport(filePath, true),
  });
}

async function doImport(path: string, overwrite: boolean) {
  try {
    const r = await api.importTemplatesJson(path, overwrite);
    const parts: string[] = [];
    if (r.added) parts.push(`新增 ${r.added}`);
    if (r.updated) parts.push(`覆盖 ${r.updated}`);
    if (r.skipped) parts.push(`跳过 ${r.skipped}`);
    if (r.invalid) parts.push(`丢弃 ${r.invalid}`);
    message.success(`导入完成：${parts.join(" / ") || "无变化"}`);
    await loadData();
  } catch (e: any) {
    message.error(`导入失败: ${e}`);
  }
}

// ========== 社区预设包 ==========

const showCommunity = ref(false);
const communityPacks = ref<CommunityTemplatePack[]>(COMMUNITY_PACKS);
/** 记录正在装入中的包 id */
const installingPackId = ref<string | null>(null);
/** 记录已成功装入的包 id（用于短暂显示“已装入”状态） */
const installedPackIds = ref<Set<string>>(new Set());

/** 从社区预设包一键安装：逐个 upsert。实现上等价于 overwrite=true */
async function installCommunityPack(pack: CommunityTemplatePack) {
  if (installingPackId.value) return;
  installingPackId.value = pack.id;
  let added = 0;
  let updated = 0;
  const existingIds = new Set(templates.value.map((t) => t.id));
  for (const t of pack.templates) {
    try {
      await api.upsertTemplate(t);
      if (existingIds.has(t.id)) updated++;
      else added++;
    } catch (e: any) {
      message.error(`模板「${t.name}」安装失败: ${e}`);
    }
  }
  installingPackId.value = null;
  installedPackIds.value.add(pack.id);
  message.success(
    `「${pack.name}」已装入：新增 ${added}、更新 ${updated}（共 ${pack.templates.length}）`,
  );
  await loadData();
}

async function loadData() {
  loading.value = true;
  try {
    templates.value = await api.listTemplates();
  } catch (e: any) {
    message.error(`加载失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

onMounted(loadData);

// ========== 列定义 ==========

const columns: DataTableColumns<AccountTemplate> = [
  {
    title: "图标",
    key: "icon",
    width: 60,
    render: (row) => row.icon || "",
  },
  {
    title: "名称",
    key: "name",
    width: 160,
  },
  {
    title: "id",
    key: "id",
    width: 160,
    render: (row) =>
      h(
        "span",
        {
          style:
            "font-family: ui-monospace, monospace; font-size: 12px; color: var(--n-text-color-3);",
        },
        row.id,
      ),
  },
  {
    title: "字段",
    key: "fields",
    render(row) {
      if (!row.fields?.length)
        return h(
          "span",
          { style: "color: var(--n-text-color-3); font-size: 12px;" },
          "无字段",
        );
      return h(
        "div",
        { style: "display: flex; gap: 6px; flex-wrap: wrap;" },
        row.fields.map((f) =>
          h(
            "span",
            {
              style:
                "padding: 1px 8px; border-radius: 8px; background: var(--app-border-color, rgba(0,0,0,0.06)); font-size: 12px;",
              title: `${f.key} · ${f.field_type || "text"}`,
            },
            f.label,
          ),
        ),
      );
    },
  },
  {
    title: "推荐规则",
    key: "match_rules",
    width: 90,
    render(row) {
      const n = row.match_rules?.length || 0;
      if (n === 0)
        return h(
          "span",
          { style: "color: var(--n-text-color-3); font-size: 12px;" },
          "—",
        );
      return h(
        "span",
        {
          style:
            "padding: 1px 8px; border-radius: 8px; background: var(--app-border-color, rgba(0,0,0,0.06)); font-size: 12px;",
          title: "命中任一条即推荐应用此模板",
        },
        `${n} 条`,
      );
    },
  },
  {
    title: "操作",
    key: "actions",
    width: 200,
    render(row) {
      return h("div", { style: "display: flex; gap: 6px;" }, [
        h(
          "button",
          {
            class: "tpl-act tpl-act-edit",
            title: "编辑",
            onClick: () => openEdit(row),
          },
          "编辑",
        ),
        h(
          "button",
          {
            class: "tpl-act",
            title: "导出为 JSON 模板包",
            onClick: () => handleExportOne(row),
          },
          "导出",
        ),
        h(
          "button",
          {
            class: "tpl-act tpl-act-del",
            title: "删除",
            onClick: () => handleDelete(row),
          },
          "删除",
        ),
      ]);
    },
  },
];

const builtinIds = computed(() =>
  new Set(["bank_card", "id_card", "work_internal", "game"]),
);
const hasAllBuiltins = computed(() => {
  const ids = new Set(templates.value.map((t) => t.id));
  for (const b of builtinIds.value) if (!ids.has(b)) return false;
  return true;
});
</script>

<template>
  <div class="tpl-mgmt">
    <div class="toolbar">
      <div class="toolbar-left">
        <h3 class="title">账号模板</h3>
        <span class="hint">为不同类型账号定义业务字段，编辑账号时按模板动态渲染</span>
      </div>
      <div class="toolbar-right">
        <n-button
          v-if="!hasAllBuiltins"
          quaternary
          @click="handleSeedDefaults"
          title="补装内建的 4 个模板（已存在的不会覆盖）"
        >
          <template #icon>
            <n-icon><ReloadOutline /></n-icon>
          </template>
          补装内建模板
        </n-button>
        <n-button quaternary @click="showCommunity = true" title="浏览内置的社区预设包">
          <template #icon>
            <n-icon><GlobeOutline /></n-icon>
          </template>
          社区预设
        </n-button>
        <n-button quaternary @click="handleImport" title="从 JSON 模板包导入">
          <template #icon>
            <n-icon><CloudUploadOutline /></n-icon>
          </template>
          导入
        </n-button>
        <n-button
          quaternary
          @click="handleExportAll"
          :disabled="templates.length === 0"
          title="导出全部模板为 JSON 模板包"
        >
          <template #icon>
            <n-icon><DownloadOutline /></n-icon>
          </template>
          导出全部
        </n-button>
        <n-button type="primary" @click="openCreate">
          <template #icon>
            <n-icon><AddOutline /></n-icon>
          </template>
          新建模板
        </n-button>
      </div>
    </div>

    <n-data-table
      :columns="columns"
      :data="templates"
      :loading="loading"
      :bordered="false"
      :pagination="false"
    />

    <n-empty
      v-if="!loading && templates.length === 0"
      description="当前账号库未配置任何模板，可点击「补装内建模板」一键填充"
      style="padding: 60px 0"
    />

    <!-- 编辑器 -->
    <n-modal
      v-model:show="showEditor"
      preset="card"
      :title="editorMode === 'create' ? '新建模板' : `编辑模板：${form.id}`"
      style="width: 720px"
    >
      <n-form label-placement="left" label-width="80">
        <n-form-item label="id" required>
          <n-input
            v-model:value="form.id"
            placeholder="如：bank_card / wechat / steam（保存后不可改）"
            :disabled="editorMode === 'edit'"
          />
        </n-form-item>
        <n-form-item label="名称" required>
          <n-input v-model:value="form.name" placeholder="例如：银行卡" />
        </n-form-item>
        <n-form-item label="图标">
          <n-input
            v-model:value="form.icon"
            placeholder="可选，单字符 emoji，例如 💳"
            style="width: 220px"
          />
        </n-form-item>
        <n-divider style="margin: 8px 0">业务字段</n-divider>

        <div class="fields-editor">
          <div v-if="form.fields.length === 0" class="fields-empty">
            尚未定义字段，点击下方「新增字段」开始
          </div>
          <div
            v-for="(f, i) in form.fields"
            :key="i"
            class="field-row"
          >
            <div class="field-ord">{{ i + 1 }}</div>
            <n-input
              v-model:value="f.key"
              placeholder="key"
              style="width: 130px"
              size="small"
            />
            <n-input
              v-model:value="f.label"
              placeholder="显示名"
              style="width: 130px"
              size="small"
            />
            <n-select
              v-model:value="f.field_type"
              :options="fieldTypeOptions"
              size="small"
              style="width: 150px"
            />
            <n-input
              v-model:value="f.placeholder"
              placeholder="占位提示（可选）"
              size="small"
              style="flex: 1; min-width: 100px"
            />
            <div class="field-actions">
              <n-button
                quaternary
                size="tiny"
                @click="moveField(i, -1)"
                :disabled="i === 0"
                title="上移"
              >
                ↑
              </n-button>
              <n-button
                quaternary
                size="tiny"
                @click="moveField(i, 1)"
                :disabled="i === form.fields.length - 1"
                title="下移"
              >
                ↓
              </n-button>
              <n-button
                quaternary
                size="tiny"
                type="error"
                @click="removeField(i)"
                title="删除"
              >
                <template #icon>
                  <n-icon><TrashOutline /></n-icon>
                </template>
              </n-button>
            </div>
          </div>
          <n-button
            block
            dashed
            @click="addField"
            style="margin-top: 8px"
          >
            <template #icon>
              <n-icon><AddOutline /></n-icon>
            </template>
            新增字段
          </n-button>
        </div>

        <n-divider style="margin: 16px 0 8px">自动推荐规则</n-divider>
        <div style="font-size:12px;color:var(--n-text-color-3);margin:-4px 0 8px 0;line-height:1.6">
          编辑账号时如命中以下任一规则，会提示「应用此模板」。
          主要适合以 url 为主要识别点的账号（如「看到 icbc.com.cn 即推荐银行卡」）。
        </div>
        <div class="fields-editor">
          <div v-if="form.match_rules.length === 0" class="fields-empty">
            未配置推荐规则，点击下方「新增规则」添加
          </div>
          <div
            v-for="(r, i) in form.match_rules"
            :key="i"
            class="field-row"
          >
            <div class="field-ord">{{ i + 1 }}</div>
            <n-select
              v-model:value="r.kind"
              :options="matchRuleKindOptions"
              size="small"
              style="width: 130px"
            />
            <n-input
              v-model:value="r.value"
              :placeholder="
                r.kind === 'url_contains'
                  ? '如：icbc.com.cn'
                  : r.kind === 'keyword'
                  ? '如：银行、身份证'
                  : '如：工作'
              "
              size="small"
              style="flex: 1; min-width: 100px"
            />
            <div class="field-actions">
              <n-button
                quaternary
                size="tiny"
                type="error"
                @click="removeMatchRule(i)"
                title="删除"
              >
                <template #icon>
                  <n-icon><TrashOutline /></n-icon>
                </template>
              </n-button>
            </div>
          </div>
          <n-button
            block
            dashed
            @click="addMatchRule"
            style="margin-top: 8px"
          >
            <template #icon>
              <n-icon><AddOutline /></n-icon>
            </template>
            新增规则
          </n-button>
        </div>
      </n-form>

      <template #footer>
        <n-space justify="end">
          <n-button @click="showEditor = false">取消</n-button>
          <n-button type="primary" :loading="saving" @click="handleSave">
            保存
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- 社区预设包浏览器 -->
    <n-modal
      v-model:show="showCommunity"
      preset="card"
      title="社区预设模板包"
      style="width: 720px"
    >
      <div style="font-size:12px;color:var(--n-text-color-3);margin-bottom:12px;line-height:1.6">
        下面是内置的一组社区预设模板包，点击「装入」会将该包内的模板一次性加入到当前账号库。
        同 id 模板将被覆盖。
      </div>
      <div class="community-list">
        <div
          v-for="pack in communityPacks"
          :key="pack.id"
          class="community-pack"
        >
          <div class="pack-head">
            <div class="pack-title">
              <span class="pack-icon">{{ pack.icon }}</span>
              <span class="pack-name">{{ pack.name }}</span>
              <span class="pack-count">{{ pack.templates.length }} 个模板</span>
            </div>
            <n-button
              size="small"
              :type="installedPackIds.has(pack.id) ? 'success' : 'primary'"
              :loading="installingPackId === pack.id"
              :disabled="installingPackId !== null && installingPackId !== pack.id"
              @click="installCommunityPack(pack)"
            >
              {{ installedPackIds.has(pack.id) ? '✓ 已装入' : '装入' }}
            </n-button>
          </div>
          <div class="pack-desc">{{ pack.description }}</div>
          <div class="pack-templates">
            <span
              v-for="t in pack.templates"
              :key="t.id"
              class="pack-tpl-chip"
              :title="`${t.id} · ${t.fields.length} 字段 · ${(t.match_rules || []).length} 规则`"
            >{{ t.icon ? t.icon + ' ' : '' }}{{ t.name }}</span>
          </div>
        </div>
      </div>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showCommunity = false">关闭</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.tpl-mgmt {
  height: 100%;
  overflow-y: auto;
  padding: 0 4px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
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

.toolbar-left {
  display: flex;
  align-items: baseline;
  gap: 12px;
  min-width: 0;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}

.hint {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  opacity: 0.85;
}

.fields-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.fields-empty {
  padding: 18px;
  text-align: center;
  color: var(--n-text-color-3, #999);
  font-size: 12px;
  border: 1px dashed var(--app-border-color);
  border-radius: 8px;
}

.field-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.field-ord {
  width: 22px;
  text-align: center;
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  font-family: ui-monospace, monospace;
}

.field-actions {
  display: flex;
  gap: 2px;
}

:deep(.tpl-act) {
  border: 1px solid var(--app-border-color);
  background: transparent;
  color: var(--n-text-color);
  padding: 2px 10px;
  font-size: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
}
:deep(.tpl-act:hover) {
  background: var(--n-action-color, rgba(0, 0, 0, 0.05));
}
:deep(.tpl-act-edit) {
  color: var(--n-color-primary, #2080f0);
  border-color: var(--n-color-primary, #2080f0);
}
:deep(.tpl-act-del) {
  color: var(--n-color-error, #d03050);
  border-color: var(--n-color-error, #d03050);
}

.community-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: 60vh;
  overflow-y: auto;
}
.community-pack {
  border: 1px solid var(--app-border-color);
  border-radius: 10px;
  padding: 12px 14px;
  background: var(--app-card-bg, transparent);
}
.pack-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 6px;
}
.pack-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.pack-icon {
  font-size: 18px;
}
.pack-name {
  font-weight: 600;
  font-size: 14px;
}
.pack-count {
  font-size: 12px;
  color: var(--n-text-color-3);
}
.pack-desc {
  font-size: 12px;
  color: var(--n-text-color-3);
  margin-bottom: 8px;
  line-height: 1.6;
}
.pack-templates {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.pack-tpl-chip {
  padding: 2px 10px;
  border-radius: 999px;
  background: var(--app-border-color, rgba(0, 0, 0, 0.06));
  font-size: 12px;
}
</style>
