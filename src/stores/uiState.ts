import { computed, ref, type WritableComputedRef } from "vue";
import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";
import { safeInvoke, tauriAvailable } from "@/lib/tauri";

export const useUiStateStore=defineStore("uiState",()=>{
  const values=ref<Record<string,unknown>>({});
  const error=ref<string|null>(null);
  let initialized:Promise<void>|null=null;
  let pending=Promise.resolve();
  function set(key:string,value:unknown){
    values.value={...values.value,[key]:value};
    if(!tauriAvailable())return;
    pending=pending.then(async()=>{await safeInvoke("settings_set_ui_state",{key,value});error.value=null;}).catch(reason=>{error.value=`保存界面状态失败：${reason}`;});
  }
  function load(){
    if(initialized)return initialized;
    initialized=(async()=>{
      if(!tauriAvailable())return;
      await listen<{key:string;value:unknown}>("settings:ui-state",event=>{values.value={...values.value,[event.payload.key]:event.payload.value};});
      const settings=await safeInvoke<{uiState?:Record<string,unknown>}>("settings_get");
      values.value={...settings?.uiState,...values.value};
      if(values.value["panel.entries"]===undefined){
        try{const old=JSON.parse(localStorage.getItem("mytools-panel-entries-v1")||"null");if(Array.isArray(old))set("panel.entries",old);}catch{error.value="旧面板设置无法读取，原数据已保留";}
      }
    })().catch(reason=>{error.value=String(reason);initialized=null;});
    return initialized;
  }
  return{values,error,set,load,flush:()=>pending};
});

export function persistedUiValue<T>(key:string,fallback:T,valid:(value:unknown)=>value is T):WritableComputedRef<T>{
  const state=useUiStateStore();
  return computed({get:()=>valid(state.values[key])?state.values[key] as T:fallback,set:value=>state.set(key,value)});
}
