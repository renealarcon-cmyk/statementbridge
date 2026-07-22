import { useI18n } from '../lib/i18n';
import { isValidCurrency, type ReviewSettings } from '../lib/options';
import type { AdapterInfo, InspectReport } from '../lib/types';

interface SettingsPanelProps {
  settings: ReviewSettings;
  report: InspectReport;
  adapters: AdapterInfo[];
  onChange: <K extends keyof ReviewSettings>(key: K, value: ReviewSettings[K]) => void;
}

const DATE_FORMAT_PRESETS = ['%d/%m/%Y', '%m/%d/%Y', '%Y-%m-%d', '%d.%m.%Y', '%d-%m-%Y'];

export function SettingsPanel({ settings, report, adapters, onChange }: SettingsPanelProps) {
  const { t } = useI18n();
  const currencyPending = settings.currency.trim() !== '' && !isValidCurrency(settings.currency);

  return (
    <div className="settings-grid">
      <div className="field">
        <label htmlFor="set-adapter">{t('settings.adapter')}</label>
        <select
          id="set-adapter"
          value={settings.adapter}
          onChange={(event) => onChange('adapter', event.target.value)}
        >
          <option value="">{t('settings.auto')}</option>
          {adapters.map((adapter) => (
            <option key={adapter.id} value={adapter.id}>
              {adapter.display_name}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <label htmlFor="set-date-format">{t('settings.dateFormat')}</label>
        <input
          id="set-date-format"
          type="text"
          list="date-format-presets"
          placeholder={report.date_format ?? t('settings.auto')}
          value={settings.dateFormat}
          onChange={(event) => onChange('dateFormat', event.target.value)}
        />
        <datalist id="date-format-presets">
          {DATE_FORMAT_PRESETS.map((preset) => (
            <option key={preset} value={preset} />
          ))}
        </datalist>
        <p className="field-hint">{t('settings.dateFormatHint')}</p>
      </div>

      <div className="field">
        <label htmlFor="set-decimal">{t('settings.decimal')}</label>
        <select
          id="set-decimal"
          value={settings.decimalSeparator}
          onChange={(event) =>
            onChange('decimalSeparator', event.target.value as ReviewSettings['decimalSeparator'])
          }
        >
          <option value="">{t('settings.auto')}</option>
          <option value=".">{t('sep.dot')}</option>
          <option value=",">{t('sep.comma')}</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="set-thousands">{t('settings.thousands')}</label>
        <select
          id="set-thousands"
          value={settings.thousandsSeparator}
          onChange={(event) =>
            onChange(
              'thousandsSeparator',
              event.target.value as ReviewSettings['thousandsSeparator'],
            )
          }
        >
          <option value="">{t('settings.auto')}</option>
          <option value=".">{t('sep.dot')}</option>
          <option value=",">{t('sep.comma')}</option>
          <option value="'">{t('sep.apostrophe')}</option>
          <option value=" ">{t('sep.space')}</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="set-order">{t('settings.order')}</label>
        <select
          id="set-order"
          value={settings.order}
          onChange={(event) => onChange('order', event.target.value as ReviewSettings['order'])}
        >
          <option value="auto">{t('order.auto')}</option>
          <option value="ascending">{t('order.ascending')}</option>
          <option value="descending">{t('order.descending')}</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="set-currency">{t('settings.currency')}</label>
        <input
          id="set-currency"
          type="text"
          maxLength={3}
          autoComplete="off"
          spellCheck={false}
          placeholder={report.currency_hint ?? t('settings.currencyHint')}
          value={settings.currency}
          onChange={(event) => onChange('currency', event.target.value)}
        />
        {currencyPending ? <p className="field-hint">{t('settings.currencyInvalid')}</p> : null}
      </div>

      {report.sheet_names.length > 1 ? (
        <div className="field">
          <label htmlFor="set-sheet">{t('settings.sheet')}</label>
          <select
            id="set-sheet"
            value={settings.sheet || (report.sheet_name ?? '')}
            onChange={(event) => onChange('sheet', event.target.value)}
          >
            {report.sheet_names.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </div>
      ) : null}

      {report.format === 'csv' ? (
        <div className="field">
          <label htmlFor="set-delimiter">{t('settings.delimiter')}</label>
          <select
            id="set-delimiter"
            value={settings.delimiter}
            onChange={(event) =>
              onChange('delimiter', event.target.value as ReviewSettings['delimiter'])
            }
          >
            <option value="">{t('settings.auto')}</option>
            <option value=",">{t('delim.comma')}</option>
            <option value=";">{t('delim.semicolon')}</option>
            <option value="tab">{t('delim.tab')}</option>
            <option value="|">{t('delim.pipe')}</option>
          </select>
        </div>
      ) : null}

      <div className="field">
        <label htmlFor="set-locale">{t('settings.locale')}</label>
        <select
          id="set-locale"
          value={settings.locale}
          onChange={(event) => onChange('locale', event.target.value as ReviewSettings['locale'])}
        >
          <option value="">{t('settings.auto')}</option>
          <option value="en">{t('locale.en')}</option>
          <option value="es">{t('locale.es')}</option>
        </select>
      </div>

      <div className="field field-checkbox">
        <input
          id="set-redact"
          type="checkbox"
          checked={settings.redact}
          onChange={(event) => onChange('redact', event.target.checked)}
        />
        <label htmlFor="set-redact">{t('settings.redact')}</label>
      </div>

      <div className="field field-checkbox">
        <input
          id="set-remove-duplicates"
          type="checkbox"
          checked={settings.removeExactDuplicates}
          onChange={(event) => onChange('removeExactDuplicates', event.target.checked)}
        />
        <label htmlFor="set-remove-duplicates">{t('settings.removeDuplicates')}</label>
      </div>
    </div>
  );
}
