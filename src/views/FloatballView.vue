<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";
import { tauriAvailable } from "@/lib/tauri";

const DRAG_THRESHOLD = 4;
const dragging = ref(false);
let startScreen = { x: 0, y: 0 };
let startPos = { x: 0, y: 0 };
let moved = false;

async function onPointerDown(event: PointerEvent) {
  if (!tauriAvailable()) return;
  dragging.value = true;
  moved = false;
  startScreen = { x: event.screenX, y: event.screenY };
  try {
    const win = getCurrentWebviewWindow();
    const pos = await win.outerPosition();
    startPos = { x: pos.x, y: pos.y };
  } catch {
    startPos = { x: 0, y: 0 };
  }
}

async function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const dx = event.screenX - startScreen.x;
  const dy = event.screenY - startScreen.y;
  if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) {
    moved = true;
  }
  if (!moved) return;
  try {
    await getCurrentWebviewWindow().setPosition(
      new PhysicalPosition(startPos.x + dx, startPos.y + dy),
    );
  } catch {
    /* ignore */
  }
}

async function endDrag(clientX: number, clientY: number) {
  if (!tauriAvailable()) return;
  dragging.value = false;
  if (moved) {
    try {
      const pos = await getCurrentWebviewWindow().outerPosition();
      await invoke("floatball_save_position", { x: pos.x, y: pos.y });
    } catch (e) {
      console.warn("[MyTools] floatball_save_position failed:", e);
    }
  } else {
    try {
      await invoke("floatball_toggle_main");
    } catch (e) {
      console.warn("[MyTools] floatball_toggle_main failed:", e);
    }
  }
  void clientX;
  void clientY;
}

function onPointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  void endDrag(event.clientX, event.clientY);
}

function onPointerLeave(event: PointerEvent) {
  if (!dragging.value) return;
  void endDrag(event.clientX, event.clientY);
}
</script>

<template>
  <div
    class="floatball"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointerleave="onPointerLeave"
  >
    <div class="floatball__ball" aria-label="悬浮球" title="MyTools">
      <span class="floatball__glyph">◆</span>
    </div>
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
}

.floatball:active {
  cursor: grabbing;
}

.floatball__ball {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent) 0%, var(--panel-bg) 100%);
  box-shadow:
    0 4px 14px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
  color: #fff;
  font-size: 18px;
  line-height: 1;
}

.floatball__glyph {
  pointer-events: none;
}
</style>
