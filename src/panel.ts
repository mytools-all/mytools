import { createApp } from "vue";
import { createPinia } from "pinia";
import PanelHost from "./PanelHost.vue";
import { useSettingsStore } from "@/stores/settings";
import { useUiStateStore } from "@/stores/uiState";
import "./styles/global.css";

const app = createApp(PanelHost);
const pinia = createPinia();
app.use(pinia);
void Promise.allSettled([useSettingsStore(pinia).init(), useUiStateStore(pinia).load()]).finally(() => app.mount("#app"));
