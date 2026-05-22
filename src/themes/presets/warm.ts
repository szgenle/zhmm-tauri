import type { ThemePreset } from "..";

export const warmPreset: ThemePreset = {
  key: "warm",
  label: "温暖圆润",
  description: "大圆角、柔和渐变、亲和力强",
  lightOverrides: {
    common: {
      primaryColor: "#f97316",
      primaryColorHover: "#fb923c",
      primaryColorPressed: "#ea580c",
      primaryColorSuppl: "#f97316",
      borderRadius: "14px",
      borderRadiusSmall: "10px",
      fontFamily:
        '"Nunito", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "16px",
      color: "#fffbf5",
    },
    Button: {
      borderRadiusMedium: "12px",
      borderRadiusSmall: "10px",
    },
    Input: {
      borderRadius: "12px",
    },
    DataTable: {
      borderRadius: "14px",
    },
    Tag: {
      borderRadius: "16px",
    },
    Dialog: {
      borderRadius: "18px",
    },
  },
  darkOverrides: {
    common: {
      primaryColor: "#fb923c",
      primaryColorHover: "#fdba74",
      primaryColorPressed: "#f97316",
      primaryColorSuppl: "#fb923c",
      borderRadius: "14px",
      borderRadiusSmall: "10px",
      fontFamily:
        '"Nunito", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "16px",
      color: "#1f1a14",
    },
    Button: {
      borderRadiusMedium: "12px",
      borderRadiusSmall: "10px",
    },
    Input: {
      borderRadius: "12px",
    },
    DataTable: {
      borderRadius: "14px",
    },
    Tag: {
      borderRadius: "16px",
    },
    Dialog: {
      borderRadius: "18px",
    },
  },
  cssVars: {
    "--app-bg": "linear-gradient(145deg, #fff7ed 0%, #fffbf5 50%, #fef3e2 100%)",
    "--app-card-bg": "#fffbf5",
    "--app-card-border": "rgba(249, 115, 22, 0.12)",
    "--app-header-bg": "rgba(255, 251, 245, 0.88)",
    "--app-header-blur": "12px",
    "--app-header-shadow": "0 2px 12px rgba(249, 115, 22, 0.06)",
    "--app-sidebar-bg": "rgba(255, 247, 237, 0.7)",
    "--app-shadow-sm": "0 2px 6px rgba(249, 115, 22, 0.04)",
    "--app-shadow-md": "0 4px 14px rgba(249, 115, 22, 0.07)",
    "--app-shadow-lg": "0 8px 28px rgba(249, 115, 22, 0.1)",
    "--app-hover-lift": "-3px",
    "--app-border-color": "rgba(249, 115, 22, 0.1)",
  },
  cssDarkVars: {
    "--app-bg": "linear-gradient(145deg, #1a1410 0%, #1f1a14 50%, #1c1610 100%)",
    "--app-card-bg": "#1f1a14",
    "--app-card-border": "rgba(251, 146, 60, 0.18)",
    "--app-header-bg": "rgba(26, 20, 16, 0.9)",
    "--app-header-blur": "12px",
    "--app-header-shadow": "0 2px 12px rgba(251, 146, 60, 0.08)",
    "--app-sidebar-bg": "rgba(22, 18, 12, 0.7)",
    "--app-shadow-sm": "0 2px 6px rgba(0, 0, 0, 0.2)",
    "--app-shadow-md": "0 4px 14px rgba(0, 0, 0, 0.25)",
    "--app-shadow-lg": "0 8px 28px rgba(0, 0, 0, 0.3)",
    "--app-hover-lift": "-3px",
    "--app-border-color": "rgba(251, 146, 60, 0.15)",
  },
};
