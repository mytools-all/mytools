<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useCatalogSourcesStore } from "@/stores/catalogSources";
import { useToolCatalogStore } from "@/stores/toolCatalog";
import defaults from "@/config/catalog.json";
const sources = useCatalogSourcesStore();
const catalog = useToolCatalogStore();
const primary = ref(sources.pluginList);
const mirror = ref(sources.pluginListMirror);
const busy = ref(false);
const message = ref("");
watch(() => sources.pluginList, value => { primary.value = value; });
watch(() => sources.pluginListMirror, value => { mirror.value = value; });
onMounted(() => { void sources.load().catch(reason => { message.value = String(reason); }); });
async function save() {
  busy.value = true;
  message.value = "";
  try {
    await sources.save(primary.value, mirror.value);
    await catalog.refresh(true);
    message.value = catalog.loadError || "仓库配置已保存，插件列表已更新";
  } catch (reason) { message.value = String(reason); }
  finally { busy.value = false; }
}
function reset() {
  primary.value = defaults.pluginList;
  mirror.value = defaults.pluginListMirror;
}
</script>

<template>
  <section class="sources" aria-label="插件仓库设置">
    <h4>插件仓库</h4>
    <p>优先使用插件列表；访问失败或内容无效时自动使用镜像。两个仓库均不可用时保留已有列表。</p>
    <label>插件列表 <span>主仓库</span><input v-model="primary" type="url" :disabled="busy" aria-label="插件列表地址" /></label>
    <label>插件列表镜像 <span>备用仓库</span><input v-model="mirror" type="url" :disabled="busy" aria-label="插件列表镜像地址" /></label>
    <p class="sources__active">当前使用：{{ sources.activeUrl || '尚未连接远程仓库，使用已有列表' }}</p>
    <div class="sources__actions">
      <button class="pc__btn" :disabled="busy || catalog.loading" @click="save">{{ busy ? '连接中…' : '保存并刷新' }}</button>
      <button class="pc__btn" :disabled="busy" @click="reset">恢复默认地址</button>
    </div>
    <p v-if="message" role="status">{{ message }}</p>
  </section>
</template>

<style scoped src="./plugin-center.css"></style>
<style scoped>
.sources { padding-bottom:20px; margin-bottom:20px; border-bottom:1px solid var(--panel-control-border); }
h4 { margin:0 0 12px; font-size:14px; }
p { color:var(--text-dim); font-size:12px; line-height:1.6; overflow-wrap:anywhere; }
label { display:block; margin:14px 0; font-size:13px; }
label span { color:var(--text-dim); margin-left:8px; font-size:12px; }
input { display:block; box-sizing:border-box; width:100%; height:36px; margin-top:8px; padding:0 10px; border:1px solid var(--panel-control-border); border-radius:6px; background:var(--panel-control-bg); color:var(--panel-fg); font:inherit; font-size:12px; }
input:focus { outline:1px solid var(--accent); }
.sources__actions { display:flex; flex-wrap:wrap; gap:8px; }
</style>
