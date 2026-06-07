/**
 * 全局共享的应用设置（reactive，模块单例）
 */
import { reactive } from "vue";
import { writeText, clear } from "@tauri-apps/plugin-clipboard-manager";
import { api, type AppSettings } from "./api";

/**
 * 主题模式的 localStorage 缓存键。
 * 用途：在 Tauri 后端 settings 异步加载完成前，前端可以同步读取到上次的偏好，
 *      避免亮/暗主题切换闪烁（首帧白底黑字）。
 */
export const THEME_MODE_CACHE_KEY = "ajot_theme_mode";

function readCachedThemeMode(): "auto" | "light" | "dark" {
  try {
    const v = localStorage.getItem(THEME_MODE_CACHE_KEY);
    if (v === "light" || v === "dark" || v === "auto") return v;
  } catch {}
  return "auto";
}

function writeCachedThemeMode(mode: "auto" | "light" | "dark"): void {
  try {
    localStorage.setItem(THEME_MODE_CACHE_KEY, mode);
  } catch {}
}

export const settings = reactive<AppSettings>({
  theme: readCachedThemeMode(),
  auto_lock_minutes: 5,
  clipboard_clear_seconds: 30,
  anti_screenshot: true,
});

let loaded = false;

export async function loadSettings(): Promise<void> {
  try {
    const s = await api.getSettings();
    settings.theme = s.theme;
    settings.auto_lock_minutes = s.auto_lock_minutes;
    settings.clipboard_clear_seconds = s.clipboard_clear_seconds;
    settings.anti_screenshot = s.anti_screenshot ?? true;
    writeCachedThemeMode(s.theme as "auto" | "light" | "dark");
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
  if (patch.theme) writeCachedThemeMode(patch.theme as "auto" | "light" | "dark");
  await api.updateSettings({ ...settings });
}

/**
 * 复制后定时清空剪贴板。
 *
 * 使用 @tauri-apps/plugin-clipboard-manager 直接操作系统剪贴板，
 * 避免 navigator.clipboard 在 Tauri webview 中的权限问题。
 * 使用 token 模式防止连续复制时早期的定时器误清后续复制的内容。
 */
let __clipboardToken = 0;

export async function copyAndScheduleClear(text: string): Promise<void> {
  const token = ++__clipboardToken;
  await writeText(text);
  const sec = settings.clipboard_clear_seconds;
  if (sec <= 0) return;
  window.setTimeout(async () => {
    if (token !== __clipboardToken) return; // 期间又有新的复制，交给后来的定时器处理
    try {
      await clear();
    } catch {
      // 清空失败忽略
    }
  }, sec * 1000);
}
