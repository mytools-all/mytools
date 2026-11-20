export const KEY = 'notes.v1';
export const MAX_LENGTH = 20000;

export function parseSaved(value) {
  if (value === null) return '';
  if (typeof value !== 'object' || Array.isArray(value) || value.version !== 1 ||
      typeof value.text !== 'string' || value.text.length > MAX_LENGTH) {
    throw new Error('便签数据损坏或版本不支持，已停止写入；请保留原数据');
  }
  return value.text;
}

export function createNotes(call, notify = () => {}) {
  let state = {phase: 'idle', text: '', savedText: '', message: '等待 MyTools 就绪…'};
  const snapshot = () => ({...state, dirty: state.text !== state.savedText});
  const update = (patch) => {
    state = {...state, ...patch};
    notify(snapshot());
  };
  const message = (error) => error?.message || String(error);

  return {
    snapshot,
    async load() {
      if (!['idle', 'load-error'].includes(state.phase)) return false;
      update({phase: 'loading', message: '正在加载…'});
      try {
        const text = parseSaved(await call('storage.get', {key: KEY}));
        update({phase: 'ready', text, savedText: text, message: '已加载，可以编辑'});
        return true;
      } catch (error) {
        update({phase: 'load-error', message: `加载失败：${message(error)}。可重试加载。`});
        return false;
      }
    },
    edit(text) {
      if (state.phase !== 'ready') return false;
      if (typeof text !== 'string' || text.length > MAX_LENGTH) {
        update({message: '内容超过长度限制，未接受本次修改'});
        return false;
      }
      update({text, message: text === state.savedText ? '内容与已保存版本一致' : '有未保存的修改'});
      return true;
    },
    async save() {
      if (state.phase !== 'ready' || state.text === state.savedText) return false;
      const text = state.text;
      update({phase: 'saving', message: '正在保存…'});
      try {
        await call('storage.set', {key: KEY, value: {version: 1, text}});
        update({phase: 'ready', savedText: text, message: '已保存'});
        return true;
      } catch (error) {
        update({phase: 'ready', message: `保存失败：${message(error)}。输入仍保留，请点击保存重试。`});
        return false;
      }
    }
  };
}
