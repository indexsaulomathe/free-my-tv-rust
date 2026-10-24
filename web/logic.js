/*
 * The pure logic of data.js and strings.js in Rust (rust/crates/freemytv-logic), compiled to WebAssembly and packed
 * into logic_wasm.js by `freemytv make-logic`. Where the TV runs WebAssembly (Chromium 57+: webOS 5+, Tizen 5+), each
 * call here answers once the module has loaded. Until then, on older TVs, and for input the Rust leaves alone (what
 * JavaScript would coerce or throw on), a call answers undefined, and data.js and strings.js do it in JavaScript.
 * Written for the Chromium of older TVs: ES5.
 */
var Logic = (function () {
  'use strict';

  var x = null, enc = null, dec = null;

  (function load(base64) {
    if (!base64 || typeof WebAssembly !== 'object' || typeof TextEncoder !== 'function') return;
    try {
      var raw = atob(base64), bytes = new Uint8Array(raw.length);
      for (var i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
      WebAssembly.instantiate(bytes, {}).then(function (r) {
        enc = new TextEncoder();
        dec = new TextDecoder();
        x = r.instance.exports;
      }, function () { /* stays in JavaScript */ });
    } catch (e) { /* stays in JavaScript */ }
  })(window.LOGIC_WASM);

  /** Calls an export with each string as (pointer, length) in the module's memory, then the numbers. */
  function run(fn, strings, numbers) {
    var args = [], held = [];
    try {
      for (var i = 0; i < strings.length; i++) {
        var b = enc.encode(strings[i]), p = x.alloc(b.length);
        new Uint8Array(x.memory.buffer, p, b.length).set(b);   // a new view: alloc may have grown the memory
        held.push(p, b.length);
        args.push(p, b.length);
      }
      return fn.apply(null, args.concat(numbers || []));
    } finally {
      for (var j = 0; j < held.length; j += 2) x.dealloc(held[j], held[j + 1]);
    }
  }

  function out() {
    return dec.decode(new Uint8Array(x.memory.buffer, x.out_ptr(), x.out_len()));
  }

  /** The answer as text, or undefined when the module isn't there or leaves it to the JavaScript. */
  function text(fn, strings, numbers) {
    return x && run(fn, strings, numbers) ? out() : undefined;
  }

  function json(fn, value, numbers) {
    if (!x || value === undefined) return undefined;
    var s = text(fn, [JSON.stringify(value)], numbers);
    return s === undefined ? undefined : JSON.parse(s);
  }

  function strings(list) {
    for (var i = 0; i < list.length; i++) if (typeof list[i] !== 'string') return false;
    return true;
  }

  return {
    /** Whether calls go to the WebAssembly (false while it loads, and on TVs without it). */
    ready: function () { return !!x; },
    weather: function (o, now) { return json(x && x.weather, o, [now]); },
    backdrop: function (images) { return json(x && x.backdrop, images); },
    localTime: function (iso, offsetSeconds) {
      if (!x || typeof iso !== 'string' || typeof offsetSeconds !== 'number') return undefined;
      return run(x.local_time, [iso], [offsetSeconds]);
    },
    /** The English key of a weather code's words ('' for none), for tr(). */
    condition: function (code, day) {
      return typeof code === 'number' && x ? text(x.condition, [], [code, day ? 1 : 0]) : undefined;
    },
    symbol: function (code, day) {
      return typeof code === 'number' && x ? text(x.symbol, [], [code, day ? 1 : 0]) : undefined;
    },
    franchise: function (title) { return typeof title === 'string' && x ? text(x.franchise, [title]) : undefined; },
    /** The index in data.js's SERVICES, -1 for none. */
    service: function (name, id) {
      if (!x || typeof name !== 'string' || typeof id !== 'number') return undefined;
      var i = run(x.service, [name], [id]);
      return i === -2 ? undefined : i;
    },
    pluralCategory: function (lang, n) {
      return typeof lang === 'string' && typeof n === 'number' && x ? text(x.plural_category, [lang], [n]) : undefined;
    },
    pluralForm: function (forms, category, n) {
      return strings([forms, category]) && x ? text(x.plural_form, [forms, category, String(n)]) : undefined;
    },
    base: function (code) { return typeof code === 'string' && x ? text(x.base, [code]) : undefined; },
    localeOf: function (code, tv) { return strings([code, tv]) && x ? text(x.locale_of, [code, tv]) : undefined; },
    resolve: function (setting, tv, languages) {
      return strings([setting, tv]) && x ? text(x.resolve, [setting, tv, languages.join('\n')]) : undefined;
    }
  };
})();
