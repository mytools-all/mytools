import { createApp } from "vue";
import { createPinia } from "pinia";
import FloatballView from "./views/FloatballView.vue";
import { useSettingsStore } from "@/stores/settings";
import "./styles/global.css";

const app = createApp(FloatballView);
const pinia = createPinia();
app.use(pinia);
void useSettingsStore(pinia).init().finally(() => app.mount("#app"));
