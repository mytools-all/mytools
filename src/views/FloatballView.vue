<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";
import appIcon from "../../src-tauri/icons/128x128.png";

const DRAG_THRESHOLD = 4;
const dragging = ref(false);
const error = ref("");
let startScreen = { x: 0, y: 0 };
let startPos = { x: 0, y: 0 };
let moved = false;
let scale = 1;
let positionReady: Promise<void> = Promise.resolve();
let moveTask: Promise<void> = Promise.resolve();
let queuedPosition: {x:number; y:number} | null = null;
let moving = false;

async function open(command: string) {
  if (!tauriAvailable()) return;
  error.value = "";
  try { await invoke(command); }
  catch { error.value = "打开失败，请重试"; }
}

function queueMove(position: {x:number; y:number}) {
  queuedPosition = position;
  if (moving) return;
  moving = true;
  moveTask = (async () => {
    try {
      while (queuedPosition) {
        const point = queuedPosition;
        queuedPosition = null;
        await getCurrentWebviewWindow().setPosition(new PhysicalPosition(point.x,point.y));
      }
    } catch { error.value = "移动失败，请重试"; queuedPosition = null; }
    finally { moving = false; }
  })();
}

async function onPointerDown(event: PointerEvent) {
  if (!tauriAvailable() || event.button > 0 || dragging.value) return;
  error.value = "";
  dragging.value = true;
  moved = false;
  startScreen = { x: event.screenX, y: event.screenY };
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  positionReady = (async () => { try {
    const win = getCurrentWebviewWindow();
    const pos = await win.outerPosition();
    startPos = { x: pos.x, y: pos.y };
    scale = await (win.scaleFactor?.() ?? Promise.resolve(1));
  } catch {
    dragging.value = false;
    error.value = "无法读取悬浮球位置";
  } })();
  await positionReady;
}

async function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const dx = event.screenX - startScreen.x;
  const dy = event.screenY - startScreen.y;
  if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) {
    moved = true;
  }
  if (!moved) return;
  await positionReady;
  if (error.value) return;
  queueMove({x:Math.round(startPos.x + dx * scale),y:Math.round(startPos.y + dy * scale)});
}

async function endDrag(cancelled = false) {
  if (!tauriAvailable()) return;
  dragging.value = false;
  await positionReady;
  await moveTask;
  if (error.value) return;
  if (moved) {
    try {
      const pos = await getCurrentWebviewWindow().outerPosition();
      await invoke("floatball_save_position", { x: pos.x, y: pos.y });
    } catch { error.value = "位置保存失败，请重试"; }
  } else if (!cancelled) await open("floatball_toggle_main");
}

function onPointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  if (event.button > 0) return;
  void endDrag();
}

function onPointerCancel() {
  if (!dragging.value) return;
  void endDrag(true);
}

function onKeydown(event: KeyboardEvent) {
  if (!["Enter", " "].includes(event.key)) return;
  event.preventDefault();
  if (!event.repeat) void open(event.shiftKey ? "panel_show" : "floatball_toggle_main");
}
</script>

<template>
  <div
    class="floatball"
    :class="{'floatball--dragging':dragging,'floatball--error':error}"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerCancel"
    @lostpointercapture="onPointerCancel"
    @contextmenu.prevent="open('panel_show')"
  >
    <button type="button" class="floatball__ball" aria-label="MyTools 悬浮球：单击搜索，右键我的面板" :title="error || '单击搜索 · 右键我的面板 · 拖动移动'" @keydown="onKeydown" @click="event => { if (event.detail === 0) open('floatball_toggle_main'); }">
      <span class="floatball__rim" aria-hidden="true"></span>
      <img class="floatball__logo" :src="appIcon" alt="" draggable="false" />
      <span class="floatball__panel" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
    </button>
    <span v-if="error" class="floatball__status" role="status">{{ error }}</span>
  </div>
</template>

<style scoped>
.floatball {
  width: 100vw;
  height: 100vh;
  background: transparent;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
  -webkit-user-select: none;
  cursor: grab;
  touch-action: none;
}

.floatball:active {
  cursor: grabbing;
}

.floatball__ball {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: min(48px, calc(100vw - 12px));
  height: min(48px, calc(100vh - 12px));
  flex-shrink: 0;
  padding: 0;
  border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
  border-radius: 35%;
  background: linear-gradient(145deg, var(--surface-bg), var(--panel-bg-strong));
  box-shadow:
    0 4px 9px rgba(0, 0, 0, 0.20),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
  color: #fff;
  font-size: 18px;
  line-height: 1;
  cursor: grab;
  transition: transform .18s ease, border-color .18s ease, box-shadow .18s ease;
}

.floatball__rim { position:absolute; inset:3px; border:1px solid color-mix(in srgb,var(--accent) 12%,transparent); border-radius:29%; pointer-events:none; }
.floatball__logo { width:62%; height:62%; object-fit:contain; pointer-events:none; }
.floatball__panel { position:absolute; right:-2px; bottom:-2px; width:36%; height:36%; display:grid; grid-template-columns:repeat(2,2px); grid-template-rows:repeat(2,2px); place-content:center; gap:1px; border:1.5px solid var(--panel-bg-strong); border-radius:35%; background:var(--accent); pointer-events:none; }
.floatball__panel i { border-radius:1px; background:var(--panel-bg); }
.floatball__ball:hover,.floatball__ball:focus-visible { transform:translateY(-1px); border-color:var(--accent); box-shadow:0 5px 10px #00000030,0 0 0 2px var(--accent-soft); }
.floatball--dragging .floatball__ball { cursor:grabbing; transform:scale(.95); border-color:var(--accent); }
.floatball--error .floatball__panel { background:var(--danger); }
.floatball__status { position:absolute; width:1px; height:1px; overflow:hidden; clip-path:inset(50%); }
@media(prefers-reduced-motion:reduce) { .floatball__ball { transition:none; } }
</style>
