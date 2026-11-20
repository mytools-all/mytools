import {createNotes} from './model.mjs';

const note = document.querySelector('#note');
const save = document.querySelector('#save');
const retry = document.querySelector('#retry');
const status = document.querySelector('#status');

function report(error) {
  status.textContent = `操作失败：${error?.message || String(error)}`;
}

const notes = createNotes((id, args) => {
  if (!window.mytools) throw new Error('请从 MyTools 打开此插件');
  return window.mytools.call(id, args);
}, (state) => {
  if (note.value !== state.text) note.value = state.text;
  note.disabled = state.phase !== 'ready';
  save.disabled = state.phase !== 'ready' || !state.dirty;
  retry.hidden = state.phase !== 'load-error';
  status.textContent = state.message;
});

note.addEventListener('input', () => {
  if (!notes.edit(note.value)) note.value = notes.snapshot().text;
});
save.addEventListener('click', () => { void notes.save().catch(report); });
retry.addEventListener('click', () => { void notes.load().catch(report); });

if (window.mytools) {
  window.mytools.onPluginReady(() => { void notes.load().catch(report); });
} else {
  status.textContent = '网页预览：请安装到 MyTools 后使用，浏览器预览不提供宿主存储。';
}
