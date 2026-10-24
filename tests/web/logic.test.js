// web/logic.js: the Rust (WebAssembly) answers exactly what the JavaScript does, over many inputs, odd ones included.
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {load, internal} = require('./harness');

const ALL = ['strings.js', 'tv.js', 'zones.js', 'data.js'];
const GENERIC = {tmdbKey: '', tmdbRegion: '', latitude: '', longitude: '', city: '', language: 'auto', currency: '',
  temperature: 'auto', showCrypto: true, hideAnimation: false, hideApps: []};

function pair(opts = {}) {
  const js = load(ALL, Object.assign({config: GENERIC, wasm: false}, opts));
  const rs = load(ALL, Object.assign({config: GENERIC, wasm: true}, opts));
  return {js, rs};
}

/** Same value, or both threw. */
function same(f) {
  const run = (ctx) => { try { return {v: JSON.stringify(f(ctx))}; } catch (e) { return {threw: true}; } };
  return [run, (a, b) => assert.deepEqual(a, b)];
}

function check(ctxs, f, label) {
  const [run] = same(f);
  assert.deepEqual(run(ctxs.rs), run(ctxs.js), label);
}

test('the Rust is in use where WebAssembly is', () => {
  const {js, rs} = pair();
  assert.equal(rs.Logic.ready(), true);
  assert.equal(js.Logic.ready(), false);
});

test('weather words and symbols', () => {
  const p = pair();
  p.js.I18N.set('pt');
  p.rs.I18N.set('pt');
  const codes = [-5, -1, 0, 1, 2, 3, 4, 45, 48, 50, 51, 56.5, 57, 58, 61, 65, 67, 71, 77, 80, 82, 85, 86, 94, 95, 99, 1e9,
    NaN, Infinity, -Infinity, undefined, null, '3', true];
  for (const c of codes) {
    for (const day of [true, false, 0, 1, undefined]) {
      check(p, (ctx) => ctx.Data.condition(c, day), `condition(${c}, ${day})`);
      check(p, (ctx) => ctx.Data.symbol(c, day), `symbol(${c}, ${day})`);
    }
  }
});

test('plurals, with and without Intl.PluralRules', () => {
  const key = 'plural|one:{n} season|other:{n} seasons';
  for (const intl of [true, false]) {
    for (const lang of ['en', 'pt', 'ru', 'uk', 'pl', 'cs', 'ro', 'ja', 'de']) {
      const p = pair();
      for (const ctx of [p.js, p.rs]) {
        if (!intl) ctx.Intl.PluralRules = undefined;
        ctx.I18N.set(lang);
      }
      for (const n of [0, 1, 2, 3, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111, 1.5, -1, -21, 1e21]) {
        check(p, (ctx) => ctx.I18N.plural(key, n), `${lang} ${intl} plural(${n})`);
        check(p, (ctx) => ctx.I18N.plural('plural|one:{n}|other:{n}{n}', n), `${lang} two {n}`);
      }
    }
  }
});

test('the language and the locale picked', () => {
  for (const tv of ['en-US', 'pt-BR', 'pt-PT', 'es-MX', 'es-ES', 'de-AT', 'zh-TW', 'zh-Hant-HK', 'zh-CN', 'zh',
    'no-NO', 'nn', 'in-ID', 'iw-IL', 'fr_CA', 'xx', '']) {
    const p = pair({language: tv});
    for (const setting of ['auto', '', undefined, 'pt', 'pt-BR', 'pt_BR', 'PT', 'zh-cn', 'zh', 'es', 'fr', 'he', 'nb']) {
      check(p, (ctx) => { ctx.I18N.set(setting); return [ctx.I18N.lang, ctx.I18N.base, ctx.I18N.locale()]; },
        `tv ${tv} setting ${setting}`);
    }
  }
});

test('franchises, services and backdrops', () => {
  const {js, rs} = pair();
  const a = internal(js, ['franchise', 'service', 'bestBackdrop']);
  const b = internal(rs, ['franchise', 'service', 'bestBackdrop']);
  for (const t of ['Star Wars: Andor', ' Dune - Part Two ', ':x', ' - y', 'ÉLITE: Temporada', 'ΣΟΦΙΑ', 'İstanbul', '',
    ' Spaced ', undefined, null]) {
    assert.deepEqual(b.franchise(t), a.franchise(t), `franchise(${t})`);
  }
  for (const n of ['Netflix', 'Netflix basic with Ads', 'Amazon Prime Video', 'Max', 'HBO Max', 'Maxdome', 'max_go',
    'Apple TV', 'Apple TV+', 'Paramount Plus Apple TV Channel', 'Disney Plus', 'JioHotstar', '', 'CRUNCHYROLL']) {
    for (const id of [2, 350, 8, undefined]) {
      const key = (s) => (s ? s[1] : s);   // the arrays come from two contexts: compare the key
      assert.equal(key(b.service(n, id)), key(a.service(n, id)), `service(${n}, ${id})`);
    }
  }
  const images = [undefined, null, {}, {backdrops: []}, {backdrops: null},
    {backdrops: [{file_path: '/a.jpg', iso_639_1: 'en', width: 3840, vote_average: 9},
      {file_path: '/b.jpg', iso_639_1: null, width: 1280, vote_average: 5}, {file_path: '', iso_639_1: null}]},
    {backdrops: [{file_path: '/c.jpg', width: 5000}, {file_path: '/d.jpg', width: 3840, vote_average: 0.5}]},
    {backdrops: [{file_path: '/e.jpg', iso_639_1: null, width: '1920'}]}];
  for (const im of images) {
    assert.deepEqual(b.bestBackdrop(im), a.bestBackdrop(im), `bestBackdrop(${JSON.stringify(im)})`);
  }
});

test('weather answers, odd ones too', () => {
  const now = Date.now();
  const iso = (ms) => new Date(ms).toISOString().slice(0, 16);
  const hours = [-2, -1, 1, 2, 3, 4, 5, 6, 7, 8].map((h) => iso(now + h * 3600000));
  const good = {
    utc_offset_seconds: 0,
    current: {temperature_2m: 21.4, weather_code: 3, is_day: 1},
    daily: {temperature_2m_max: [25.1], temperature_2m_min: [14.2], sunrise: ['2026-10-20T05:30'], sunset: ['2026-10-20T18:01']},
    hourly: {time: hours, temperature_2m: hours.map((_, i) => 10 + i), weather_code: hours.map((_, i) => i * 10),
      is_day: hours.map((_, i) => i % 2)}
  };
  const variants = [good, {}, null, [], 'x', 3, {current: null}, {current: {}}, {current: 0},
    Object.assign({}, good, {utc_offset_seconds: -10800}), Object.assign({}, good, {utc_offset_seconds: '3600'}),
    Object.assign({}, good, {utc_offset_seconds: null}), Object.assign({}, good, {hourly: null}),
    Object.assign({}, good, {hourly: {time: hours}}), Object.assign({}, good, {daily: {}}),
    Object.assign({}, good, {daily: Object.assign({}, good.daily, {sunrise: [], temperature_2m_max: []})}),
    Object.assign({}, good, {current: {is_day: 0}}), Object.assign({}, good, {current: {is_day: '0'}}),
    Object.assign({}, good, {hourly: Object.assign({}, good.hourly, {is_day: 1})}),
    Object.assign({}, good, {hourly: Object.assign({}, good.hourly, {is_day: undefined, temperature_2m: [1]})}),
    Object.assign({}, good, {daily: Object.assign({}, good.daily, {sunset: ['99-01-01T00:00'], sunrise: ['2026-13-40T25:61']})})];
  for (const v of variants) {
    const p = pair();
    const run = (ctx) => {
      ctx.Data.refresh();
      const r = ctx.__requests.find((x) => x.url.indexOf('open-meteo') >= 0);
      try { r.respond(200, v); } catch (e) { return 'threw'; }
      return JSON.parse(JSON.stringify(ctx.Data.weather));
    };
    assert.deepEqual(run(p.rs), run(p.js), `weather ${JSON.stringify(v)}`);
  }
});
