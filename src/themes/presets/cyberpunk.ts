import type { ThemePreset } from "..";

export const cyberpunkPreset: ThemePreset = {
  key: "cyberpunk",
  label: "深邃科技",
  description: "暗色为主、荧光强调、卡片微光边框",
  lightOverrides: {
    common: {
      primaryColor: "#7c3aed",
      primaryColorHover: "#8b5cf6",
      primaryColorPressed: "#6d28d9",
      primaryColorSuppl: "#7c3aed",
      borderRadius: "8px",
      borderRadiusSmall: "4px",
      fontFamily:
        '"JetBrains Mono", "SF Mono", Menlo, "PingFang SC", "Microsoft YaHei", monospace',
      fontSize: "13px",
    },
    Card: {
      borderRadius: "10px",
      color: "rgba(245, 243, 255, 0.9)",
    },
    Button: {
      borderRadiusMedium: "6px",
      borderRadiusSmall: "4px",
    },
    Input: {
      borderRadius: "6px",
    },
    DataTable: {
      borderRadius: "8px",
    },
    Tag: {
      borderRadius: "4px",
    },
    Dialog: {
      borderRadius: "12px",
    },
  },
  darkOverrides: {
    common: {
      primaryColor: "#a78bfa",
      primaryColorHover: "#c4b5fd",
      primaryColorPressed: "#8b5cf6",
      primaryColorSuppl: "#a78bfa",
      borderRadius: "8px",
      borderRadiusSmall: "4px",
      fontFamily:
        '"JetBrains Mono", "SF Mono", Menlo, "PingFang SC", "Microsoft YaHei", monospace',
      fontSize: "13px",
    },
    Card: {
      borderRadius: "10px",
      color: "rgba(15, 12, 30, 0.92)",
    },
    Button: {
      borderRadiusMedium: "6px",
      borderRadiusSmall: "4px",
    },
    Input: {
      borderRadius: "6px",
    },
    DataTable: {
      borderRadius: "8px",
    },
    Tag: {
      borderRadius: "4px",
    },
    Dialog: {
      borderRadius: "12px",
    },
  },
  cssVars: {
    "--app-bg": "linear-gradient(160deg, #f5f3ff 0%, #ede9fe 100%)",
    "--app-card-bg": "rgba(245, 243, 255, 0.9)",
    "--app-card-border": "rgba(124, 58, 237, 0.15)",
    "--app-header-bg": "rgba(245, 243, 255, 0.85)",
    "--app-header-blur": "12px",
    "--app-header-shadow": "0 1px 8px rgba(124, 58, 237, 0.08)",
    "--app-sidebar-bg": "rgba(237, 233, 254, 0.6)",
    "--app-shadow-sm": "0 0 6px rgba(124, 58, 237, 0.06)",
    "--app-shadow-md": "0 2px 12px rgba(124, 58, 237, 0.08)",
    "--app-shadow-lg": "0 4px 24px rgba(124, 58, 237, 0.12)",
    "--app-hover-lift": "-2px",
    "--app-border-color": "rgba(124, 58, 237, 0.12)",
  },
  cssDarkVars: {
    "--app-bg": "linear-gradient(160deg, #0a0814 0%, #120e24 50%, #0f0c1e 100%)",
    "--app-card-bg": "rgba(15, 12, 30, 0.92)",
    "--app-card-border": "rgba(167, 139, 250, 0.2)",
    "--app-header-bg": "rgba(10, 8, 20, 0.85)",
    "--app-header-blur": "12px",
    "--app-header-shadow": "0 1px 12px rgba(167, 139, 250, 0.1)",
    "--app-sidebar-bg": "rgba(12, 10, 24, 0.8)",
    "--app-shadow-sm": "0 0 8px rgba(167, 139, 250, 0.08)",
    "--app-shadow-md": "0 2px 16px rgba(167, 139, 250, 0.12)",
    "--app-shadow-lg": "0 4px 32px rgba(167, 139, 250, 0.18)",
    "--app-hover-lift": "-2px",
    "--app-border-color": "rgba(167, 139, 250, 0.2)",
  },
};
