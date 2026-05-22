import type { ThemePreset } from "..";

export const glassPreset: ThemePreset = {
  key: "glass",
  label: "清透玻璃",
  description: "半透明卡片、柔和阴影、清爽配色",
  lightOverrides: {
    common: {
      primaryColor: "#4098fc",
      primaryColorHover: "#5aabff",
      primaryColorPressed: "#2b7de9",
      primaryColorSuppl: "#4098fc",
      borderRadius: "10px",
      borderRadiusSmall: "6px",
      fontFamily:
        'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "12px",
      color: "rgba(255, 255, 255, 0.72)",
    },
    Button: {
      borderRadiusMedium: "8px",
      borderRadiusSmall: "6px",
    },
    Input: {
      borderRadius: "8px",
    },
    DataTable: {
      borderRadius: "10px",
    },
    Tag: {
      borderRadius: "12px",
    },
    Dialog: {
      borderRadius: "14px",
    },
  },
  darkOverrides: {
    common: {
      primaryColor: "#5aabff",
      primaryColorHover: "#7abfff",
      primaryColorPressed: "#4098fc",
      primaryColorSuppl: "#5aabff",
      borderRadius: "10px",
      borderRadiusSmall: "6px",
      fontFamily:
        'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "12px",
      color: "rgba(40, 44, 52, 0.78)",
    },
    Button: {
      borderRadiusMedium: "8px",
      borderRadiusSmall: "6px",
    },
    Input: {
      borderRadius: "8px",
    },
    DataTable: {
      borderRadius: "10px",
    },
    Tag: {
      borderRadius: "12px",
    },
    Dialog: {
      borderRadius: "14px",
    },
  },
  cssVars: {
    "--app-bg": "linear-gradient(135deg, #f0f4ff 0%, #fafbff 50%, #f5f0ff 100%)",
    "--app-card-bg": "rgba(255, 255, 255, 0.72)",
    "--app-card-border": "rgba(255, 255, 255, 0.9)",
    "--app-header-bg": "rgba(255, 255, 255, 0.65)",
    "--app-header-blur": "16px",
    "--app-header-shadow": "0 1px 12px rgba(0, 0, 0, 0.06)",
    "--app-sidebar-bg": "rgba(248, 250, 255, 0.6)",
    "--app-shadow-sm": "0 2px 8px rgba(0, 0, 0, 0.04)",
    "--app-shadow-md": "0 4px 16px rgba(0, 0, 0, 0.06)",
    "--app-shadow-lg": "0 8px 32px rgba(0, 0, 0, 0.08)",
    "--app-hover-lift": "-2px",
    "--app-border-color": "rgba(0, 0, 0, 0.06)",
  },
  cssDarkVars: {
    "--app-bg": "linear-gradient(135deg, #1a1c24 0%, #1e2028 50%, #1c1a26 100%)",
    "--app-card-bg": "rgba(40, 44, 52, 0.78)",
    "--app-card-border": "rgba(255, 255, 255, 0.08)",
    "--app-header-bg": "rgba(30, 32, 40, 0.72)",
    "--app-header-blur": "16px",
    "--app-header-shadow": "0 1px 12px rgba(0, 0, 0, 0.3)",
    "--app-sidebar-bg": "rgba(26, 28, 36, 0.6)",
    "--app-shadow-sm": "0 2px 8px rgba(0, 0, 0, 0.2)",
    "--app-shadow-md": "0 4px 16px rgba(0, 0, 0, 0.25)",
    "--app-shadow-lg": "0 8px 32px rgba(0, 0, 0, 0.35)",
    "--app-hover-lift": "-2px",
    "--app-border-color": "rgba(255, 255, 255, 0.08)",
  },
};
