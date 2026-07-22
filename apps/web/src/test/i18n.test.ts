import { describe, expect, it } from 'vitest';
import { detectLang, dictionaries, interpolate } from '../lib/i18n';

describe('i18n', () => {
  it('has identical key sets in EN and ES', () => {
    const enKeys = Object.keys(dictionaries.en).sort();
    const esKeys = Object.keys(dictionaries.es).sort();
    expect(esKeys).toEqual(enKeys);
  });

  it('has no empty translations', () => {
    for (const lang of ['en', 'es'] as const) {
      for (const [key, value] of Object.entries(dictionaries[lang])) {
        expect(value.trim(), `${lang}:${key}`).not.toBe('');
      }
    }
  });

  it('detects Spanish browser languages and defaults to English', () => {
    expect(detectLang('es')).toBe('es');
    expect(detectLang('es-MX')).toBe('es');
    expect(detectLang('ES-ES')).toBe('es');
    expect(detectLang('en-US')).toBe('en');
    expect(detectLang('fr-FR')).toBe('en');
  });

  it('interpolates parameters and keeps unknown placeholders', () => {
    expect(interpolate('Page {page} of {total}', { page: 2, total: 9 })).toBe('Page 2 of 9');
    expect(interpolate('keep {unknown}', { other: 1 })).toBe('keep {unknown}');
    expect(interpolate('no params')).toBe('no params');
  });
});
