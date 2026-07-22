import { useState } from 'react';
import { useI18n } from '../lib/i18n';
import type { Transaction } from '../lib/types';
import { Pagination } from './Pagination';

const PAGE_SIZE = 100;

interface TransactionsTableProps {
  transactions: Transaction[];
}

function Amount({ value }: { value: string | null }) {
  if (value === null) return <td className="num">—</td>;
  return <td className={value.startsWith('-') ? 'num neg' : 'num'}>{value}</td>;
}

export function TransactionsTable({ transactions }: TransactionsTableProps) {
  const { t } = useI18n();
  const [page, setPage] = useState(1);

  if (transactions.length === 0) return <p className="muted">{t('table.empty')}</p>;

  const totalPages = Math.ceil(transactions.length / PAGE_SIZE);
  const safePage = Math.min(page, totalPages);
  const start = (safePage - 1) * PAGE_SIZE;
  const rows = transactions.slice(start, start + PAGE_SIZE);

  return (
    <>
      <div className="table-wrap">
        <table className="tx-table">
          <caption className="visually-hidden">{t('table.caption')}</caption>
          <thead>
            <tr>
              <th scope="col">{t('table.date')}</th>
              <th scope="col">{t('table.description')}</th>
              <th scope="col">{t('table.reference')}</th>
              <th scope="col" className="num">
                {t('table.debit')}
              </th>
              <th scope="col" className="num">
                {t('table.credit')}
              </th>
              <th scope="col" className="num">
                {t('table.amount')}
              </th>
              <th scope="col" className="num">
                {t('table.balance')}
              </th>
              <th scope="col" className="num">
                {t('table.row')}
              </th>
              <th scope="col">{t('table.flags')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((tx) => (
              <tr key={tx.id}>
                <td className="nowrap">{tx.date}</td>
                <td>{tx.description}</td>
                <td>{tx.reference ?? '—'}</td>
                <Amount value={tx.debit} />
                <Amount value={tx.credit} />
                <Amount value={tx.amount} />
                <Amount value={tx.balance} />
                <td className="num">{tx.original_row}</td>
                <td>
                  {tx.duplicate_kind ? (
                    <span className={`chip chip-dup-${tx.duplicate_kind}`}>
                      {t(`dup.${tx.duplicate_kind}`)}
                    </span>
                  ) : null}
                  {tx.warnings.length > 0 ? (
                    <span
                      className="warn-marker"
                      title={tx.warnings.map((w) => `${w.code}: ${w.message}`).join('\n')}
                    >
                      <span aria-hidden="true">⚠</span>
                      <span className="visually-hidden">{t('table.warnMarker')}</span>
                    </span>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Pagination
        page={safePage}
        totalPages={totalPages}
        from={start + 1}
        to={start + rows.length}
        totalItems={transactions.length}
        onPage={setPage}
      />
    </>
  );
}
