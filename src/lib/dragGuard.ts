let armedUntil = 0;
const TTL_MS = 2000;

export function armDragGuard() {
  armedUntil = Date.now() + TTL_MS;
}

export function disarmDragGuard() {
  armedUntil = 0;
}

export function isDragGuardActive() {
  return Date.now() < armedUntil;
}

export function setupDragGuard(root: Document | Window = document): () => void {
  const onDown = (e: Event) => {
    const t = e.target as Element | null;
    if (t?.closest?.("[data-tauri-drag-region]")) armDragGuard();
  };
  const onUp = () => disarmDragGuard();
  root.addEventListener("pointerdown", onDown, true);
  root.addEventListener("pointerup", onUp, true);
  return () => {
    root.removeEventListener("pointerdown", onDown, true);
    root.removeEventListener("pointerup", onUp, true);
  };
}