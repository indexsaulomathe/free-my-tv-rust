// JavaScript vs the Rust (WebAssembly) behind web/logic.js, per call, in Node's V8 (the TVs' Chromium engine).
//   node tests/bench/logic_bench.js
'use strict';
const {load, internal} = require('../web/harness');

const GENERIC = {tmdbKey: '', tmdbRegion: '', latitude: '', longitude: '', city: '', language: 'auto', currency: '',
  temperature: 'auto', showCrypto: true, hideAnimation: false, hideApps: []};
const ALL = ['strings.js', 'tv.js', 'zones.js', 'data.js'];
const now = Date.now();
const iso = (ms) => new Date(ms).toISOString().slice(0, 16);
const hours = Array.from({length: 48}, (_, h) => iso(now + (h - 12) * 3600000));
const forecast = {
  utc_offset_seconds: -10800, current: {temperature_2m: 21.4, weather_code: 3, is_day: 1},
  daily: {temperature_2m_max: [25.1, 26], temperature_2m_min: [14.2, 15], sunrise: ['2026-10-20T05:30', '2026-10-21T05:29'],
    sunset: ['2026-10-20T18:01', '2026-10-21T18:02']},
  hourly: {time: hours, temperature_2m: hours.map((_, i) => 10 + i / 3), weather_code: hours.map((_, i) => i % 4),
    is_day: hours.map((_, i) => i % 2)}
};

function time(fn, runs) {
  for (let i = 0; i < 2000; i++) fn(i);   // warm up the JIT
  const t = process.hrtime.bigint();
  for (let i = 0; i < runs; i++) fn(i);
  return Number(process.hrtime.bigint() - t) / runs;   // ns per call
}

const rows = [];
for (const [name, runs, fn] of [
  ['Data.condition (weather words)', 200000, (ctx) => (i) => ctx.Data.condition(i % 100, i & 1)],
  ['Data.symbol (weather icon)', 200000, (ctx) => (i) => ctx.Data.symbol(i % 100, i & 1)],
  ['I18N.plural (ru, no Intl)', 100000, (ctx) => (i) => ctx.I18N.plural('plural|one:{n} season|other:{n} seasons', i)],
  ['parseWeather (48 h forecast)', 5000, (ctx) => { const f = internal(ctx, ['localTime', 'parseWeather']); return () => f.parseWeather(forecast); }],
  ['franchise', 200000, (ctx) => { const f = internal(ctx, ['franchise']); return (i) => f.franchise(i & 1 ? 'Star Wars: Andor' : 'Dune - Part Two'); }]
]) {
  const out = [];
  for (const wasm of [false, true]) {
    const ctx = load(ALL, {config: GENERIC, wasm});
    ctx.Intl.PluralRules = undefined;
    ctx.I18N.set('ru');
    const f = fn(ctx);
    out.push(time(f, runs));
  }
  rows.push([name, out[0], out[1]]);
}
console.log('| Call | JavaScript | Rust (WASM) | Rust / JS |');
console.log('|---|---:|---:|---:|');
for (const [name, js, rs] of rows) {
  const f = (ns) => (ns >= 1000 ? (ns / 1000).toFixed(1) + ' µs' : ns.toFixed(0) + ' ns');
  console.log(`| ${name} | ${f(js)} | ${f(rs)} | ${(rs / js).toFixed(1)}× |`);
}
