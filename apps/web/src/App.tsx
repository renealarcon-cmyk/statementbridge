import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { DropZone } from './components/DropZone';
import { Header } from './components/Header';
import { ResultsScreen } from './components/ResultsScreen';
import { ReviewScreen } from './components/ReviewScreen';
import { downloadText, revokeAllDownloads } from './lib/download';
import { convert, EngineFailure, initEngine, inspect, toCsv, type EngineInfo } from './lib/engine';
import { exportName, humanBytes } from './lib/format';
import {
  detectLang,
  I18nContext,
  translate,
  type I18n,
  type Lang,
  type MessageKey,
  type TranslateParams,
} from './lib/i18n';
import { buildOptions, defaultSettings, type ReviewSettings } from './lib/options';
import type { InspectReport, Statement } from './lib/types';

const MAX_FILE_BYTES = 50 * 1024 * 1024;

type Stage = 'home' | 'review' | 'results';
type Busy = null | 'inspect' | 'update' | 'convert';

type EngineState =
  | { status: 'loading' }
  | { status: 'ready'; info: EngineInfo }
  | { status: 'failed'; message: string };

function fieldsFromReport(report: InspectReport): Record<number, string> {
  const fields: Record<number, string> = {};
  for (const column of report.columns) fields[column.index] = column.field ?? '';
  return fields;
}

export default function App() {
  const [lang, setLang] = useState<Lang>(() => detectLang());
  const [stage, setStage] = useState<Stage>('home');
  const [engineState, setEngineState] = useState<EngineState>({ status: 'loading' });
  const [fileName, setFileName] = useState('');
  const [report, setReport] = useState<InspectReport | null>(null);
  const [settings, setSettings] = useState<ReviewSettings>(defaultSettings);
  const [columnFields, setColumnFields] = useState<Record<number, string>>({});
  const [mappingOverride, setMappingOverride] = useState<Record<string, string> | null>(null);
  const [statement, setStatement] = useState<Statement | null>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const [error, setError] = useState<string | null>(null);
  const [homeError, setHomeError] = useState<string | null>(null);

  const bytesRef = useRef<Uint8Array | null>(null);
  const lastInspectedRef = useRef('');

  const t = useCallback(
    (key: MessageKey, params?: TranslateParams) => translate(lang, key, params),
    [lang],
  );
  const i18n = useMemo<I18n>(() => ({ lang, setLang, t }), [lang, t]);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  useEffect(() => {
    let cancelled = false;
    initEngine()
      .then((info) => {
        if (!cancelled) setEngineState({ status: 'ready', info });
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          const message = cause instanceof Error ? cause.message : String(cause);
          setEngineState({ status: 'failed', message });
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const describeError = useCallback(
    (cause: unknown): string => {
      if (cause instanceof EngineFailure) {
        return t('error.engine', { code: cause.code, message: cause.message });
      }
      const message = cause instanceof Error ? cause.message : String(cause);
      return t('error.generic', { message });
    },
    [t],
  );

  const handleFile = useCallback(
    async (file: File) => {
      setHomeError(null);
      if (file.size > MAX_FILE_BYTES) {
        setHomeError(t('home.errorTooLarge', { size: humanBytes(file.size) }));
        return;
      }
      setBusy('inspect');
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        bytesRef.current = bytes;
        const nextReport = await inspect(bytes, file.name, {});
        lastInspectedRef.current = '{}';
        setFileName(file.name);
        setReport(nextReport);
        setColumnFields(fieldsFromReport(nextReport));
        setSettings(defaultSettings);
        setMappingOverride(null);
        setError(null);
        setStage('review');
      } catch (cause) {
        bytesRef.current = null;
        setHomeError(describeError(cause));
      } finally {
        setBusy(null);
      }
    },
    [t, describeError],
  );

  // Re-run inspection (debounced) whenever settings or the mapping change.
  useEffect(() => {
    if (stage !== 'review') return;
    const bytes = bytesRef.current;
    if (!bytes) return;
    const options = buildOptions(settings, mappingOverride);
    const key = JSON.stringify(options);
    if (key === lastInspectedRef.current) return;
    const timer = window.setTimeout(() => {
      lastInspectedRef.current = key;
      setBusy('update');
      inspect(bytes, fileName, options)
        .then((nextReport) => {
          setReport(nextReport);
          setColumnFields(fieldsFromReport(nextReport));
          setError(null);
        })
        .catch((cause: unknown) => setError(describeError(cause)))
        .finally(() => setBusy(null));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [stage, settings, mappingOverride, fileName, describeError]);

  const handleSettingChange = useCallback(
    <K extends keyof ReviewSettings>(settingKey: K, value: ReviewSettings[K]) => {
      setSettings((previous) => ({ ...previous, [settingKey]: value }));
      // Changing the sheet or delimiter changes the column layout entirely:
      // drop any mapping override so detection starts fresh.
      if (settingKey === 'sheet' || settingKey === 'delimiter') setMappingOverride(null);
    },
    [],
  );

  const handleMappingChange = useCallback((columnIndex: number, field: string) => {
    setColumnFields((previous) => {
      const next = { ...previous, [columnIndex]: field };
      // Full field -> "#column" map from the displayed state. If the same
      // field is picked for several columns, the highest column wins (the UI
      // shows a note); the engine receives one column per field.
      const map: Record<string, string> = {};
      for (const [index, assigned] of Object.entries(next)) {
        if (assigned) map[assigned] = `#${index}`;
      }
      setMappingOverride(map);
      return next;
    });
  }, []);

  const handleNormalize = useCallback(async () => {
    const bytes = bytesRef.current;
    if (!bytes) return;
    setBusy('convert');
    setError(null);
    try {
      const options = {
        ...buildOptions(settings, mappingOverride),
        imported_at: new Date().toISOString(),
      };
      const nextStatement = await convert(bytes, fileName, options);
      setStatement(nextStatement);
      setStage('results');
    } catch (cause) {
      setError(describeError(cause));
    } finally {
      setBusy(null);
    }
  }, [settings, mappingOverride, fileName, describeError]);

  const resetAll = useCallback(() => {
    revokeAllDownloads();
    bytesRef.current = null;
    lastInspectedRef.current = '';
    setFileName('');
    setReport(null);
    setStatement(null);
    setSettings(defaultSettings);
    setColumnFields({});
    setMappingOverride(null);
    setError(null);
    setHomeError(null);
    setBusy(null);
    setStage('home');
  }, []);

  const handleDownloadJson = useCallback(() => {
    if (!statement) return;
    downloadText(
      JSON.stringify(statement, null, 2),
      'application/json',
      exportName(statement.source.file_name, 'json'),
    );
  }, [statement]);

  const handleDownloadCsv = useCallback(async () => {
    if (!statement) return;
    try {
      const csv = await toCsv(statement);
      downloadText(csv, 'text/csv', exportName(statement.source.file_name, 'csv'));
    } catch (cause) {
      setError(describeError(cause));
    }
  }, [statement, describeError]);

  const busyLabel =
    busy === 'inspect'
      ? t('processing.inspect')
      : busy === 'update'
        ? t('processing.update')
        : busy === 'convert'
          ? t('processing.convert')
          : null;
  const announcement = busyLabel ?? t(`announce.${stage}`);

  return (
    <I18nContext.Provider value={i18n}>
      <div className="app">
        <Header />
        <div aria-live="polite" role="status" className="visually-hidden">
          {announcement}
        </div>
        <main aria-busy={busy !== null}>
          {stage === 'home' ? (
            <section className="home" aria-labelledby="home-heading">
              <h1 id="home-heading">{t('app.title')}</h1>
              <p className="tagline">{t('app.tagline')}</p>
              <p className="privacy-badge">
                <svg
                  viewBox="0 0 24 24"
                  width="18"
                  height="18"
                  aria-hidden="true"
                  focusable="false"
                >
                  <rect
                    x="5"
                    y="10"
                    width="14"
                    height="10"
                    rx="2"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  />
                  <path
                    d="M8 10V7a4 4 0 0 1 8 0v3"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  />
                </svg>
                {t('privacy.line')}
              </p>
              <p className="muted">{t('privacy.detail')}</p>

              {engineState.status === 'failed' ? (
                <div className="banner banner-invalid" role="alert">
                  {t('engine.initFailed', { message: engineState.message })}
                </div>
              ) : (
                <DropZone
                  disabled={engineState.status !== 'ready' || busy !== null}
                  onFile={handleFile}
                />
              )}
              {engineState.status === 'loading' ? (
                <p className="muted">{t('engine.loading')}</p>
              ) : null}
              {busyLabel ? <p className="muted processing">{busyLabel}</p> : null}
              {homeError ? (
                <div className="banner banner-invalid" role="alert" data-testid="home-error">
                  {homeError}
                </div>
              ) : null}

              <h2>{t('home.howTitle')}</h2>
              <ol className="how-list">
                <li>{t('home.how1')}</li>
                <li>{t('home.how2')}</li>
                <li>{t('home.how3')}</li>
              </ol>
            </section>
          ) : null}

          {stage === 'review' && report ? (
            <ReviewScreen
              report={report}
              fileName={fileName}
              settings={settings}
              columnFields={columnFields}
              adapters={engineState.status === 'ready' ? engineState.info.adapters : []}
              busyLabel={busyLabel}
              error={error}
              onSettingChange={handleSettingChange}
              onMappingChange={handleMappingChange}
              onBack={resetAll}
              onNormalize={() => void handleNormalize()}
            />
          ) : null}

          {stage === 'results' && statement ? (
            <ResultsScreen
              statement={statement}
              error={error}
              onDownloadJson={handleDownloadJson}
              onDownloadCsv={() => void handleDownloadCsv()}
              onStartOver={resetAll}
            />
          ) : null}
        </main>
        <footer className="footer">
          <span>
            {engineState.status === 'ready'
              ? t('footer.engine', { version: engineState.info.version })
              : t('app.title')}
          </span>
          <span aria-hidden="true">·</span>
          <span>{t('footer.local')}</span>
          <span aria-hidden="true">·</span>
          <span>{t('footer.notAdvice')}</span>
          <span aria-hidden="true">·</span>
          <span>{t('footer.license')}</span>
        </footer>
      </div>
    </I18nContext.Provider>
  );
}
