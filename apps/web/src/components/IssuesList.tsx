import { useI18n } from '../lib/i18n';
import type { Issue } from '../lib/types';

interface IssuesListProps {
  errors: Issue[];
  warnings: Issue[];
  collapsible?: boolean;
}

/**
 * Issue codes are rendered as chips; the human message next to them comes
 * from the engine and is English-only in v0.1 (documented limitation) — it is
 * rendered as-is.
 */
function IssueItems({ issues, kind }: { issues: Issue[]; kind: 'error' | 'warning' }) {
  const { t } = useI18n();
  return (
    <>
      {issues.map((issue, index) => (
        <li key={`${kind}-${index}`} className={`issue issue-${kind}`}>
          <span className="chip">{issue.code}</span>
          <span>{issue.message}</span>
          {issue.row !== undefined ? (
            <span className="issue-row">({t('issues.row', { row: issue.row })})</span>
          ) : null}
        </li>
      ))}
    </>
  );
}

export function IssuesList({ errors, warnings, collapsible = false }: IssuesListProps) {
  const { t } = useI18n();
  const total = errors.length + warnings.length;

  const body =
    total === 0 ? (
      <p className="muted">{t('issues.none')}</p>
    ) : (
      <ul className="issues-list">
        <IssueItems issues={errors} kind="error" />
        <IssueItems issues={warnings} kind="warning" />
      </ul>
    );

  if (!collapsible) return body;

  return (
    <details className="issues" open={errors.length > 0} data-testid="issues-section">
      <summary>
        {t('issues.title')} ({total})
      </summary>
      {body}
    </details>
  );
}
