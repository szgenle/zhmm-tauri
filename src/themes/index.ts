import type { GlobalThemeOverrides } from "naive-ui";
import { glassPreset } from "./presets/glass";
import { minimalPreset } from "./presets/minimal";
import { cyberpunkPreset } from "./presets/cyberpunk";
import { warmPreset } from "./presets/warm";

export type VisualStyle = "glass" | "minimal" | "cyberpunk" | "warm";

export interface ThemePreset {
  key: VisualStyle;
  label: string;
  description: string;
  lightOverrides: GlobalThemeOverrides;
  darkOverrides: GlobalThemeOverrides;
  cssVars: Record<string, string>;
  cssDarkVars: Record<string, string>;
}

export const themePresets: Record<VisualStyle, ThemePreset> = {
  glass: glassPreset,
  minimal: minimalPreset,
  cyberpunk: cyberpunkPreset,
  warm: warmPreset,
};

export const visualStyleOptions: { key: VisualStyle; label: string; description: string; color: string }[] = [
  { key: "glass", label: "清透玻璃", description: "半透明卡片、柔和阴影", color: "#4098fc" },
  { key: "minimal", label: "极简纯净", description: "大留白、细线条", color: "#111111" },
  { key: "cyberpunk", label: "深邃科技", description: "荧光强调、微光边框", color: "#7c3aed" },
  { key: "warm", label: "温暖圆润", description: "大圆角、柔和渐变", color: "#f97316" },
];

const STORAGE_KEY = "zhmm_visual_style";

export function getVisualStyle(): VisualStyle {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && stored in themePresets) return stored as VisualStyle;
  } catch {}
  return "glass";
}

export function setVisualStyle(style: VisualStyle): void {
  localStorage.setItem(STORAGE_KEY, style);
}

/**
 * Apply CSS variables to the document root element
 */
export function applyCssVars(vars: Record<string, string>): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(vars)) {
    root.style.setProperty(key, value);
  }
}
