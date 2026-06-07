<script setup lang="ts">
import { ref } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import BackupListDialog from "../components/BackupListDialog.vue";
import TagManagementDialog from "../components/TagManagementDialog.vue";
import SiteCatalogDialog from "../components/SiteCatalogDialog.vue";

const message = useMessage();
const dialog = useDialog();

const busy = ref(false);

const showBackupDialog = ref(false);
const backupMode = ref<"backup" | "restore">("backup");
const showBackupListDialog = ref(false);
const showTagManagement = ref(false);
const showSiteCatalog = ref(false);
const backupPath = ref("");
const backupPassword = ref("");
// 备份时是否复用当前主密码（默认开，一键备份）
const useMasterForBackup = ref(true);

// ========== 网站词典导入/导出 ==========
const catalogBusy = ref(false);
const hasUserCatalog = ref(false);
const showPrompt = ref(false);

const LLM_PROMPT_TEMPLATE = `你是一位分类整理专家。我会给你一份 JSON 格式的"网站词典"，其中每个条目包含域名（host）、名称（name）和标签（tags）。

我的行业/职业：【在这里填写你的行业和职业，例如：互联网行业 / 前端开发工程师】

请你帮我按以下原则重新整理所有条目的标签（tags）：

1. **我所在行业相关的网站**：标签应更细致、更具体，体现子领域差异。
   - 例如程序员可细分为：前端开发、后端开发、DevOps、数据库、云服务、AI/ML、开源社区、技术博客、包管理、代码托管、API工具 等
2. **非本行业的网站**：标签可以粗粒度归类，避免分得太细。
   - 例如：购物、视频、社交、新闻、金融、政务、邮箱、搜索 等大类即可
3. **每个站点的标签数量**：1~3 个为宜，不超过 4 个
4. **标签命名风格**：简短中文词组（2~4字），同类站点保持一致
5. **name 字段**：如果为空，请根据域名推测填写一个简洁的中文名称；已有的名称保持不变
6. **不要删除任何条目**，只修改 tags 和补充空的 name

请直接输出整理后的完整 JSON（保持原格式，含 _meta 和 sites），不需要解释。`;

function copyPrompt() {
  navigator.clipboard.writeText(LLM_PROMPT_TEMPLATE);
  message.success("提示词已复制到剪贴板");
}

async function checkUserCatalog() {
  try {
    hasUserCatalog.value = await api.hasUserCatalog();
  } catch {
    hasUserCatalog.value = false;
  }
}
// 页面初始化时检查
checkUserCatalog();

async function handleExportCatalog() {
  const path = await saveDialog({
    title: "导出网站词典",
    defaultPath: `site_catalog_${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  catalogBusy.value = true;
  try {
    const count = await api.exportSiteCatalog(path as string);
    message.success(`已导出 ${count} 个站点（含密码库中的网站）`);
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

async function handleImportCatalog() {
  const path = await openDialog({
    title: "选择词典 JSON 文件",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  catalogBusy.value = true;
  try {
    const count = await api.importSiteCatalog(path as string);
    message.success(`已导入 ${count} 个站点`);
    hasUserCatalog.value = true;
  } catch (e: any) {
    message.error(`导入失败: ${e}`);
  } finally {
    catalogBusy.value = false;
  }
}

function handleResetCatalog() {
  dialog.warning({
    title: "确认恢复默认",
    content: "将清除所有自定义词典数据，恢复为内置词典。此操作不可撤销。",
    positiveText: "确认恢复",
    negativeText: "取消",
    onPositiveClick: async () => {
      catalogBusy.value = true;
      try {
        await api.resetSiteCatalog();
        message.success("已恢复为默认词典");
        hasUserCatalog.value = false;
      } catch (e: any) {
        message.error(`重置失败: ${e}`);
      } finally {
        catalogBusy.value = false;
      }
    },
  });
}

// 导出 xlsx 二次身份确认
const showExportXlsxDialog = ref(false);
const exportXlsxPath = ref("");
const exportXlsxPassword = ref("");

async function handleExportXlsx() {
  const path = await saveDialog({
    title: "导出为 xlsx",
    defaultPath: "zhmm-export.xlsx",
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  // xlsx 明文落盘不可逆，先让用户重验主密码
  exportXlsxPath.value = path as string;
  exportXlsxPassword.value = "";
  showExportXlsxDialog.value = true;
}

async function confirmExportXlsx() {
  if (!exportXlsxPassword.value) {
    message.error("请输入主密码");
    return;
  }
  busy.value = true;
  try {
    await api.exportXlsx(exportXlsxPath.value, exportXlsxPassword.value);
    message.success(`已导出到 ${exportXlsxPath.value}`);
    showExportXlsxDialog.value = false;
    exportXlsxPassword.value = "";
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  } finally {
    busy.value = false;
  }
}

async function handleImportXlsx() {
  const path = await openDialog({
    title: "选择 xlsx 文件",
    multiple: false,
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  busy.value = true;
  try {
    const count = await api.importXlsx(path as string);
    message.success(`已导入 ${count} 条`);
  } catch (e: any) {
    message.error(`导入失败: ${e}`);
  } finally {
    busy.value = false;
  }
}

async function handleDownloadTemplate() {
  const path = await saveDialog({
    title: "保存 xlsx 模板",
    defaultPath: "zhmm-template.xlsx",
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  busy.value = true;
  try {
    await api.exportXlsxTemplate(path as string);
    message.success(`模板已保存到 ${path}`);
  } catch (e: any) {
    message.error(`模板生成失败: ${e}`);
  } finally {
    busy.value = false;
  }
}

async function startBackup() {
  const path = await saveDialog({
    title: "保存加密备份",
    defaultPath: `account-jotter-backup-${new Date().toISOString().slice(0, 10)}.ajot`,
    filters: [
      { name: "加密备份（.ajot）", extensions: ["ajot"] },
      { name: "加密备份（.zmb 旧版）", extensions: ["zmb"] },
    ],
  });
  if (!path) return;
  backupPath.value = path as string;
  backupMode.value = "backup";
  backupPassword.value = "";
  // 备份默认复用主密码，用户需要独立密码时可取消勾选
  useMasterForBackup.value = true;
  showBackupDialog.value = true;
}

async function startRestore() {
  const path = await openDialog({
    title: "选择加密备份文件",
    multiple: false,
    filters: [
      { name: "加密备份", extensions: ["ajot", "zmb"] },
      { name: "账号小本本（.ajot）", extensions: ["ajot"] },
      { name: "ZMB 备份（旧版）", extensions: ["zmb"] },
    ],
  });
  if (!path) return;
  dialog.warning({
    title: "确认恢复",
    content: "恢复会完全覆盖当前密码库的所有数据，且不可撤销，是否继续？",
    positiveText: "继续",
    negativeText: "取消",
    onPositiveClick: () => {
      backupPath.value = path as string;
      backupMode.value = "restore";
      backupPassword.value = "";
      showBackupDialog.value = true;
    },
  });
}

async function confirmBackup() {
  // 备份场景下，如果勾选"使用主密码"则不需输入密码
  const isMasterBackup =
    backupMode.value === "backup" && useMasterForBackup.value;
  if (!isMasterBackup && !backupPassword.value) {
    message.error(backupMode.value === "backup" ? "请输入备份密码" : "请输入备份密码");
    return;
  }
  busy.value = true;
  try {
    if (backupMode.value === "backup") {
      // 勾选主密码时传 null，后端会使用会话主密码
      await api.backupToFile(
        backupPath.value,
        isMasterBackup ? null : backupPassword.value,
      );
      message.success(`已加密备份到 ${backupPath.value}`);
    } else {
      await api.restoreFromFile(backupPath.value, backupPassword.value);
      message.success("已从备份恢复");
    }
    showBackupDialog.value = false;
    backupPassword.value = "";
  } catch (e: any) {
    message.error(`操作失败: ${e}`);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="data-mgmt">
    <n-card title="Excel 导入导出" style="margin-bottom: 16px">
      <n-space vertical size="medium">
        <n-space>
          <n-button :disabled="busy" @click="handleExportXlsx">导出 xlsx</n-button>
          <n-button :disabled="busy" @click="handleImportXlsx">导入 xlsx</n-button>
          <n-button :disabled="busy" @click="handleDownloadTemplate">下载模板</n-button>
        </n-space>
        <n-text depth="3" style="font-size: 12px">
          xlsx 用于跨工具迁移；导出为明文，需重验主密码。故意不导出 TOTP 密钥与密码历史，避免敏感信息扩散。
        </n-text>
      </n-space>
    </n-card>

    <n-card title="加密备份" style="margin-bottom: 16px">
      <n-space vertical size="medium">
        <n-space>
          <n-button type="primary" :disabled="busy" @click="startBackup">加密备份</n-button>
          <n-button type="warning" :disabled="busy" @click="startRestore">恢复备份</n-button>
          <n-button :disabled="busy" @click="showBackupListDialog = true">备份管理</n-button>
        </n-space>
        <n-text depth="3" style="font-size: 12px">
          加密备份保留所有信息（含 TOTP 密钥与历史）。默认复用主密码一键备份，也可设置独立备份密码。
        </n-text>
      </n-space>
    </n-card>

    <n-card title="标签与词典" style="margin-bottom: 16px">
      <n-space vertical size="medium">
        <n-space>
          <n-button @click="showTagManagement = true">标签管理</n-button>
          <n-button @click="showSiteCatalog = true">查看词典</n-button>
        </n-space>
        <n-text depth="3" style="font-size: 12px; display: block">
          标签管理支持重命名和删除；网站词典可查看内置的离线站点匹配规则。
        </n-text>
      </n-space>
    </n-card>

    <n-card title="网站词典导入导出">
      <n-space vertical size="medium">
        <n-space align="center">
          <n-button type="primary" :loading="catalogBusy" :disabled="busy" @click="handleExportCatalog">
            导出词典
          </n-button>
          <n-button :loading="catalogBusy" :disabled="busy" @click="handleImportCatalog">
            导入词典
          </n-button>
          <n-button
            v-if="hasUserCatalog"
            quaternary
            type="warning"
            :loading="catalogBusy"
            :disabled="busy"
            @click="handleResetCatalog"
          >
            恢复默认
          </n-button>
          <n-tag v-if="hasUserCatalog" type="info" size="small">已自定义</n-tag>
        </n-space>
        <n-text depth="3" style="font-size: 12px; display: block">
          导出时会自动包含密码库中已录入但词典尚未收录的网站（名称/标签留空）。你可以将导出的 JSON
          交给 AI 按行业整理分类标签，再导入即可生效。
        </n-text>
        <n-collapse-transition :show="showPrompt">
          <n-card
            size="small"
            style="margin-top: 8px; background: var(--n-color-embedded)"
            :bordered="false"
          >
            <template #header>
              <n-text style="font-size: 13px; font-weight: 500">AI 整理提示词模板</n-text>
            </template>
            <template #header-extra>
              <n-button size="tiny" quaternary @click="copyPrompt">复制</n-button>
            </template>
            <n-text
              tag="pre"
              style="font-size: 12px; white-space: pre-wrap; word-break: break-word; margin: 0; line-height: 1.6"
            >{{ LLM_PROMPT_TEMPLATE }}</n-text>
            <n-text depth="3" tag="p" style="font-size: 11px; margin-top: 8px; margin-bottom: 0">
              使用方法：复制上方提示词 → 将"【在这里填写…】"替换为你的行业/职业 → 连同导出的 JSON 一起发送给任意 AI 对话 → 将 AI 返回的 JSON 保存为文件后导入。
            </n-text>
          </n-card>
        </n-collapse-transition>
        <n-button
          text
          type="primary"
          size="small"
          style="margin-top: 4px"
          @click="showPrompt = !showPrompt"
        >
          {{ showPrompt ? '收起提示词' : '查看 AI 整理提示词' }}
        </n-button>
      </n-space>
    </n-card>

    <n-modal
      v-model:show="showBackupDialog"
      preset="card"
      :title="backupMode === 'backup' ? '设置备份密码' : '输入备份密码'"
      style="width: 420px"
    >
      <n-form>
        <n-form-item label="文件">
          <n-text style="word-break: break-all">{{ backupPath }}</n-text>
        </n-form-item>
        <n-form-item v-if="backupMode === 'backup'" label="密码来源">
          <n-checkbox v-model:checked="useMasterForBackup">
            使用当前主密码（一键备份）
          </n-checkbox>
        </n-form-item>
        <n-form-item
          v-if="!(backupMode === 'backup' && useMasterForBackup)"
          label="密码"
        >
          <n-input
            v-model:value="backupPassword"
            type="password"
            show-password-on="click"
            :placeholder="backupMode === 'backup' ? '独立的备份密码' : '备份文件的密码'"
            @keyup.enter="confirmBackup"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showBackupDialog = false">取消</n-button>
          <n-button type="primary" :loading="busy" @click="confirmBackup">
            {{ backupMode === "backup" ? "备份" : "恢复" }}
          </n-button>
        </n-space>
      </template>
    </n-modal>
    <n-modal
      v-model:show="showExportXlsxDialog"
      preset="card"
      title="确认导出 xlsx（明文）"
      style="width: 460px"
    >
      <n-alert type="warning" style="margin-bottom: 12px">
        xlsx 将以明文落盘，不可逆。请重新输入主密码以确认本人操作。
      </n-alert>
      <n-form>
        <n-form-item label="文件">
          <n-text style="word-break: break-all">{{ exportXlsxPath }}</n-text>
        </n-form-item>
        <n-form-item label="主密码">
          <n-input
            v-model:value="exportXlsxPassword"
            type="password"
            show-password-on="click"
            placeholder="请输入当前主密码"
            @keyup.enter="confirmExportXlsx"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showExportXlsxDialog = false">取消</n-button>
          <n-button type="primary" :loading="busy" @click="confirmExportXlsx">导出</n-button>
        </n-space>
      </template>
    </n-modal>
    <BackupListDialog v-model:show="showBackupListDialog" />
    <TagManagementDialog v-model:show="showTagManagement" @changed="() => {}" />
    <SiteCatalogDialog v-model:show="showSiteCatalog" />
  </div>
</template>

<style scoped>
.data-mgmt {
  max-width: 700px;
  height: 100%;
  overflow-y: auto;
  overscroll-behavior: none;
}
</style>
