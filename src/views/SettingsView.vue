<script setup lang="ts">
import { ref, watch } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { settings, saveSettings } from "../settings";
import BackupListDialog from "../components/BackupListDialog.vue";

const message = useMessage();
const dialog = useDialog();

const busy = ref(false);

// 数字类设置使用本地表单暂存，避免每次输入都触发后端写入
const form = ref({
  auto_lock_minutes: settings.auto_lock_minutes,
  clipboard_clear_seconds: settings.clipboard_clear_seconds,
});

// 外部设置变化时同步表单（例如重新加载）
watch(
  () => [settings.auto_lock_minutes, settings.clipboard_clear_seconds],
  ([a, c]) => {
    form.value.auto_lock_minutes = a as number;
    form.value.clipboard_clear_seconds = c as number;
  }
);

// 主题切换实时生效并自动持久化
async function onThemeChange(value: string) {
  try {
    await saveSettings({ theme: value as any });
  } catch (e: any) {
    message.error(`主题保存失败: ${e}`);
  }
}

async function applySettings() {
  try {
    await saveSettings({
      auto_lock_minutes: form.value.auto_lock_minutes,
      clipboard_clear_seconds: form.value.clipboard_clear_seconds,
    });
    message.success("设置已保存");
  } catch (e: any) {
    message.error(`保存失败: ${e}`);
  }
}

// 加密备份/恢复对话框
const showBackupDialog = ref(false);
const backupMode = ref<"backup" | "restore">("backup");
const showBackupListDialog = ref(false);
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

async function startBackup() {
  const path = await saveDialog({
    title: "保存加密备份",
    defaultPath: `zhmm-backup-${new Date().toISOString().slice(0, 10)}.zhmm`,
    filters: [{ name: "zhmm 加密备份", extensions: ["zhmm"] }],
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
    filters: [{ name: "zhmm 加密备份", extensions: ["zhmm"] }],
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
  <div>
    <n-card title="安全" style="margin-bottom: 16px">
      <n-form label-placement="left" label-width="160">
        <n-form-item label="空闲自动锁定 (分钟)">
          <n-input-number
            v-model:value="form.auto_lock_minutes"
            :min="0"
            :max="1440"
            style="width: 160px"
          />
          <n-text depth="3" style="margin-left: 12px; font-size: 12px">0 = 不自动锁定</n-text>
        </n-form-item>
        <n-form-item label="剪贴板自动清空 (秒)">
          <n-input-number
            v-model:value="form.clipboard_clear_seconds"
            :min="0"
            :max="600"
            style="width: 160px"
          />
          <n-text depth="3" style="margin-left: 12px; font-size: 12px">0 = 不清空</n-text>
        </n-form-item>
      </n-form>
      <n-button type="primary" size="small" @click="applySettings">保存设置</n-button>
    </n-card>
    <n-card title="外观" style="margin-bottom: 16px">
      <n-form label-placement="left" label-width="160">
        <n-form-item label="主题">
          <n-radio-group :value="settings.theme" @update:value="onThemeChange">
            <n-radio-button value="auto">跟随系统</n-radio-button>
            <n-radio-button value="light">亮色</n-radio-button>
            <n-radio-button value="dark">暗色</n-radio-button>
          </n-radio-group>
        </n-form-item>
      </n-form>
      <n-text depth="3" style="font-size: 12px">主题切换会立即生效并自动保存。</n-text>
    </n-card>
    <n-card title="数据管理" style="margin-bottom: 16px">
      <n-space vertical size="medium">
        <n-space>
          <n-button :disabled="busy" @click="handleExportXlsx">导出 xlsx</n-button>
          <n-button :disabled="busy" @click="handleImportXlsx">导入 xlsx</n-button>
        </n-space>
        <n-text depth="3" style="font-size: 12px">
          xlsx 用于跨工具迁移；故意不导出 TOTP 密钥与密码历史，避免敏感信息扩散。
        </n-text>
        <n-divider />
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
    <n-card title="关于">
      <p>智慧密码 · Tauri 2.0 + Vue 3</p>
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
  </div>
</template>
