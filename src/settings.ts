/**
 * 全局共享的应用设置（reactive，模块单例）
 */
import { reactive } from "vue";
import { api, type AppSettings } from "./api";

export const settings = reactive<AppSettings>({
  theme: "auto",
  auto_lock_minutes: 5,
  clipboard_clear_seconds: 30,
});

let loaded = false;

export async function loadSettings(): Promise<void> {
  try {
    const s = await api.getSettings();
    settings.theme = s.theme;
    settings.auto_lock_minutes = s.auto_lock_minutes;
    settings.clipboard_clear_seconds = s.clipboard_clear_seconds;
    loaded = true;
  } catch {
    // 后端未就绪时静默 fallback 到默认
  }
}

export function isLoaded(): boolean {
  return loaded;
}

export async function saveSettings(patch: Partial<AppSettings>): Promise<void> {
  Object.assign(settings, patch);
  await api.updateSettings({ ...settings });
}

/**
 * 复制后定时清空剪贴板。
 *
 * 由于 Tauri webview 默认禁止 navigator.clipboard.readText()，无法通过比对当前
 * 剪贴板内容判断是否被我们写入；改为模块级 token 模式：每次写入生成新 token，
 * 定时器触发时只有 token 仍是当前的（中间没有再次走过本函数）才执行清空。
 * 这样能避免连续复制相互打架，但若用户在外部复制了别的内容，仍可能被一并清空，
 * 这是为了在 Tauri 环境下保证“到期一定清空”的安全语义所做的折衷。
 */
let __clipboardToken = 0;

export function copyAndScheduleClear(text: string): Promise<void> {
  const token = ++__clipboardToken;
  return navigator.clipboard.writeText(text).then(() => {
    const sec = settings.clipboard_clear_seconds;
    if (sec <= 0) return;
    window.setTimeout(async () => {
      if (token !== __clipboardToken) return; // 期间又有新的复制，交给后来的定时器处理
      try {
        await navigator.clipboard.writeText("");
      } catch {
        // 写入失败忽略
      }
    }, sec * 1000);
  });
}
