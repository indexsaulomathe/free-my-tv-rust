// web/strings.js: tr, trf, plural, formatDate and the language picked.
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {load} = require('./harness');

function i18n(lang) {
  const ctx = load(['strings.js']);
  ctx.I18N.set(lang);
  return ctx;
}

test('English returns the key itself, without its context', () => {
  const ctx = i18n('en');
  assert.equal(ctx.tr('Live TV'), 'Live TV');
  assert.equal(ctx.tr('night|Clear'), 'Clear');
  assert.equal(ctx.I18N.locale(), 'en-US');
});

test('Portuguese translates and keeps unknown keys in English', () => {
  const ctx = i18n('pt');
  assert.equal(ctx.I18N.lang, 'pt');
  assert.equal(ctx.I18N.locale(), 'pt-BR');
  assert.notEqual(ctx.tr('Live TV'), 'Live TV');
  assert.equal(ctx.tr('a string nobody translated'), 'a string nobody translated');
});

test('trf fills named placeholders and leaves unknown ones', () => {
  const ctx = i18n('en');
  assert.equal(ctx.trf('{n} min ago', {n: 12}), '12 min ago');
  assert.equal(ctx.trf('{n} and {m}', {n: 1}), '1 and {m}');
});

test('plural picks the CLDR form', () => {
  const key = 'plural|one:{n} season|other:{n} seasons';
  const en = i18n('en');
  assert.equal(en.plural(key, 1), '1 season');
  assert.equal(en.plural(key, 3), '3 seasons');
  const ru = i18n('ru');
  const one = ru.plural(key, 1), few = ru.plural(key, 3), many = ru.plural(key, 5);
  assert.match(one, /^1 /);
  assert.notEqual(few.slice(2), many.slice(2));   // Russian: сезона vs сезонов
});

test('auto follows the TV language', () => {
  const ctx = load(['strings.js'], {language: 'de-AT'});
  ctx.I18N.set('auto');
  assert.equal(ctx.I18N.lang, 'de');
  assert.equal(ctx.I18N.locale(), 'de-AT');
  assert.equal(ctx.I18N.base, 'de');
});

test('formatDate uses the catalogue pattern', () => {
  const at = new Date(2026, 9, 24, 15, 5).getTime();
  assert.equal(i18n('en').formatDate('format|h:mm a', at), '3:05 PM');
  assert.equal(i18n('pt').formatDate('format|h:mm a', at), '15:05');
});

test('languages lists every catalogue with its own name', () => {
  const ctx = i18n('en');
  const langs = ctx.I18N.languages();
  const codes = langs.map((l) => (typeof l === 'string' ? l : l.code || l[0]));
  assert.ok(codes.length >= 20);
  assert.equal(ctx.I18N.languageName('pt'), 'Português');
});
