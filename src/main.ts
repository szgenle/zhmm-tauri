import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { migrateZhmmToAjot } from "./utils/storageMigration";
import { themePresets, getVisualStyle, applyCssVars } from "./themes";
import { THEME_MODE_CACHE_KEY } from "./settings";
import "./styles/global.css";

// v2.0 品牌切换：把 zhmm_* / zhmm:* 前缀的本地存储一次性迁移到 ajot 前缀
migrateZhmmToAjot();

/**
 * 在 Vue 挂载前同步预设主题 CSS 变量与 data-theme 属性，
 * 避免初次进入主页时因后端 settings 异步加载导致的「白底黑字闪烁」。
 *
 * - 主题模式：从 localStorage 读上次保存的 theme（auto/light/dark），
 *   auto 时读系统媒体查询。
 * - 视觉风格：从 localStorage 读上次保存的 visualStyle。
 */
(function preApplyTheme() {
  let mode: "auto" | "light" | "dark" = "auto";
  try {
    const v = localStorage.getItem(THEME_MODE_CACHE_KEY);
    if (v === "light" || v === "dark" || v === "auto") mode = v;
  } catch {}
  const isDark =
    mode === "dark" ||
    (mode === "auto" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  const preset = themePresets[getVisualStyle()];
  applyCssVars(isDark ? preset.cssDarkVars : preset.cssVars);
  document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
})();

createApp(App).use(router).mount("#app");
