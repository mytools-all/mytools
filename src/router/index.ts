import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import LauncherView from "@/views/LauncherView.vue";

const routes: RouteRecordRaw[] = [
  { path: "/panel", name: "my-panel", component: () => import("@/views/MyPanelView.vue") },
  {
    path: "/",
    name: "launcher",
    component: LauncherView,
  },
  {
    path: "/tool/calculator",
    name: "calculator",
    component: () => import("@/views/tools/CalculatorView.vue"),
  },
  {
    path: "/tool/json-formatter",
    name: "json-formatter",
    component: () => import("@/views/tools/JsonFormatterView.vue"),
  },
  {
    path: "/tool/base64-url",
    name: "base64-url",
    component: () => import("@/views/tools/Base64UrlView.vue"),
  },
  {
    path: "/tool/timestamp-uuid",
    name: "timestamp-uuid",
    component: () => import("@/views/tools/TimestampUuidView.vue"),
  },
  {
    path: "/tool/color-picker",
    name: "color-picker",
    component: () => import("@/views/tools/ColorPickerView.vue"),
  },
  {
    path: "/tool/clipboard",
    name: "clipboard",
    component: () => import("@/views/tools/ClipboardView.vue"),
  },
  {
    path: "/plugin/:id/:code",
    name: "plugin",
    component: () => import("@/views/tools/PluginView.vue"),
  },
  {
    path: "/plugins",
    name: "plugins",
    component: () => import("@/views/PluginManagerView.vue"),
  },
  {
    path: "/settings",
    redirect: { path: "/plugins", query: { section: "settings" } },
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
