<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useUiStateStore } from "@/stores/uiState";

interface Plugin { id: string; title: string; version: string | null }
interface RestoreResult { success: boolean; error?: string; report?: { recoveryBackup: string } }
interface Preview {
  token: string;
  manifest: { hostVersion: string; createdAt: string; plugins: Plugin[]; filesCount: number; hasHost: boolean; hasClipboard: boolean; hasPrograms: boolean };
  warnings: string[];
  hasExistingData: boolean;
}
const ui = useUiStateStore();
const plugins = ref<Plugin[]>([]);
const mode = ref("all");
const selected = ref<string[]>([]);
const includePrograms = ref(false);
const busy = ref(false);
const message = ref("");
const error = ref("");
const preview = ref<Preview | null>(null);
const restoreHost = ref(false);
const restoreClipboard = ref(false);
const restorePlugins = ref<string[]>([]);
const restorePrograms = ref(false);
const confirming = ref(false);
const cancelButton = ref<HTMLButtonElement>();
const lastResult = ref<RestoreResult | null>(null);
let alive = true;
let previousFocus: HTMLElement | null = null;
const canRestore = computed(() => restoreHost.value || restoreClipboard.value || restorePlugins.value.length > 0);
const zipFilters = [{ name: "ZIP 备份", extensions: ["zip"] }];

onMounted(async () => {
  try {
    const info = await invoke<{ plugins: Plugin[]; lastRestore: RestoreResult | null }>("backup_info");
    if (alive) { plugins.value = info.plugins; lastResult.value = info.lastRestore; }
  } catch (reason) { error.value = String(reason); }
});
onBeforeUnmount(() => { alive = false; discard(); });
function discard() {
  const token = preview.value?.token;
  preview.value = null;
  if (token) void invoke("backup_cancel_preview", { token }).catch(() => undefined);
}
async function run(action: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true; error.value = ""; message.value = "";
  try { await action(); } catch (reason) { error.value = String(reason); }
  finally { busy.value = false; }
}
function stamp() { return new Date().toISOString().replace(/[:.]/g, "-"); }
function exportBackup() {
  void run(async () => {
    const path = await save({ defaultPath: `MyTools-${stamp()}.zip`, filters: zipFilters });
    if (!path) return;
    await ui.flush();
    if (ui.error) throw new Error(ui.error);
    await invoke("backup_export", { path, selection: { mode: mode.value, plugins: selected.value, includePrograms: includePrograms.value } });
    message.value = `备份已保存：${path}`;
  });
}
function chooseRestore() {
  void run(async () => {
    const path = await open({ multiple: false, filters: zipFilters });
    if (typeof path !== "string") return;
    const result = await invoke<Preview>("backup_preview", { path });
    if (!alive) { void invoke("backup_cancel_preview", { token: result.token }); return; }
    discard(); preview.value = result;
    restoreHost.value = result.manifest.hasHost;
    restoreClipboard.value = result.manifest.hasClipboard;
    restorePlugins.value = result.manifest.plugins.map(plugin => plugin.id);
    restorePrograms.value = false;
  });
}
async function askRestore() {
  previousFocus = document.activeElement as HTMLElement | null;
  confirming.value = true;
  await nextTick(); cancelButton.value?.focus();
}
function closeConfirmation() {
  confirming.value = false; void nextTick(() => previousFocus?.focus());
}
function trap(event: KeyboardEvent) {
  if (event.key === "Escape" && !busy.value) { event.preventDefault(); closeConfirmation(); }
  if (event.key !== "Tab") return;
  const buttons = [...(event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
  event.preventDefault(); buttons[(index + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length]?.focus();
}
function restore() {
  void run(async () => {
    await invoke("backup_restore", { token: preview.value?.token, confirmed: true, options: { host: restoreHost.value, clipboard: restoreClipboard.value, plugins: restorePlugins.value, programs: restorePrograms.value } });
    message.value = "正在重启并恢复，请稍候…";
  });
}
function diagnostics() {
  void run(async () => {
    const path = await save({ defaultPath: `MyTools-diagnostics-${stamp()}.zip`, filters: zipFilters });
    if (!path) return;
    await invoke("backup_diagnostics", { path }); message.value = `诊断文件已保存：${path}`;
  });
}
</script>

<template>
  <div class="backup-content" :inert="confirming">
    <p v-if="lastResult" role="status" :class="lastResult.success ? 'pc__notice' : 'pc__message'">
      {{ lastResult.success ? '上次恢复已完成。恢复前的安全备份：' + lastResult.report?.recoveryBackup : '上次恢复未完成：' + lastResult.error }}
    </p>
    <section class="pc__settings-block">
      <h4 class="pc__side-h">备份数据</h4>
      <p class="pc__hint">请先保存内容并关闭正在使用的插件。备份使用 ZIP 格式，包含选择范围内的数据与设置。</p>
      <select v-model="mode" class="backup-select" :disabled="busy" aria-label="备份范围">
        <option value="all">所有用户数据</option><option value="host">仅 MyTools 配置</option>
        <option value="plugins">所有插件的数据与设置</option><option value="selected">所选插件的数据与设置</option>
      </select>
      <div v-if="mode === 'selected'" class="backup-list">
        <label v-for="plugin in plugins" :key="plugin.id"><input v-model="selected" type="checkbox" :value="plugin.id" :disabled="busy">{{ plugin.title }}</label>
        <span v-if="!plugins.length">暂无插件数据</span>
      </div>
      <label v-if="mode !== 'host'"><input v-model="includePrograms" type="checkbox" :disabled="busy">同时备份插件程序（便于离线恢复，文件会更大）</label>
      <p class="pc__hint">全部数据包含剪贴板历史；插件范围包含待办、识别历史等业务数据。不迁移目录授权、开机自启任务或浏览器登录状态。备份可能包含插件保存的敏感数据，请妥善保管。</p>
      <div><button class="pc__btn pc__btn--primary" :disabled="busy || (mode === 'selected' && !selected.length)" @click="exportBackup">导出备份</button></div>
    </section>
    <section class="pc__settings-block">
      <h4 class="pc__side-h">恢复数据</h4>
      <div><button class="pc__btn" :disabled="busy" @click="chooseRestore">选择 ZIP 备份…</button></div>
      <template v-if="preview">
        <p class="pc__hint">备份于 {{ new Date(preview.manifest.createdAt).toLocaleString() }} · MyTools {{ preview.manifest.hostVersion }} · {{ preview.manifest.filesCount }} 个文件</p>
        <p v-for="warning in preview.warnings" :key="warning" class="pc__hint">{{ warning }}</p>
        <label v-if="preview.manifest.hasHost"><input v-model="restoreHost" type="checkbox" :disabled="busy">MyTools 配置</label>
        <label v-if="preview.manifest.hasClipboard"><input v-model="restoreClipboard" type="checkbox" :disabled="busy">剪贴板历史</label>
        <div class="backup-list"><label v-for="plugin in preview.manifest.plugins" :key="plugin.id"><input v-model="restorePlugins" type="checkbox" :value="plugin.id" :disabled="busy">{{ plugin.title }} 的数据与设置</label></div>
        <label v-if="preview.manifest.hasPrograms"><input v-model="restorePrograms" type="checkbox" :disabled="busy">同时恢复所选插件的程序</label>
        <p class="pc__hint">仅覆盖所选项目，其他插件保留。恢复前会自动创建安全备份，随后重启 MyTools。</p>
        <div class="pc__row"><button class="pc__btn pc__btn--primary" :disabled="busy || !canRestore" @click="askRestore">恢复所选内容…</button><button class="pc__btn" :disabled="busy" @click="discard">取消预览</button></div>
      </template>
    </section>
    <section class="pc__settings-block">
      <h4 class="pc__side-h">运行日志</h4>
      <p class="pc__hint">记录启动、插件管理和备份恢复等操作。诊断包包含版本、环境概况和近期操作结果，不含剪贴板、插件内容及凭据。</p>
      <div class="pc__row"><button class="pc__btn" :disabled="busy" @click="run(async () => { await invoke('backup_open_logs'); })">打开日志目录</button><button class="pc__btn" :disabled="busy" @click="diagnostics">导出诊断信息…</button></div>
    </section>
    <p v-if="busy" role="status" class="pc__hint">正在处理，请稍候…</p>
    <p v-if="message" role="status" class="pc__notice">{{ message }}</p>
    <p v-if="error" role="alert" class="pc__message">{{ error }}</p>
  </div>
  <Teleport to="body">
    <div v-if="confirming" class="backup-mask" @keydown="trap">
      <section class="backup-dialog" role="alertdialog" aria-modal="true" aria-labelledby="backup-title" aria-describedby="backup-description">
        <h3 id="backup-title">确认恢复所选内容？</h3>
        <p id="backup-description">{{ preview?.hasExistingData ? '当前已有数据，所选项目将被备份中的内容覆盖。' : '即将写入所选备份内容。' }}请先保存并关闭插件中的编辑内容。MyTools 将创建安全备份并重启，恢复失败时自动回退。</p>
        <p v-if="error" role="alert">{{ error }}</p>
        <div class="pc__row"><button ref="cancelButton" class="pc__btn" :disabled="busy" @click="closeConfirmation">取消</button><button class="pc__btn pc__btn--primary" :disabled="busy" @click="restore">{{ busy ? '正在准备…' : '确认覆盖并重启' }}</button></div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped src="./plugin-center.css"></style>
<style scoped>
.backup-content{display:flex;flex-direction:column;gap:16px;font-size:13px}.backup-list{display:flex;flex-direction:column;gap:8px;max-height:180px;overflow:auto}.backup-content label{display:flex;gap:8px;align-items:center}.backup-select{padding:8px;border:1px solid var(--panel-control-border);border-radius:6px;background:var(--panel-bg);color:var(--panel-fg)}.backup-mask{position:fixed;inset:0;background:#0008;z-index:10000;display:grid;place-items:center;padding:24px}.backup-dialog{max-width:480px;padding:24px;border:1px solid var(--panel-control-border);border-radius:12px;background:var(--panel-bg);color:var(--panel-fg);font-size:13px;line-height:1.7}.backup-dialog h3{margin-top:0}.backup-dialog .pc__row{justify-content:flex-end}
.backup-content .pc__hint{margin:0;line-height:1.65;align-self:stretch;text-align:left}
</style>
