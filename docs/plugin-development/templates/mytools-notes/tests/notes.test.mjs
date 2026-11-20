import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createNotes, parseSaved, KEY, MAX_LENGTH} from '../model.mjs';

test('缺键初始化，保存后新实例恢复原始文本', async () => {
  let stored = null;
  const call = async (id, args) => {
    assert.equal(args.key, KEY);
    assert.equal('namespace' in args, false);
    if (id === 'storage.get') return structuredClone(stored);
    assert.equal(id, 'storage.set');
    stored = structuredClone(args.value);
    return null;
  };
  const notes = createNotes(call);
  assert.equal(notes.edit('过早编辑'), false);
  assert.equal(await notes.save(), false);
  await notes.load();
  assert.equal(notes.snapshot().text, '');
  const text = '中文\n<script>alert(1)</script> 😀';
  notes.edit(text);
  assert.equal(await notes.save(), true);
  assert.equal(notes.snapshot().dirty, false);
  const reopened = createNotes(call);
  await reopened.load();
  assert.equal(reopened.snapshot().text, text);
});

test('读取被拒绝时禁止编辑与写入，重试后恢复', async () => {
  let denied = true;
  let writes = 0;
  const notes = createNotes(async (id) => {
    if (id === 'storage.set') { writes++; return null; }
    if (denied) throw new Error('权限被拒绝');
    return {version: 1, text: '原数据'};
  });
  assert.equal(await notes.load(), false);
  assert.equal(notes.snapshot().phase, 'load-error');
  assert.match(notes.snapshot().message, /权限被拒绝/);
  assert.equal(notes.edit('覆盖'), false);
  assert.equal(await notes.save(), false);
  assert.equal(writes, 0);
  denied = false;
  await notes.load();
  assert.equal(notes.snapshot().text, '原数据');
});

test('损坏、未知版本或超长数据不会被空数据覆盖', async () => {
  for (const value of [undefined, [], 'bad', {version: 2, text: '未来数据'}, {version: 1, text: 3}, {version: 1, text: 'x'.repeat(MAX_LENGTH + 1)}]) {
    let writes = 0;
    const notes = createNotes(async (id) => {
      if (id === 'storage.set') writes++;
      return value;
    });
    await notes.load();
    assert.equal(notes.snapshot().phase, 'load-error');
    assert.equal(await notes.save(), false);
    assert.equal(writes, 0);
  }
});

test('保存失败保留输入与已保存值，重试成功才提交', async () => {
  let fail = true;
  const notes = createNotes(async (id) => {
    if (id === 'storage.get') return {version: 1, text: '旧值'};
    if (fail) throw new Error('磁盘写入失败');
    return null;
  });
  await notes.load();
  notes.edit('新值');
  assert.equal(await notes.save(), false);
  assert.equal(notes.snapshot().text, '新值');
  assert.equal(notes.snapshot().savedText, '旧值');
  assert.equal(notes.snapshot().dirty, true);
  assert.match(notes.snapshot().message, /磁盘写入失败/);
  fail = false;
  await notes.save();
  assert.equal(notes.snapshot().savedText, '新值');
});

test('慢加载、重复 ready、慢保存期间不能重复调用或覆盖编辑', async () => {
  let resolveRead;
  let resolveWrite;
  let reads = 0;
  let writes = 0;
  const notes = createNotes((id) => {
    if (id === 'storage.get') { reads++; return new Promise((r) => { resolveRead = r; }); }
    writes++;
    return new Promise((r) => { resolveWrite = r; });
  });
  const loading = notes.load();
  assert.equal(await notes.load(), false);
  assert.equal(notes.edit('过早'), false);
  resolveRead(null);
  await loading;
  notes.edit('保留的编辑');
  assert.equal(await notes.load(), false);
  assert.equal(notes.snapshot().text, '保留的编辑');
  const saving = notes.save();
  assert.equal(notes.edit('保存中修改'), false);
  assert.equal(await notes.save(), false);
  resolveWrite(null);
  await saving;
  assert.equal(reads, 1);
  assert.equal(writes, 1);
});

test('长度边界和空文本有效，不接受超长修改', async () => {
  assert.equal(parseSaved({version: 1, text: ''}), '');
  const notes = createNotes(async () => null);
  await notes.load();
  assert.equal(notes.edit('x'.repeat(MAX_LENGTH)), true);
  assert.equal(notes.edit('x'.repeat(MAX_LENGTH + 1)), false);
  assert.equal(notes.snapshot().text.length, MAX_LENGTH);
  notes.edit('');
  assert.equal(notes.snapshot().dirty, false);
});

test('清单版本、能力与全部交付资源一致', async () => {
  const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
  const manifest = JSON.parse(await read('plugin.json'));
  const pkg = JSON.parse(await read('package.json'));
  assert.equal(manifest.version, pkg.version);
  assert.equal(manifest.name, pkg.name);
  assert.deepEqual(manifest.capabilities, ['storage.get', 'storage.set']);
  assert.equal(pkg.private, true);
  for (const path of pkg.files) assert.ok((await read(path)).length > 0, path);
  const html = await read('index.html');
  assert.match(html, /type="module" src="\.\/app\.mjs"/);
  assert.match(html, /maxlength="20000"/);
});
