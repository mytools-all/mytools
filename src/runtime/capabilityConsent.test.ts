import { afterEach, describe, expect, it } from "vitest";
import { askCapabilityConsent, resolveCapabilityConsent, useCapabilityConsent } from "@/runtime/capabilityConsent";

const request = (plugin: string) => ({ plugin, capability: "fs.write", message: "Allow?" });
afterEach(() => {
  while (useCapabilityConsent().consent.value) resolveCapabilityConsent(false);
});

describe("capability consent queue", () => {
  it("resolves concurrent requests in order without losing a caller", async () => {
    const first = askCapabilityConsent(request("first"));
    const second = askCapabilityConsent(request("second"));
    expect(useCapabilityConsent().consent.value?.plugin).toBe("first");
    resolveCapabilityConsent(true);
    await expect(first).resolves.toBe(true);
    expect(useCapabilityConsent().consent.value?.plugin).toBe("second");
    resolveCapabilityConsent(false);
    await expect(second).resolves.toBe(false);
    expect(useCapabilityConsent().consent.value).toBeNull();
  });

  it("cancels the active request when its plugin is destroyed", async () => {
    const controller = new AbortController();
    const first = askCapabilityConsent(request("first"), controller.signal);
    const second = askCapabilityConsent(request("second"));
    controller.abort();
    await expect(first).resolves.toBe(false);
    expect(useCapabilityConsent().consent.value?.plugin).toBe("second");
    resolveCapabilityConsent(true);
    await expect(second).resolves.toBe(true);
  });

  it("cancels a queued request without dismissing another plugin's prompt", async () => {
    const controller = new AbortController();
    const first = askCapabilityConsent(request("first"));
    const second = askCapabilityConsent(request("second"), controller.signal);
    controller.abort();
    await expect(second).resolves.toBe(false);
    expect(useCapabilityConsent().consent.value?.plugin).toBe("first");
    resolveCapabilityConsent(true);
    await expect(first).resolves.toBe(true);
    expect(useCapabilityConsent().consent.value).toBeNull();
  });
});
