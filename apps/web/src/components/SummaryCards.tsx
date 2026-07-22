import { useI18n } from '../lib/i18n';
import type { Statement } from '../lib/types';

interface SummaryCardsProps {
  statement: Statement;
}

function Card({
  label,
  value,
  detail,
  testId,
}: {
  label: string;
  value: string;
  detail?: string;
  testId: string;
}) {
  return (
    <div className="card">
      <div className="card-label">{label}</div>
      <div className="card-value" data-testid={testId}>
        {value}
      </div>
      {detail ? <div className="card-detail">{detail}</div> : null}
    </div>
  );
}

export function SummaryCards({ statement }: SummaryCardsProps) {
  const { t } = useI18n();
  const { summary, validation, period, account } = statement;
  const currency = account.currency ? ` ${account.currency}` : '';
  const balance = validation.balance_check;

  let balanceDetail: string | undefined;
  if (balance.status === 'inconsistent' && balance.first_mismatch_row !== null) {
    balanceDetail = t('balance.mismatch', {
      row: balance.first_mismatch_row,
      expected: balance.expected ?? '—',
      actual: balance.actual ?? '—',
    });
  } else if (balance.status === 'consistent') {
    balanceDetail = t('balance.checked', { count: balance.checked_pairs });
  }

  return (
    <div className="cards">
      <Card
        label={t('summary.transactions')}
        value={String(summary.transaction_count)}
        testId="card-transactions"
      />
      <Card
        label={t('summary.debits')}
        value={`${summary.total_debits}${currency}`}
        testId="card-debits"
      />
      <Card
        label={t('summary.credits')}
        value={`${summary.total_credits}${currency}`}
        testId="card-credits"
      />
      <Card
        label={t('summary.period')}
        value={period ? `${period.from} → ${period.to}` : '—'}
        testId="card-period"
      />
      <Card
        label={t('summary.duplicates')}
        value={t('summary.dupValue', {
          exact: summary.exact_duplicates,
          possible: summary.possible_duplicates,
        })}
        detail={
          summary.duplicates_removed > 0
            ? t('summary.dupRemoved', { count: summary.duplicates_removed })
            : undefined
        }
        testId="card-duplicates"
      />
      <Card
        label={t('summary.balance')}
        value={t(`balance.${balance.status}`)}
        detail={balanceDetail}
        testId="card-balance"
      />
    </div>
  );
}
