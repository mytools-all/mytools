<script setup lang="ts">
import { fallbackIconDataUri } from "@/lib/iconFallback";
import { ref } from "vue";
import { useEventListener } from "@vueuse/core";

export interface TileItem {
  id: string;
  name: string;
  subtitle?: string;
  icon?: string;
}

defineProps<{
  title?: string;
  items: TileItem[];
  emptyText?: string;
  pinnable?: boolean;
  pinnedIds?: Set<string>;
  removable?: boolean;
  pinOnHover?: boolean;
  panelable?: boolean;
  panelIds?: Set<string>;
}>();

const emit = defineEmits<{
  open: [id: string];
  pin: [id: string];
  remove: [id: string];
  panel: [id: string];
  unpanel: [id: string];
}>();

function iconFor(item: TileItem): string {
  return item.icon || fallbackIconDataUri(item.name);
}
const menu = ref<{item:TileItem;x:number;y:number} | null>(null);
function openMenu(event: MouseEvent, item: TileItem) {
  menu.value = {item,x:Math.max(0,Math.min(event.clientX,innerWidth-210)),y:Math.max(0,Math.min(event.clientY,innerHeight-190))};
}
useEventListener(document,"pointerdown",()=>{menu.value=null;});
useEventListener(document,"keydown",event=>{if(event.key==="Escape")menu.value=null;});
</script>

<template>
  <section class="ptiles">
    <h3 v-if="title" class="ptiles__title">{{ title }} <span class="ptiles__count">{{ items.length }}</span></h3>
    <p v-if="items.length === 0 && emptyText" class="ptiles__empty">
      {{ emptyText }}
    </p>
    <ul v-else class="ptiles__grid">
      <li
        v-for="item in items"
        :key="item.id"
        class="ptiles__tile"
        role="button"
        tabindex="0"
        @keydown.enter.self="emit('open', item.id)"
        @keydown.space.self.prevent="emit('open', item.id)"
        :title="item.subtitle"
        @click="emit('open', item.id)"
        @contextmenu.prevent.stop="openMenu($event, item)"
      >
        <img
          :src="iconFor(item)"
          :alt="item.name"
          class="ptiles__icon"
          draggable="false"
        />
        <span class="ptiles__name">{{ item.name }}</span>
        <span v-if="item.subtitle" class="ptiles__sub">{{ item.subtitle }}</span>
        <button
          v-if="pinnable"
          type="button"
          class="ptiles__pin"
          :class="{ 
            'ptiles__pin--on': pinnedIds?.has(item.id),
            'ptiles__pin--hover-only': pinOnHover 
          }"
          :title="pinnedIds?.has(item.id) ? '取消固定' : '固定'"
          :aria-label="pinnedIds?.has(item.id) ? '取消固定' : '固定'"
          @click.stop="emit('pin', item.id)"
        >
          {{ pinnedIds?.has(item.id) ? "★" : "☆" }}
        </button>
        <button
          v-if="removable"
          type="button"
          class="ptiles__remove"
          title="从最近记录中移除"
          aria-label="从最近记录中移除"
          @click.stop="emit('remove', item.id)"
        >
          ×
        </button>
      </li>
    </ul>
    <Teleport to="body">
      <div v-if="menu" class="ptiles__menu" role="menu" :style="{left:menu.x+'px',top:menu.y+'px'}" @pointerdown.stop @contextmenu.prevent>
        <button role="menuitem" @click="emit('open',menu.item.id); menu=null">打开</button>
        <button v-if="pinnable" role="menuitem" @click="emit('pin',menu.item.id);menu=null">{{ pinnedIds?.has(menu.item.id) ? '取消固定' : '固定到主页' }}</button>
        <button v-if="panelable && !menu.item.id.startsWith('app:')" role="menuitem" @click="panelIds?.has(menu.item.id) ? emit('unpanel',menu.item.id) : emit('panel',menu.item.id);menu=null">{{ panelIds?.has(menu.item.id) ? '从我的面板移除' : '添加到我的面板' }}</button>
        <button v-if="removable" role="menuitem" @click="emit('remove',menu.item.id);menu=null">从最近使用中移除</button>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.ptiles__menu {position:fixed;z-index:10000;width:200px;padding:5px;border:1px solid var(--panel-border);border-radius:9px;background:var(--panel-bg-strong);box-shadow:0 8px 25px #0003;}
.ptiles__menu button {display:block;width:100%;text-align:left;padding:9px;border:0;border-radius:5px;background:transparent;color:var(--panel-fg);cursor:pointer;font-size:12px;}
.ptiles__menu button:hover {background:var(--hover-bg);}
.ptiles {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ptiles__title {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-dim-strong);
  letter-spacing: 0.04em;
}

.ptiles__empty {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim-strong);
  border: 1px solid var(--panel-border);
  background: var(--chip-bg);
  border-radius: 8px;
  padding: 12px;
  text-align: center;
}

.ptiles__grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 10px;
}

.ptiles__tile {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 12px 8px 10px;
  border: 1px solid var(--panel-control-border);
  border-radius: 10px;
  cursor: pointer;
  background: var(--surface-bg);
  overflow: hidden;
  transition: background 0.08s ease, border-color 0.08s ease;
}

.ptiles__tile:hover {
  background: var(--hover-bg);
  border-color: var(--accent);
}

.ptiles__icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  object-fit: contain;
  background: var(--hover-bg);
}

.ptiles__name {
  font-size: 12px;
  color: var(--panel-fg);
  font-weight: 500;
  text-align: center;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ptiles__sub {
  font-size: 10px;
  color: var(--text-dim-strong);
  text-align: center;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ptiles__pin {
  position: absolute;
  top: 4px;
  right: 4px;
  border: none;
  background: transparent;
  color: var(--text-dim-strong);
  font-size: 13px;
  cursor: pointer;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
}

.ptiles__pin:hover {
  color: #e5c07b;
}

.ptiles__pin--on {
  color: #e5c07b;
}

.ptiles__pin--hover-only {
  opacity: 0;
}

.ptiles__tile:focus-within .ptiles__pin--hover-only,
.ptiles__tile:hover .ptiles__pin--hover-only {
  opacity: 1;
}

.ptiles__pin--hover-only.ptiles__pin--on {
  opacity: 1;
}

.ptiles__remove {
  position: absolute;
  top: 4px;
  left: 4px;
  border: none;
  background: rgba(0, 0, 0, 0.4);
  color: var(--panel-fg);
  font-size: 12px;
  cursor: pointer;
  line-height: 1;
  padding: 2px 5px;
  border-radius: 4px;
  opacity: 0;
  transition: opacity 0.08s ease;
}

.ptiles__tile:focus-within .ptiles__remove,
.ptiles__tile:hover .ptiles__remove {
  opacity: 1;
}

.ptiles__remove:hover {
  background: rgba(239, 68, 68, 0.8);
}
.ptiles__count { font-size: 10px; font-weight: 400; color: var(--text-dim-strong); }
.ptiles__tile:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
</style>
