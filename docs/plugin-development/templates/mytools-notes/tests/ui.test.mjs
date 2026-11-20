import test from 'node:test';
import assert from 'node:assert/strict';

test('实际 UI 连接就绪、加载重试、保存失败与文本安全显示', async () => {
  const elements = new Map(['note', 'save', 'retry', 'status'].map((id) => [id, {
    value: '', disabled: id !== 'status', hidden: id === 'retry', textContent: '', handlers: {},
    addEventListener(event, handler) { this.handlers[event] = handler; }
  }]));
  const element = (id) => elements.get(id);
  let ready;
  let denyRead = true;
  let denyWrite = true;
  let stored = null;
  let reads = 0;
  const originalWindow = globalThis.window;
  const originalDocument = globalThis.document;
  globalThis.document = {querySelector: (selector) => element(selector.slice(1))};
  globalThis.window = {mytools: {
    onPluginReady(callback) { ready = callback; },
    async call(id, args) {
      if (id === 'storage.get') {
        reads++;
        if (denyRead) throw new Error('读取被拒绝');
        return stored;
      }
      if (denyWrite) throw new Error('保存被拒绝');
      stored = structuredClone(args.value);
      return null;
    }
  }};
  const settle = () => new Promise((resolve) => setImmediate(resolve));
  try {
    await import('../app.mjs');
    ready();
    await settle();
    assert.equal(element('note').disabled, true);
    assert.equal(element('retry').hidden, false);
    denyRead = false;
    element('retry').handlers.click();
    await settle();
    assert.equal(element('note').disabled, false);
    assert.equal(element('retry').hidden, true);
    const text = '<script>alert(1)</script>\n原样保存';
    element('note').value = text;
    element('note').handlers.input();
    assert.equal(element('save').disabled, false);
    ready();
    await settle();
    assert.equal(reads, 2);
    assert.equal(element('note').value, text);
    element('save').handlers.click();
    assert.equal(element('note').disabled, true);
    await settle();
    assert.match(element('status').textContent, /保存被拒绝/);
    assert.equal(element('note').value, text);
    assert.equal(element('save').disabled, false);
    denyWrite = false;
    element('save').handlers.click();
    await settle();
    assert.equal(stored.text, text);
    assert.equal(element('status').textContent, '已保存');
    assert.equal(element('save').disabled, true);
  } finally {
    if (originalWindow === undefined) delete globalThis.window;
    else globalThis.window = originalWindow;
    if (originalDocument === undefined) delete globalThis.document;
    else globalThis.document = originalDocument;
  }
});
