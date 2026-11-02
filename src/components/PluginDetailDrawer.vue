<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useEventListener } from "@vueuse/core";
import PluginDetail from "@/components/PluginDetail.vue";

defineProps<{name:string}>();
const emit=defineEmits<{close:[]}>();
const panel=ref<HTMLElement | null>(null);
const closeButton=ref<HTMLButtonElement | null>(null);
const previousFocus=document.activeElement instanceof HTMLElement ? document.activeElement : null;

useEventListener(document,"keydown",event=>{
  if(document.querySelector(".app-consent"))return;
  if(event.key==="Escape"){event.preventDefault();event.stopPropagation();emit("close");return;}
  if(event.key!=="Tab")return;
  const controls=panel.value?.querySelectorAll<HTMLElement>('button:not(:disabled),a[href],input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]');
  if(!controls?.length)return;
  const first=controls[0],last=controls[controls.length-1];
  if(event.shiftKey && (document.activeElement===first || !panel.value?.contains(document.activeElement))){event.preventDefault();last?.focus();}
  else if(!event.shiftKey && (document.activeElement===last || !panel.value?.contains(document.activeElement))){event.preventDefault();first?.focus();}
});
onMounted(()=>{void nextTick(()=>closeButton.value?.focus());});
onBeforeUnmount(()=>{void nextTick(()=>{if(previousFocus?.isConnected)previousFocus.focus();});});
</script>

<template>
  <div class="drawer" @click.self="emit('close')">
    <section ref="panel" class="drawer__panel" role="dialog" aria-modal="true" aria-label="插件详情">
      <header class="drawer__header"><strong>插件详情</strong><button ref="closeButton" type="button" aria-label="关闭插件详情" title="关闭（Esc）" @click="emit('close')">×</button></header>
      <PluginDetail :name="name" embedded />
    </section>
  </div>
</template>

<style scoped>
.drawer{position:fixed;inset:0;z-index:30;display:flex;justify-content:flex-end;background:rgba(12,20,33,.32);overflow:hidden;border-radius:14px;}
.drawer__panel{display:flex;flex-direction:column;width:min(820px,82vw);height:100%;min-width:0;background:var(--panel-bg-strong);color:var(--panel-fg);border-left:1px solid var(--panel-border);box-shadow:-16px 0 45px #0002;}
.drawer__header{display:flex;align-items:center;justify-content:space-between;padding:15px 24px;border-bottom:1px solid var(--panel-border);flex:none;font-size:13px;}
.drawer__header button{width:30px;height:30px;border:0;border-radius:7px;background:transparent;color:var(--text-dim);font-size:25px;cursor:pointer;}
.drawer__header button:hover{background:var(--hover-bg);color:var(--panel-fg);}
.drawer__panel :deep(.pd){flex:1;min-height:0;padding:24px 28px;}
.detail-drawer-enter-active,.detail-drawer-leave-active{transition:opacity .2s ease;}
.detail-drawer-enter-active .drawer__panel,.detail-drawer-leave-active .drawer__panel{transition:transform .22s cubic-bezier(.2,.7,.2,1);}
.detail-drawer-enter-from,.detail-drawer-leave-to{opacity:0;}
.detail-drawer-enter-from .drawer__panel,.detail-drawer-leave-to .drawer__panel{transform:translateX(100%);}
@media(max-width:650px){.drawer__panel{width:94vw;}.drawer__panel :deep(.pd){padding:18px;}.drawer__panel :deep(.pd__layout){grid-template-columns:1fr;gap:0;}}
@media(prefers-reduced-motion:reduce){.detail-drawer-enter-active,.detail-drawer-leave-active,.detail-drawer-enter-active .drawer__panel,.detail-drawer-leave-active .drawer__panel{transition:none;}}
</style>
