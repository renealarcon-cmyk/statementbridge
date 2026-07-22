/**
 * Test fixtures mirroring real engine output for
 * fixtures/csv/semicolon_comma_es.csv (see canonical schema v1.0). Each
 * factory returns a fresh deep copy so tests can mutate freely.
 */
import type { EngineInfo } from '../lib/engine';
import type { InspectReport, Statement, Transaction } from '../lib/types';

export const engineInfoFixture: EngineInfo = {
  version: '0.1.0',
  schemaVersion: '1.0',
  adapters: [
    {
      id: 'generic-csv',
      version: '1.0.0',
      display_name: 'Generic (auto-detect)',
      description: 'Fallback adapter that auto-detects everything.',
      amount_mode: 'auto',
    },
    {
      id: 'generic-debit-credit',
      version: '1.0.0',
      display_name: 'Generic: separate debit/credit columns',
      description: 'For exports with separate debit and credit columns.',
      amount_mode: 'debit_credit',
    },
    {
      id: 'generic-signed-amount',
      version: '1.0.0',
      display_name: 'Generic: single signed amount column',
      description: 'For exports with one signed amount column.',
      amount_mode: 'signed',
    },
  ],
};

const baseInspect: InspectReport = {
  file_name: 'semicolon_comma_es.csv',
  format: 'csv',
  encoding: 'utf-8',
  delimiter: ';',
  sheet_name: null,
  sheet_names: [],
  total_rows: 8,
  header_row: 3,
  data_rows: 5,
  columns: [
    { index: 1, header: 'Fecha', field: 'date' },
    { index: 2, header: 'Concepto', field: 'description' },
    { index: 3, header: 'Cargo', field: 'debit' },
    { index: 4, header: 'Abono', field: 'credit' },
    { index: 5, header: 'Saldo', field: 'balance' },
  ],
  amount_mode: 'debit_credit',
  date_format: '%d/%m/%Y',
  decimal_separator: ',',
  thousands_separator: '.',
  currency_hint: null,
  sample_rows: [
    ['02/01/2026', 'Depósito inicial', '', '1.500,00', '1.500,00'],
    ['05/01/2026', 'Pago proveedor Ferretería Central', '249,99', '', '1.250,01'],
  ],
  errors: [],
  warnings: [
    {
      code: 'preamble_rows_skipped',
      message: '2 informational row(s) before the header were skipped',
    },
  ],
  adapter_id: 'generic-debit-credit',
};

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function inspectFixture(overrides: Partial<InspectReport> = {}): InspectReport {
  return { ...clone(baseInspect), ...overrides };
}

function tx(overrides: Partial<Transaction>): Transaction {
  return {
    id: 'sb1-0000000000000000',
    date: '2026-01-02',
    posted_at: null,
    description: 'Depósito inicial',
    reference: null,
    debit: null,
    credit: '1500.00',
    amount: '1500.00',
    balance: '1500.00',
    currency: null,
    original_row: 4,
    fingerprint: 'f'.repeat(64),
    warnings: [],
    ...overrides,
  };
}

const baseStatement: Statement = {
  schema_version: '1.0',
  source: {
    file_name: 'semicolon_comma_es.csv',
    format: 'csv',
    adapter_id: 'generic-debit-credit',
    imported_at: '2026-07-22T10:00:00.000Z',
    content_hash: 'a'.repeat(64),
  },
  account: { institution: null, account_name: null, account_masked: null, currency: null },
  period: { from: '2026-01-02', to: '2026-01-12' },
  transactions: [
    tx({ id: 'sb1-7b78fb4a32dfa75a' }),
    tx({
      id: 'sb1-a00b964d08677817',
      date: '2026-01-05',
      description: 'Pago proveedor Ferretería Central',
      debit: '249.99',
      credit: null,
      amount: '-249.99',
      balance: '1250.01',
      original_row: 5,
    }),
    tx({
      id: 'sb1-01e81719c948f95b',
      date: '2026-01-09',
      description: 'Factura consultoría proyecto Delta',
      credit: '875.50',
      amount: '875.50',
      balance: '2125.51',
      original_row: 6,
    }),
    tx({
      id: 'sb1-1fba73c7ed84350e',
      date: '2026-01-12',
      description: 'Suscripción software',
      debit: '49.00',
      credit: null,
      amount: '-49.00',
      balance: '2076.51',
      original_row: 7,
      duplicate_kind: 'possible',
      duplicate_of_row: 5,
      warnings: [{ code: 'possible_duplicate', message: 'similar to row 5' }],
    }),
  ],
  summary: {
    transaction_count: 4,
    total_debits: '298.99',
    total_credits: '2375.50',
    opening_balance: '0.00',
    closing_balance: '2076.51',
    exact_duplicates: 1,
    possible_duplicates: 2,
    duplicates_removed: 0,
  },
  validation: {
    status: 'valid_with_warnings',
    errors: [],
    warnings: [
      {
        code: 'preamble_rows_skipped',
        message: '2 informational row(s) before the header were skipped',
      },
      { code: 'footer_row_skipped', message: 'footer/summary row skipped', row: 8 },
    ],
    balance_check: {
      status: 'inconsistent',
      checked_pairs: 3,
      mismatches: 1,
      first_mismatch_row: 6,
      expected: '2125.51',
      actual: '2130.00',
    },
  },
};

export function statementFixture(overrides: Partial<Statement> = {}): Statement {
  return { ...clone(baseStatement), ...overrides };
}

export function invalidStatementFixture(): Statement {
  const statement = statementFixture();
  statement.validation.status = 'invalid';
  statement.validation.errors = [
    {
      code: 'invalid_amount',
      message: "amount 'abc' could not be parsed",
      row: 5,
      column: 'Cargo',
    },
  ];
  return statement;
}
