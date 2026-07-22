import { delimiterLabel, separatorLabel } from '../lib/format';
import { useI18n } from '../lib/i18n';
import type { ReviewSettings } from '../lib/options';
import type { AdapterInfo, InspectReport } from '../lib/types';
import { IssuesList } from './IssuesList';
import { MappingTable } from './MappingTable';
import { SettingsPanel } from './SettingsPanel';

interface ReviewScreenProps {
  report: InspectReport;
  fileName: string;
  settings: ReviewSettings;
  columnFields: Record<number, string>;
  adapters: AdapterInfo[];
  busyLabel: string | null;
  error: string | null;
  onSettingChange: <K extends keyof ReviewSettings>(key: K, value: ReviewSettings[K]) => void;
  onMappingChange: (columnIndex: number, field: string) => void;
  onBack: () => void;
  onNormalize: () => void;
}

/** Detection errors the user must fix through mapping before continuing. */
const BLOCKING_CODES = new Set(['no_date_column', 'no_amount_column']);

export function ReviewScreen(props: ReviewScreenProps) {
  const { t } = useI18n();
  const { report, settings } = props;
  const blocked = report.errors.some((issue) => BLOCKING_CODES.has(issue.code));
  const busy = props.busyLabel !== null;

  const detectionItems: Array<{ key: string; label: string; value: string; testId?: string }> = [
    { key: 'format', label: t('detect.format'), value: report.format.toUpperCase() },
    { key: 'encoding', label: t('detect.encoding'), value: report.encoding ?? t('detect.none') },
  ];
  if (report.format === 'csv') {
    detectionItems.push({
      key: 'delimiter',
      label: t('detect.delimiter'),
      value: delimiterLabel(report.delimiter, t),
      testId: 'detection-delimiter',
    });
  } else {
    detectionItems.push({
      key: 'sheet',
      label: t('detect.sheet'),
      value: report.sheet_name ?? t('detect.none'),
      testId: 'detection-sheet',
    });
  }
  detectionItems.push(
    {
      key: 'headerRow',
      label: t('detect.headerRow'),
      value: report.header_row !== null ? String(report.header_row) : t('detect.none'),
    },
    { key: 'dataRows', label: t('detect.dataRows'), value: String(report.data_rows) },
    {
      key: 'dateFormat',
      label: t('detect.dateFormat'),
      value: report.date_format ?? t('detect.auto'),
    },
    {
      key: 'decimal',
      label: t('detect.decimal'),
      value: separatorLabel(report.decimal_separator, t),
    },
    {
      key: 'thousands',
      label: t('detect.thousands'),
      value: separatorLabel(report.thousands_separator, t),
    },
    {
      key: 'adapter',
      label: t('detect.adapter'),
      value: report.adapter_id,
      testId: 'detection-adapter',
    },
  );

  return (
    <section aria-labelledby="review-heading">
      <h1 id="review-heading">{t('review.title')}</h1>
      <p className="muted">
        {t('review.file')}: <strong>{props.fileName}</strong>
      </p>

      {props.error ? (
        <div className="banner banner-invalid" role="alert">
          {props.error}
        </div>
      ) : null}

      <h2>{t('review.detectionTitle')}</h2>
      <dl className="detection-grid">
        {detectionItems.map((item) => (
          <div className="detection-item" key={item.key}>
            <dt>{item.label}</dt>
            <dd data-testid={item.testId}>{item.value}</dd>
          </div>
        ))}
      </dl>

      <h2>{t('review.previewTitle')}</h2>
      <p className="muted">{t('review.previewNote')}</p>
      <div className="table-wrap">
        <table className="preview-table">
          <thead>
            <tr>
              {report.columns.map((column) => (
                <th scope="col" key={column.index}>
                  {column.header || `#${column.index}`}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {report.sample_rows.map((row, rowIndex) => (
              <tr key={rowIndex}>
                {report.columns.map((_, cellIndex) => (
                  <td key={cellIndex}>{row[cellIndex] ?? ''}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="review-columns">
        <div>
          <h2>{t('review.mappingTitle')}</h2>
          <MappingTable
            columns={report.columns}
            columnFields={props.columnFields}
            onChange={props.onMappingChange}
          />
        </div>
        <div>
          <h2>{t('review.settingsTitle')}</h2>
          <SettingsPanel
            settings={settings}
            report={report}
            adapters={props.adapters}
            onChange={props.onSettingChange}
          />
        </div>
      </div>

      {report.errors.length + report.warnings.length > 0 ? (
        <>
          <h2>{t('review.issuesTitle')}</h2>
          <IssuesList errors={report.errors} warnings={report.warnings} />
        </>
      ) : null}

      {blocked ? (
        <p className="banner banner-invalid" data-testid="review-blocked">
          {t('review.blocked')}
        </p>
      ) : null}

      <div className="actions">
        <button type="button" className="btn" onClick={props.onBack}>
          {t('actions.back')}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          disabled={blocked || busy || props.error !== null}
          onClick={props.onNormalize}
        >
          {t('actions.normalize')}
        </button>
        {props.busyLabel ? <span className="muted">{props.busyLabel}</span> : null}
      </div>
    </section>
  );
}
