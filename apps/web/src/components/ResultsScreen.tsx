import { useI18n } from '../lib/i18n';
import type { Statement, ValidationStatus } from '../lib/types';
import { IssuesList } from './IssuesList';
import { SummaryCards } from './SummaryCards';
import { TransactionsTable } from './TransactionsTable';

interface ResultsScreenProps {
  statement: Statement;
  error: string | null;
  onDownloadJson: () => void;
  onDownloadCsv: () => void;
  onStartOver: () => void;
}

function StatusIcon({ status }: { status: ValidationStatus }) {
  const common = {
    viewBox: '0 0 24 24',
    width: 22,
    height: 22,
    'aria-hidden': true,
    focusable: false,
  } as const;
  if (status === 'valid') {
    return (
      <svg {...common}>
        <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" strokeWidth="2" />
        <path d="m7.5 12.5 3 3 6-7" fill="none" stroke="currentColor" strokeWidth="2" />
      </svg>
    );
  }
  if (status === 'valid_with_warnings') {
    return (
      <svg {...common}>
        <path
          d="M12 3 2.5 20h19L12 3Z"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinejoin="round"
        />
        <path d="M12 9.5v5" stroke="currentColor" strokeWidth="2" />
        <circle cx="12" cy="17" r="1.2" fill="currentColor" />
      </svg>
    );
  }
  return (
    <svg {...common}>
      <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="m8.5 8.5 7 7m0-7-7 7" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

export function ResultsScreen(props: ResultsScreenProps) {
  const { t } = useI18n();
  const { statement } = props;
  const status = statement.validation.status;

  return (
    <section aria-labelledby="results-heading">
      <h1 id="results-heading">{t('results.title')}</h1>
      <p className="muted">
        {t('review.file')}: <strong>{statement.source.file_name}</strong>
      </p>

      {props.error ? (
        <div className="banner banner-invalid" role="alert">
          {props.error}
        </div>
      ) : null}

      <div className={`banner banner-${status}`} data-testid="status-banner" role="status">
        <StatusIcon status={status} />
        <div>
          <strong>{t(`status.${status}`)}</strong>
          <p>{t(`statusDetail.${status}`)}</p>
        </div>
      </div>

      <SummaryCards statement={statement} />

      <IssuesList
        errors={statement.validation.errors}
        warnings={statement.validation.warnings}
        collapsible
      />

      <TransactionsTable transactions={statement.transactions} />

      <div className="actions">
        <button type="button" className="btn btn-primary" onClick={props.onDownloadJson}>
          {t('actions.downloadJson')}
        </button>
        <button type="button" className="btn btn-primary" onClick={props.onDownloadCsv}>
          {t('actions.downloadCsv')}
        </button>
        <button type="button" className="btn" onClick={props.onStartOver}>
          {t('actions.startOver')}
        </button>
      </div>
    </section>
  );
}
