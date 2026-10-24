// web/data.js (Place, Prefs, Data) and web/tv.js in a desktop browser's stand-in.
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {load} = require('./harness');

const ALL = ['strings.js', 'tv.js', 'zones.js', 'data.js'];
const GENERIC = {tmdbKey: '', tmdbRegion: '', latitude: '', longitude: '', city: '', language: 'auto', currency: '',
  temperature: 'auto', showCrypto: true, hideAnimation: false, hideApps: []};

function app(opts = {}) {
  const ctx = load(ALL, Object.assign({config: GENERIC}, opts));
  ctx.I18N.set('en');
  return ctx;
}

// MARK: Place

test('generic build: São Paulo from the time zone, Brazilian reais', () => {
  const {Place} = app();
  assert.equal(Place.city, 'São Paulo');
  assert.equal(Place.country, 'BR');
  assert.equal(Place.currency, 'BRL');
  assert.equal(Place.lat, '-23.53');
  assert.equal(Place.fahrenheit, false);
});

test('an alias zone follows its target', () => {
  assert.equal(app({zone: 'Brazil/East'}).Place.city, 'São Paulo');
  assert.equal(app({zone: 'Europe/Lisbon'}).Place.currency, 'EUR');
});

test('US zone means Fahrenheit and dollars', () => {
  const {Place} = app({zone: 'America/Chicago'});
  assert.equal(Place.fahrenheit, true);
  assert.equal(Place.currency, 'USD');
});

test('unknown zone falls back to New York, language decides Fahrenheit', () => {
  const {Place} = app({zone: 'Mars/Base', language: 'en-US'});
  assert.equal(Place.lat, '40.7128');
  assert.equal(Place.country, 'US');
  assert.equal(Place.fahrenheit, true);
});

test('config.properties wins over the zone', () => {
  const {Place} = app({config: Object.assign({}, GENERIC, {latitude: '38.72', longitude: '-9.13', city: 'Lisboa',
    tmdbRegion: 'pt', currency: 'gbp'})});
  assert.equal(Place.lat, '38.72');
  assert.equal(Place.city, 'Lisboa');
  assert.equal(Place.country, 'PT');
  assert.equal(Place.currency, 'GBP');
});

// MARK: Prefs

test('Prefs: TV settings win and are stored', () => {
  const ctx = app({config: Object.assign({}, GENERIC, {temperature: 'C', showCrypto: false})});
  const {Prefs} = ctx;
  assert.equal(Prefs.temperature(), 'C');
  assert.equal(Prefs.quotes(), false);
  Prefs.set('temperature', 'F');
  assert.equal(Prefs.temperature(), 'F');
  assert.equal(Prefs.version, 1);
  Prefs.setCoin(2, false);
  assert.equal(Prefs.coin(2), false);
  assert.equal(Prefs.coin(0), true);
  assert.deepEqual(JSON.parse(ctx.__items['fmtv.prefs']), {temperature: 'F', coins: [true, true, false, true]});
});

test('Prefs: broken storage is ignored', () => {
  const {Prefs} = app({storage: {'fmtv.prefs': '{not json'}});
  assert.equal(Prefs.theme(), 'auto');
});

// MARK: Data formatting

test('compact, money, change', () => {
  const {Data} = app();
  assert.equal(Data.compact(NaN), '—');
  assert.equal(Data.compact(1234567), '1.23M');
  assert.equal(Data.compact(12345), '12.3k');
  assert.equal(Data.compact(1234), '1.23k');
  assert.equal(Data.compact(123.4), '123');
  assert.equal(Data.compact(1.5), '1.50');
  assert.equal(Data.money(NaN), 'R$ —');
  assert.equal(Data.money(5.4321), 'R$ 5.43');
  assert.equal(Data.money(350000), 'R$ 350,000');
  assert.equal(Data.change(2.345), '+2.3%');
  assert.equal(Data.change(-1), '−1.0%');
  assert.equal(Data.change(NaN), '');
});

test('degrees follow the unit', () => {
  const ctx = app();
  assert.equal(ctx.Data.degrees(21.6), '22°');
  assert.equal(ctx.Data.degrees(null), '—');
  ctx.Prefs.set('temperature', 'F');
  assert.equal(ctx.Data.degrees(0), '32°');
});

test('weather codes to text and symbol', () => {
  const {Data} = app();
  assert.equal(Data.condition(0, true), 'Clear');
  assert.equal(Data.condition(1, false), 'Mostly Clear');
  assert.equal(Data.condition(48, true), 'Fog');
  assert.equal(Data.condition(61, true), 'Rain');
  assert.equal(Data.condition(96, true), 'Thunderstorms');
  assert.equal(Data.condition(-1, true), '');
  assert.equal(Data.symbol(0, false), 'clear_night');
  assert.equal(Data.symbol(2, true), 'partly_cloudy_day');
  assert.equal(Data.symbol(73, true), 'snowflake');
  assert.equal(Data.symbol(63, true), 'rainy');
});

test('night without a forecast is 18:00 to 6:00, theme overrides', () => {
  const ctx = app();
  assert.equal(ctx.Data.isNight(new Date(2026, 0, 1, 12).getTime()), false);
  assert.equal(ctx.Data.isNight(new Date(2026, 0, 1, 19).getTime()), true);
  assert.equal(ctx.Data.isNight(new Date(2026, 0, 1, 5, 59).getTime()), true);
  ctx.Prefs.set('theme', 'dark');
  assert.equal(ctx.Data.isNight(new Date(2026, 0, 1, 12).getTime()), true);
});

test('staleness only after twice the interval', () => {
  const {Data} = app();
  assert.equal(Data.staleness(0, 60000), '');
  assert.equal(Data.staleness(Date.now() - 60000, 60000), '');
  assert.equal(Data.staleness(Date.now() - 5 * 60000, 60000), '5 min ago');
  assert.equal(Data.staleness(Date.now() - 3 * 3600000, 60000), '3 h ago');
});

test('image urls and the key', () => {
  const {Data} = app();
  assert.equal(Data.image('/a.jpg'), 'https://image.tmdb.org/t/p/w1280/a.jpg');
  assert.equal(Data.image('/a.jpg', 'w300'), 'https://image.tmdb.org/t/p/w300/a.jpg');
  assert.equal(Data.image(''), '');
  assert.equal(Data.hasKey(), false);
  assert.equal(app({config: Object.assign({}, GENERIC, {tmdbKey: ' k '})}).Data.hasKey(), true);
});

// MARK: Data over the (fake) network

test('refresh asks Open-Meteo and Binance over HTTPS only, not TMDB without a key', () => {
  const ctx = app();
  ctx.Data.refresh();
  const urls = ctx.__requests.map((r) => r.url);
  assert.ok(urls.some((u) => u.startsWith('https://api.open-meteo.com/v1/forecast?')), urls.join('\n'));
  assert.ok(urls.some((u) => /^https:\/\/api\.binance\.(com|us)\//.test(u)), urls.join('\n'));
  assert.ok(urls.every((u) => u.startsWith('https://')));
  assert.ok(!urls.some((u) => u.includes('themoviedb')));
  const meteo = urls.find((u) => u.includes('open-meteo'));
  assert.ok(meteo.includes('-23.53') && meteo.includes('-46.62'));
});

test('a failed request does not break the app', () => {
  const ctx = app();
  ctx.Data.refresh();
  for (const r of ctx.__requests.slice()) r.fail();
  assert.equal(ctx.Data.weather, null);
});

// MARK: TV stand-in

test('TV stand-in: sample apps, inputs and keys', async () => {
  const {TV} = app();
  const apps = await new Promise((resolve) => TV.apps(resolve));
  assert.ok(Array.isArray(apps) && apps.length > 0);
  const inputs = await new Promise((resolve) => TV.inputs(resolve));
  assert.ok(inputs.length >= 5);
  assert.equal(inputs[2].connected, false);
  assert.equal(TV.keys.back, 461);
  assert.equal(TV.zone(), 'America/Sao_Paulo');
  assert.equal(await new Promise((resolve) => TV.launch('netflix', null, resolve)), true);
});
