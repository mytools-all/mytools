import { ref } from "vue";

export interface CapabilityConsent {
  capability: string;
  plugin: string;
  message: string;
}

interface PendingConsent {
  request: CapabilityConsent;
  finish: (granted: boolean) => void;
}

const consent = ref<CapabilityConsent | null>(null);
const queue: PendingConsent[] = [];

function showNext() {
  consent.value = queue[0]?.request ?? null;
}

export function useCapabilityConsent() {
  return { consent };
}

export function resolveCapabilityConsent(granted: boolean) {
  queue[0]?.finish(granted);
}

export function askCapabilityConsent(request: CapabilityConsent, signal?: AbortSignal): Promise<boolean> {
  if (signal?.aborted) return Promise.resolve(false);
  return new Promise((resolve) => {
    const abort = () => pending.finish(false);
    const pending: PendingConsent = {
      request,
      finish(granted) {
        const index = queue.indexOf(pending);
        if (index < 0) return;
        queue.splice(index, 1);
        signal?.removeEventListener("abort", abort);
        showNext();
        resolve(granted);
      },
    };
    queue.push(pending);
    signal?.addEventListener("abort", abort, { once: true });
    showNext();
  });
}
