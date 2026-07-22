/** Data shapes produced by the StatementBridge engine (see schemas/statementbridge-v1.schema.json). */

export interface Issue {
  code: string;
  message: string;
  row?: number;
  column?: string;
}

export interface InspectColumn {
  /** 1-based column index. */
  index: number;
  header: string;
  field: string | null;
}

export interface InspectReport {
  file_name: string;
  format: 'csv' | 'xlsx';
  encoding: string | null;
  delimiter: string | null;
  sheet_name: string | null;
  sheet_names: string[];
  total_rows: number;
  header_row: number | null;
  data_rows: number;
  columns: InspectColumn[];
  amount_mode: string;
  date_format: string | null;
  decimal_separator: string;
  thousands_separator: string | null;
  currency_hint: string | null;
  /** Masked sample rows — safe to render. */
  sample_rows: string[][];
  errors: Issue[];
  warnings: Issue[];
  adapter_id: string;
}

export type DuplicateKind = 'exact' | 'possible' | 'repeated';

export interface Transaction {
  id: string;
  date: string;
  posted_at: string | null;
  description: string;
  reference: string | null;
  debit: string | null;
  credit: string | null;
  amount: string;
  balance: string | null;
  currency: string | null;
  original_row: number;
  fingerprint: string;
  duplicate_of_row?: number;
  duplicate_kind?: DuplicateKind;
  warnings: Issue[];
}

export type ValidationStatus = 'valid' | 'valid_with_warnings' | 'invalid';
export type BalanceStatus = 'consistent' | 'inconsistent' | 'not_available';

export interface BalanceCheck {
  status: BalanceStatus;
  checked_pairs: number;
  mismatches: number;
  first_mismatch_row: number | null;
  expected: string | null;
  actual: string | null;
}

export interface Statement {
  schema_version: string;
  source: {
    file_name: string;
    format: 'csv' | 'xlsx';
    adapter_id: string;
    imported_at: string | null;
    content_hash: string;
  };
  account: {
    institution: string | null;
    account_name: string | null;
    account_masked: string | null;
    currency: string | null;
  };
  period: { from: string; to: string } | null;
  transactions: Transaction[];
  summary: {
    transaction_count: number;
    total_debits: string;
    total_credits: string;
    opening_balance: string | null;
    closing_balance: string | null;
    exact_duplicates: number;
    possible_duplicates: number;
    duplicates_removed: number;
  };
  validation: {
    status: ValidationStatus;
    errors: Issue[];
    warnings: Issue[];
    balance_check: BalanceCheck;
  };
}

export interface AdapterInfo {
  id: string;
  version: string;
  display_name: string;
  description: string;
  amount_mode: string;
}

/**
 * Options accepted by the engine. Field names mirror the wasm contract; the
 * engine rejects unknown fields, so never add extras and omit empty values.
 */
export interface EngineOptions {
  adapter?: string;
  currency?: string;
  locale?: 'en' | 'es';
  date_format?: string;
  decimal_separator?: string;
  thousands_separator?: string;
  order?: 'ascending' | 'descending';
  mapping?: Record<string, string>;
  sheet?: string;
  delimiter?: string;
  redact?: boolean;
  remove_exact_duplicates?: boolean;
  imported_at?: string;
}
