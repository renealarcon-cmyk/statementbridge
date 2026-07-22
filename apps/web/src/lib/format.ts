import type { MessageKey, Translate } from './i18n';

export const CANONICAL_FIELDS = [
  'date',
  'posted_at',
  'description',
  'reference',
  'debit',
  'credit',
  'amount',
  'balance',
  'currency',
  'account',
] as const;

export type CanonicalField = (typeof CANONICAL_FIELDS)[number];

export function humanBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KiB', 'MiB', 'GiB'];
  let value = bytes;
  let unit = 'B';
  for (const next of units) {
    if (value < 1024) break;
    value /= 1024;
    unit = next;
  }
  return `${value.toFixed(1)} ${unit}`;
}

/** Human label for a delimiter reported by the engine (`\t` arrives as "\\t"). */
export function delimiterLabel(delimiter: string | null, t: Translate): string {
  if (delimiter === null) return t('detect.none');
  const known: Record<string, MessageKey> = {
    ',': 'delim.comma',
    ';': 'delim.semicolon',
    '\\t': 'delim.tab',
    '\t': 'delim.tab',
    '|': 'delim.pipe',
  };
  const key = known[delimiter];
  return key ? t(key) : delimiter;
}

export function separatorLabel(separator: string | null, t: Translate): string {
  if (separator === null) return t('detect.none');
  const known: Record<string, MessageKey> = {
    '.': 'sep.dot',
    ',': 'sep.comma',
    "'": 'sep.apostrophe',
    ' ': 'sep.space',
  };
  const key = known[separator];
  return key ? t(key) : separator;
}

/** File name for exports: statement.csv -> statement.normalized.json */
export function exportName(sourceName: string, extension: 'json' | 'csv'): string {
  const base = sourceName.replace(/\.[^.]+$/, '') || 'statement';
  return `${base}.normalized.${extension}`;
}
