import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

export interface CapabilityGrant {
  plugin: string;
  capability: string;
  grantedAt: string;
}

export const useGrantsStore = defineStore("grants", () => {
  const grants = ref<CapabilityGrant[]>([]);
  const error = ref<string | null>(null);
  const loading = ref(false);

  async function load(): Promise<void> {
    if (!tauriAvailable()) {
      grants.value = [];
      return;
    }
    loading.value = true;
    try {
      const list = await safeInvoke<CapabilityGrant[]>("plugin_capability_grants");
      grants.value = Array.isArray(list) ? list : [];
      error.value = null;
    } catch (err) {
      error.value = String(err);
    } finally {
      loading.value = false;
    }
  }

  async function revoke(plugin: string, capability: string): Promise<void> {
    try {
      await invoke("plugin_revoke_capability", { plugin, capability });
      error.value = null;
    } catch (err) {
      error.value = String(err);
    }
    await load();
  }

  return { grants, error, loading, load, revoke };
});
