import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

export interface DirGrant {
  plugin: string;
  path: string;
}

export const useDirGrantsStore = defineStore("dirGrants", () => {
  const grants = ref<DirGrant[]>([]);
  const error = ref<string | null>(null);
  const loading = ref(false);

  async function load(): Promise<void> {
    if (!tauriAvailable()) {
      grants.value = [];
      return;
    }
    loading.value = true;
    try {
      const list = await safeInvoke<DirGrant[]>("dir_grants_list");
      grants.value = Array.isArray(list) ? list : [];
      error.value = null;
    } catch (err) {
      error.value = String(err);
    } finally {
      loading.value = false;
    }
  }

  async function revoke(plugin: string, path?: string): Promise<void> {
    try {
      await invoke("dir_grants_revoke", { plugin, path: path ?? null });
      error.value = null;
    } catch (err) {
      error.value = String(err);
      return;
    }
    await load();
  }

  return { grants, error, loading, load, revoke };
});
