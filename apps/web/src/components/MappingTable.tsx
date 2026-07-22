import { CANONICAL_FIELDS } from '../lib/format';
import { useI18n } from '../lib/i18n';
import type { InspectColumn } from '../lib/types';

interface MappingTableProps {
  columns: InspectColumn[];
  /** Display state: 1-based column index -> canonical field ('' = unmapped). */
  columnFields: Record<number, string>;
  onChange: (columnIndex: number, field: string) => void;
}

export function MappingTable({ columns, columnFields, onChange }: MappingTableProps) {
  const { t } = useI18n();
  const assigned = Object.values(columnFields).filter(Boolean);
  const hasDuplicates = new Set(assigned).size !== assigned.length;

  return (
    <div className="table-wrap">
      <table className="mapping-table">
        <thead>
          <tr>
            <th scope="col">{t('mapping.column')}</th>
            <th scope="col">{t('mapping.header')}</th>
            <th scope="col">{t('mapping.field')}</th>
          </tr>
        </thead>
        <tbody>
          {columns.map((column) => (
            <tr key={column.index}>
              <th scope="row">{column.index}</th>
              <td>{column.header || t('detect.none')}</td>
              <td>
                <select
                  data-testid={`mapping-select-${column.index}`}
                  aria-label={t('mapping.selectLabel', {
                    column: column.header || String(column.index),
                  })}
                  value={columnFields[column.index] ?? ''}
                  onChange={(event) => onChange(column.index, event.target.value)}
                >
                  <option value="">{t('mapping.unmapped')}</option>
                  {CANONICAL_FIELDS.map((field) => (
                    <option key={field} value={field}>
                      {t(`field.${field}`)}
                    </option>
                  ))}
                </select>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {hasDuplicates ? <p className="field-hint">{t('review.duplicateField')}</p> : null}
    </div>
  );
}
