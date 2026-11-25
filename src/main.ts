import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "@/router";
import { installMyToolsApi } from "@/composables/mytools";
import { useSettingsStore } from "@/stores/settings";
import { tauriAvailable } from "@/lib/tauri";
import { useUiStateStore } from "@/stores/uiState";
import "./styles/global.css";

console.log("[MyTools] tauriAvailable:", tauriAvailable());
installMyToolsApi();
const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);

const settings = useSettingsStore(pinia);
void Promise.allSettled([settings.init(),useUiStateStore(pinia).load()]).finally(() => {
  app.mount("#app");
});
