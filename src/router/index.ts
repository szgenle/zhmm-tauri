import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
import { api } from "../api";

const routes: RouteRecordRaw[] = [
  {
    path: "/login",
    name: "login",
    component: () => import("../views/LoginView.vue"),
    meta: { public: true },
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
      return { path: "/login" };
    }
  } catch {
    return { path: "/login" };
  }
  return true;
});

export default router;
