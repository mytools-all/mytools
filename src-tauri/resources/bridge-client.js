(function () {
  var parentOrigin = null;
  try { parentOrigin = new URL(document.referrer).origin; } catch (_) {}
  var hostSeq = 0;
  var hostPending = {};
  var callbacks = { enter: [], ready: [], out: [], files: [], search: [] };
  var lastEvents = {};
  var lastEnv = { code: 'default', type: 'text', apiVersion: 1 };

  function fire(name, payload) {
    if ((name === 'ready' || name === 'enter') && payload && typeof payload === 'object') {
      lastEnv = payload;
    }
    var list = callbacks[name] || [];
    for (var i = 0; i < list.length; i++) {
      try { list[i](payload); } catch (_) {}
    }
  }

  function register(name, cb) {
    if (typeof cb !== 'function') { return; }
    callbacks[name].push(cb);
    if ((name === 'enter' || name === 'ready') && lastEvents[name] !== undefined) {
      var payload = lastEvents[name];
      setTimeout(function () { try { cb(payload); } catch (_) {} }, 0);
    }
  }

  window.addEventListener('message', function (e) {
    if (!parentOrigin || e.source !== window.parent || e.origin !== parentOrigin) { return; }
    var reply = e.data && e.data.$mytoolsReply;
    if (reply && reply.id) {
      var pendingCall = hostPending[reply.id];
      if (pendingCall) {
        delete hostPending[reply.id];
        if (reply.ok) { pendingCall.resolve(reply.value); }
        else { pendingCall.reject(new Error(reply.error || 'host call failed')); }
      }
      return;
    }
    var evt = e.data && e.data.$mytoolsEvent;
    if (!evt || typeof evt.event !== 'string') { return; }
    lastEvents[evt.event] = evt.payload;
    fire(evt.event, evt.payload);
  });

  window.mytools = {
    apiVersion: 1,
    call: function (capability, args) {
      return new Promise(function (resolve, reject) {
        var callId = ++hostSeq;
        hostPending[callId] = { resolve: resolve, reject: reject };
        window.parent.postMessage({ $mytoolsCall: { id: callId, capability: capability, args: args || null } }, '*');
      });
    },
    getEnv: function () { return lastEnv; },
    onPluginReady: function (cb) { register('ready', cb); },
    onPluginEnter: function (cb) { register('enter', cb); },
    onPluginOut: function (cb) { register('out', cb); }
    ,onFilesDropped: function (cb) { register('files', cb); }
    ,onSearch: function (cb) { register('search', cb); }
  };

  function __isNoise(payload) {
    var t = (payload && (payload.message || payload.reason)) || '';
    if (/ResizeObserver loop completed with undelivered notifications/i.test(t)) { return true; }
    if (/ResizeObserver loop limit exceeded/i.test(t)) { return true; }
    return false;
  }
  function __postMytools(payload) {
    if (__isNoise(payload)) { return; }
    try { window.parent.postMessage({ $mytools: payload }, '*'); } catch (_) {}
  }
  window.addEventListener('error', function (e) {
    __postMytools({
      kind: 'error',
      message: e && e.message ? String(e.message) : 'unknown error',
      filename: e && e.filename ? String(e.filename) : '',
      lineno: e && typeof e.lineno === 'number' ? e.lineno : 0,
      colno: e && typeof e.colno === 'number' ? e.colno : 0,
      stack: e && e.error && e.error.stack ? String(e.error.stack) : ''
    });
  });
  window.addEventListener('unhandledrejection', function (e) {
    var r = e && e.reason;
    __postMytools({
      kind: 'unhandledrejection',
      message: r && r.message ? String(r.message) : (r == null ? 'Promise rejected without a reason' : String(r)),
      reason: r && r.message ? String(r.message) : (r == null ? 'Promise rejected without a reason' : String(r)),
      stack: r && r.stack ? String(r.stack) : ''
    });
  });
})();
