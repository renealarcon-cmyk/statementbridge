import type { EngineOptions } from './types';

/** Review-screen settings; empty string / 'auto' means "let the engine decide". */
export interface ReviewSettings {
  adapter: string;
  dateFormat: string;
  decimalSeparator: '' | '.' | ',';
  thousandsSeparator: '' | '.' | ',' | "'" | ' ';
  order: 'auto' | 'ascending' | 'descending';
  currency: string;
  sheet: string;
  delimiter: '' | ',' | ';' | 'tab' | '|';
  locale: '' | 'en' | 'es';
  redact: boolean;
  removeExactDuplicates: boolean;
}

export const defaultSettings: ReviewSettings = {
  adapter: '',
  dateFormat: '',
  decimalSeparator: '',
  thousandsSeparator: '',
  order: 'auto',
  currency: '',
  sheet: '',
  delimiter: '',
  locale: '',
  redact: false,
  removeExactDuplicates: false,
};

export function isValidCurrency(value: string): boolean {
  return /^[A-Za-z]{3}$/.test(value.trim());
}

/**
 * Build the options object sent to the engine. The engine rejects unknown
 * fields, so only known keys are ever set, and empty/auto values are omitted
 * entirely instead of being sent as null.
 */
export function buildOptions(
  settings: ReviewSettings,
  mapping: Record<string, string> | null,
): EngineOptions {
  const options: EngineOptions = {};
  if (settings.adapter) options.adapter = settings.adapter;
  if (isValidCurrency(settings.currency)) options.currency = settings.currency.trim().toUpperCase();
  if (settings.locale) options.locale = settings.locale;
  if (settings.dateFormat.trim()) options.date_format = settings.dateFormat.trim();
  if (settings.decimalSeparator) options.decimal_separator = settings.decimalSeparator;
  if (settings.thousandsSeparator) options.thousands_separator = settings.thousandsSeparator;
  if (settings.order !== 'auto') options.order = settings.order;
  if (mapping && Object.keys(mapping).length > 0) options.mapping = mapping;
  if (settings.sheet) options.sheet = settings.sheet;
  if (settings.delimiter) options.delimiter = settings.delimiter;
  if (settings.redact) options.redact = true;
  if (settings.removeExactDuplicates) options.remove_exact_duplicates = true;
  return options;
}
