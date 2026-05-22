import type { ThemePreset } from "../index";

export const minimalPreset: ThemePreset = {
  key: "minimal",
  label: "极简纯净",
  description: "大留白、细线条、黑白灰 + 强调色",
  lightOverrides: {
    common: {
      primaryColor: "#111111",
      primaryColorHover: "#333333",
      primaryColorPressed: "#000000",
      primaryColorSuppl: "#111111",
      borderRadius: "6px",
      borderRadiusSmall: "4px",
      fontFamily:
        '"SF Pro Text", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "8px",
      color: "#ffffff",
    },
    Button: {
      borderRadiusMedium: "6px",
      borderRadiusSmall: "4px",
    },
    Input: {
      borderRadius: "6px",
    },
    DataTable: {
      borderRadius: "6px",
    },
    Tag: {
      borderRadius: "4px",
    },
    Dialog: {
      borderRadius: "10px",
    },
  },
  darkOverrides: {
    common: {
      primaryColor: "#ffffff",
      primaryColorHover: "#e0e0e0",
      primaryColorPressed: "#cccccc",
      primaryColorSuppl: "#ffffff",
      borderRadius: "6px",
      borderRadiusSmall: "4px",
      fontFamily:
        '"SF Pro Text", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      fontSize: "14px",
    },
    Card: {
      borderRadius: "8px",
      color: "#1c1c1e",
    },
    Button: {
      borderRadiusMedium: "6px",
      borderRadiusSmall: "4px",
    },
    Input: {
      borderRadius: "6px",
    },
    DataTable: {
      borderRadius: "6px",
    },
    Tag: {
      borderRadius: "4px",
    },
    Dialog: {
      borderRadius: "10px",
    },
  },
  cssVars: {
    "--app-bg": "#fafafa",
    "--app-card-bg": "#ffffff",
    "--app-card-border": "#e8e8e8",
    "--app-header-bg": "rgba(250, 250, 250, 0.9)",
    "--app-header-blur": "8px",
    "--app-header-shadow": "0 1px 0 rgba(0, 0, 0, 0.05)",
    "--app-sidebar-bg": "#f5f5f5",
    "--app-shadow-sm": "none",
    "--app-shadow-md": "0 1px 3px rgba(0, 0, 0, 0.04)",
    "--app-shadow-lg": "0 2px 8px rgba(0, 0, 0, 0.06)",
    "--app-hover-lift": "-1px",
    "--app-border-color": "#e8e8e8",
  },
  cssDarkVars: {
    "--app-bg": "#111111",
    "--app-card-bg": "#1c1c1e",
    "--app-card-border": "#2c2c2e",
    "--app-header-bg": "rgba(17, 17, 17, 0.92)",
    "--app-header-blur": "8px",
    "--app-header-shadow": "0 1px 0 rgba(255, 255, 255, 0.05)",
    "--app-sidebar-bg": "#161616",
    "--app-shadow-sm": "none",
    "--app-shadow-md": "0 1px 3px rgba(0, 0, 0, 0.2)",
    "--app-shadow-lg": "0 2px 8px rgba(0, 0, 0, 0.3)",
    "--app-hover-lift": "-1px",
    "--app-border-color": "#2c2c2e",
  },
};
