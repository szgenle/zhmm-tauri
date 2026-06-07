<script setup lang="ts">
import { ref } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import BackupListDialog from "../components/BackupListDialog.vue";

const message = useMessage();
const dialog = useDialog();

const busy = ref(false);

const showBackupDialog = ref(false);
const backupMode = ref<"backup" | "restore">("backup");
const showBackupListDialog = ref(false);
const backupPath = ref("");
const backupPassword = ref("");
// 备份时是否复用当前主密码（默认开，一键备份）
const useMasterForBackup = ref(true);

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
