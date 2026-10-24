// Loads web/*.js into a fresh Node `vm` context that stands in for a TV's browser: window, localStorage, navigator,
// document, and an XMLHttpRequest that never touches the network (tests answer each request by hand).
'use strict';
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ROOT = path.join(__dirname, '..', '..');

function catalogues() {
  const dir = path.join(ROOT, 'app', 'assets', 'i18n');
  const out = {};
  for (const f of fs.readdirSync(dir)) {
    if (f.endsWith('.json') && f !== 'en.json') out[f.slice(0, -5)] = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
  }
  return out;
}

/**
 * A browser-like context with the given scripts (names in web/) loaded in order.
 * opts.config: window.CONFIG; opts.zone: the TV's time zone; opts.language: navigator.language;
 * opts.storage: localStorage's initial items.
 */
function load(scripts, opts = {}) {
  const items = Object.assign({}, opts.storage || {});
  const requests = [];
  class FakeXHR {
    open(method, url) { this.method = method; this.url = url; this.headers = {}; }
    setRequestHeader(k, v) { this.headers[k] = v; }
    send() { requests.push(this); }
    respond(status, body) {
      this.readyState = 4;
      this.status = status;
      this.responseText = typeof body === 'string' ? body : JSON.stringify(body);
      this.onreadystatechange();
    }
    fail() { this.onerror(); }
  }
  const zone = opts.zone === undefined ? 'America/Sao_Paulo' : opts.zone;
  const RealIntl = Intl;
  const FakeIntl = Object.create(RealIntl);
  FakeIntl.DateTimeFormat = function (...a) {
    const f = new RealIntl.DateTimeFormat(...a);
    const resolved = f.resolvedOptions.bind(f);
    f.resolvedOptions = () => Object.assign(resolved(), {timeZone: zone});
    return f;
  };
  const ctx = {
    console, setTimeout, clearTimeout, setInterval, clearInterval, Date, Math, JSON, Intl: FakeIntl,
    navigator: {language: opts.language || 'en-US'},
    document: {documentElement: {lang: ''}},
    localStorage: {
      getItem: (k) => (Object.prototype.hasOwnProperty.call(items, k) ? items[k] : null),
      setItem: (k, v) => { items[k] = String(v); },
      removeItem: (k) => { delete items[k]; }
    },
    XMLHttpRequest: FakeXHR,
    CONFIG: opts.config,
    I18N_CATALOGUES: catalogues()
  };
  ctx.window = ctx;
  vm.createContext(ctx);
  for (const name of scripts) {
    const file = name === 'zones.js' && opts.zones ? null : path.join(ROOT, 'web', name);
    const code = file ? fs.readFileSync(file, 'utf8') : 'var ZONES = ' + JSON.stringify(opts.zones) + ';';
    vm.runInContext(code, ctx, {filename: name});
  }
  ctx.__items = items;
  ctx.__requests = requests;
  return ctx;
}

module.exports = {load, ROOT};
