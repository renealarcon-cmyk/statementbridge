import { useI18n } from '../lib/i18n';

interface PaginationProps {
  page: number;
  totalPages: number;
  from: number;
  to: number;
  totalItems: number;
  onPage: (page: number) => void;
}

export function Pagination({ page, totalPages, from, to, totalItems, onPage }: PaginationProps) {
  const { t } = useI18n();
  if (totalPages <= 1) return null;
  return (
    <nav className="pagination" aria-label={t('pagination.label')}>
      <button
        type="button"
        className="btn"
        disabled={page <= 1}
        aria-label={t('pagination.prev')}
        onClick={() => onPage(page - 1)}
      >
        ‹
      </button>
      <span aria-current="page">{t('pagination.page', { page, total: totalPages })}</span>
      <button
        type="button"
        className="btn"
        disabled={page >= totalPages}
        aria-label={t('pagination.next')}
        onClick={() => onPage(page + 1)}
      >
        ›
      </button>
      <span className="muted">{t('pagination.range', { from, to, total: totalItems })}</span>
    </nav>
  );
}
