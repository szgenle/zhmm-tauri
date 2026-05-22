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

async function handleExportXlsx() {
  const path = await saveDialog({
    title: "导出为 xlsx",
    defaultPath: "zhmm-export.xlsx",
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  busy.value = true;
  try {
    await api.exportXlsx(path as string);
    message.success(`已导出到 ${path}`);
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
    defaultPath: `zhmm-backup-${new Date().toISOString().slice(0, 10)}.zmb`,
    filters: [{ name: "zmb 加密备份", extensions: ["zmb"] }],
  });
  if (!path) return;
  backupPath.value = path as string;
  backupMode.value = "backup";
  backupPassword.value = "";
  showBackupDialog.value = true;
}

async function startRestore() {
  const path = await openDialog({
    title: "选择加密备份文件",
    multiple: false,
    filters: [{ name: "zmb 加密备份", extensions: ["zmb"] }],
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
  if (!backupPassword.value) {
    message.error("请输入备份密码");
    return;
  }
  busy.value = true;
  try {
    if (backupMode.value === "backup") {
      await api.backupToFile(backupPath.value, backupPassword.value);
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
          xlsx 用于跨工具迁移；故意不导出 TOTP 密钥与密码历史，避免敏感信息扩散。
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
          加密备份保留所有信息（含 TOTP 密钥与历史），可使用与主密码不同的备份密码。
        </n-text>
      </n-space>
    </n-card>

    <n-card title="标签与词典">
      <n-space>
        <n-button @click="showTagManagement = true">标签管理</n-button>
        <n-button @click="showSiteCatalog = true">网站词典</n-button>
      </n-space>
      <n-text depth="3" style="font-size: 12px; margin-top: 8px; display: block">
        标签管理支持重命名和删除；网站词典可查看内置的离线站点匹配规则。
      </n-text>
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
        <n-form-item label="密码">
          <n-input
            v-model:value="backupPassword"
            type="password"
            show-password-on="click"
            placeholder="备份用密码"
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
    <BackupListDialog v-model:show="showBackupListDialog" />
    <TagManagementDialog v-model:show="showTagManagement" @changed="() => {}" />
    <SiteCatalogDialog v-model:show="showSiteCatalog" />
  </div>
</template>

<style scoped>
.data-mgmt {
  max-width: 700px;
}
</style>
