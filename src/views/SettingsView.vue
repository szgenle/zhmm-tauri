<script setup lang="ts">
import { ref, watch } from "vue";
import { settings, saveSettings } from "../settings";
import RekeyDialog from "../components/RekeyDialog.vue";

const message = useMessage();

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

async function onAntiScreenshotChange(value: boolean) {
  try {
    await saveSettings({ anti_screenshot: value });
  } catch (e: any) {
    message.error(`防截屏设置保存失败: ${e}`);
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

const showRekey = ref(false);
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
        <n-form-item label="防截屏保护">
          <n-switch :value="settings.anti_screenshot" @update:value="onAntiScreenshotChange" />
          <n-text depth="3" style="margin-left: 12px; font-size: 12px">
            开启后屏幕录制 / 截图将看不到本窗口内容（macOS / Windows）
          </n-text>
        </n-form-item>
      </n-form>
      <n-button type="primary" size="small" @click="applySettings">保存设置</n-button>
      <n-divider />
      <n-space>
        <n-button type="warning" :disabled="busy" @click="showRekey = true">更换主密码</n-button>
      </n-space>
      <n-text depth="3" style="font-size: 12px">
        更换前会自动在 <n-text code>.backups/</n-text> 生成「<n-text code>rekey_</n-text>」前缀的保险备份。
      </n-text>
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
    <n-card title="关于">
      <p>zhmm · Tauri 2.0 + Vue 3</p>
    </n-card>
    <RekeyDialog v-model:show="showRekey" />
  </div>
</template>
