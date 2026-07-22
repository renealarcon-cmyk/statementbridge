/**
 * Typed wrapper around the generated wasm-bindgen package. All engine access
 * goes through this module so components never import the pkg directly (tests
 * mock this module) and so envelope parsing lives in exactly one place.
 *
 * The engine runs fully in-memory in this browser tab: no network requests
 * are made during processing.
 */
import initWasm, {
  adapters_json,
  convert_file,
  init as installPanicHook,
  inspect_file,
  schema_version,
  statement_to_csv,
  version,
} from './wasm/pkg/statementbridge_wasm.js';
import type { AdapterInfo, EngineOptions, InspectReport, Statement } from './types';

/** Failure reported by the engine (or by envelope parsing). */
export class EngineFailure extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'EngineFailure';
    this.code = code;
  }
}

type Envelope<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string } };

function unwrap<T>(raw: string): T {
  let envelope: Envelope<T>;
  try {
    envelope = JSON.parse(raw) as Envelope<T>;
  } catch {
    throw new EngineFailure('bad_envelope', 'The engine returned malformed JSON.');
  }
  if (envelope.ok) return envelope.data;
  throw new EngineFailure(envelope.error.code, envelope.error.message);
}

let ready: Promise<void> | null = null;

function ensureReady(): Promise<void> {
  ready ??= (async () => {
    if (import.meta.env.VITE_EMBED_WASM === '1') {
      // Single-file demo build (scripts/build-demo.sh): the wasm binary is
      // embedded as base64 so the whole app works from one HTML file opened
      // via file:// — no server, no network. Dead-code-eliminated (together
      // with the placeholder module) in normal builds.
      const { WASM_B64 } = await import('./wasm-inline');
      const binary = atob(WASM_B64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
      await initWasm({ module_or_path: bytes });
    } else {
      await initWasm();
    }
    installPanicHook();
  })();
  return ready;
}

/**
 * The wasm calls are synchronous and can block the main thread for large
 * files (v0.1 has no worker). Yielding one macrotask lets React paint the
 * busy state before the blocking call starts.
 */
function yieldToPaint(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

export interface EngineInfo {
  version: string;
  schemaVersion: string;
  adapters: AdapterInfo[];
}

export async function initEngine(): Promise<EngineInfo> {
  await ensureReady();
  return {
    version: version(),
    schemaVersion: schema_version(),
    adapters: unwrap<AdapterInfo[]>(adapters_json()),
  };
}

export async function inspect(
  bytes: Uint8Array,
  fileName: string,
  options: EngineOptions,
): Promise<InspectReport> {
  await ensureReady();
  await yieldToPaint();
  return unwrap<InspectReport>(inspect_file(bytes, fileName, JSON.stringify(options)));
}

export async function convert(
  bytes: Uint8Array,
  fileName: string,
  options: EngineOptions,
): Promise<Statement> {
  await ensureReady();
  await yieldToPaint();
  return unwrap<Statement>(convert_file(bytes, fileName, JSON.stringify(options)));
}

export async function toCsv(statement: Statement): Promise<string> {
  await ensureReady();
  await yieldToPaint();
  return unwrap<string>(statement_to_csv(JSON.stringify(statement)));
}
