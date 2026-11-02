<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";

const directions = [
  { dir: "n", cursor: "ns-resize" },
  { dir: "s", cursor: "ns-resize" },
  { dir: "e", cursor: "ew-resize" },
  { dir: "w", cursor: "ew-resize" },
  { dir: "ne", cursor: "nesw-resize" },
  { dir: "nw", cursor: "nwse-resize" },
  { dir: "se", cursor: "nwse-resize" },
  { dir: "sw", cursor: "nesw-resize" },
] as const;

function onPointerDown(e: PointerEvent, dir: string) {
  if (!("__TAURI_INTERNALS__" in window)) return;
  e.preventDefault();
  e.stopPropagation();
  getCurrentWindow()
    .startResizeDragging(dir as never)
    .catch(() => {});
}
</script>

<template>
  <div class="resize-handles">
    <div
      v-for="h in directions"
      :key="h.dir"
      class="resize-handle"
      :class="`resize-handle--${h.dir}`"
      :style="{ cursor: h.cursor }"
      @pointerdown.self="onPointerDown($event, h.dir)"
    />
  </div>
</template>

<style scoped>
.resize-handles {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 9999;
}
.resize-handle {
  position: absolute;
  pointer-events: auto;
}
.resize-handle--n {
  top: 0;
  left: 8px;
  right: 8px;
  height: 6px;
}
.resize-handle--s {
  bottom: 0;
  left: 8px;
  right: 8px;
  height: 6px;
}
.resize-handle--e {
  top: 8px;
  right: 0;
  bottom: 8px;
  width: 6px;
}
.resize-handle--w {
  top: 8px;
  left: 0;
  bottom: 8px;
  width: 6px;
}
.resize-handle--ne {
  top: 0;
  right: 0;
  width: 10px;
  height: 10px;
}
.resize-handle--nw {
  top: 0;
  left: 0;
  width: 10px;
  height: 10px;
}
.resize-handle--se {
  bottom: 0;
  right: 0;
  width: 10px;
  height: 10px;
}
.resize-handle--sw {
  bottom: 0;
  left: 0;
  width: 10px;
  height: 10px;
}
</style>