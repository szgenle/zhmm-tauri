import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
import { api } from "../api";

const routes: RouteRecordRaw[] = [
  {
    path: "/files",
    name: "files",
    component: () => import("../views/FileListView.vue"),
    meta: { public: true },
  },
  // 兼容历史路径：旧的 /login 跳转到新的 /files
  {
    path: "/login",
    redirect: "/files",
  },
  {
    path: "/",
    component: () => import("../layouts/MainLayout.vue"),
    children: [
      {
        path: "",
        name: "passwords",
        component: () => import("../views/PasswordListView.vue"),
        meta: { title: "账号管理" },
      },
      {
        path: "role-management",
        name: "role-management",
        component: () => import("../views/RoleManagementView.vue"),
        meta: { title: "分类管理" },
      },
      {
        path: "data-management",
        name: "data-management",
        component: () => import("../views/DataManagementView.vue"),
        meta: { title: "数据管理" },
      },
      {
        path: "settings",
        name: "settings",
        component: () => import("../views/SettingsView.vue"),
        meta: { title: "系统设置" },
      },
    ],
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach(async (to) => {
  if (to.meta.public) return true;
  try {
    const status = await api.vaultStatus();
    if (!status.unlocked) {
      return { path: "/files" };
    }
  } catch {
    return { path: "/files" };
  }
  return true;
});

export default router;
