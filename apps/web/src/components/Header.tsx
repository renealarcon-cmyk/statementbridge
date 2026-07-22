import { useI18n } from '../lib/i18n';

export function Header() {
  const { lang, setLang, t } = useI18n();
  return (
    <header className="header">
      <div className="brand">
        <svg viewBox="0 0 32 32" width="28" height="28" aria-hidden="true" focusable="false">
          <rect width="32" height="32" rx="6" fill="var(--primary)" />
          <path d="M6 21c4-8 16-8 20 0" stroke="#fff" strokeWidth="2.5" fill="none" />
          <path d="M6 21v5M16 15v11M26 21v5" stroke="#fff" strokeWidth="2.5" />
        </svg>
        <span className="brand-name">{t('app.title')}</span>
      </div>
      <div className="lang-toggle" role="group" aria-label={t('lang.label')}>
        <button
          type="button"
          className={lang === 'en' ? 'lang-btn active' : 'lang-btn'}
          aria-pressed={lang === 'en'}
          aria-label={t('lang.en')}
          onClick={() => setLang('en')}
        >
          EN
        </button>
        <button
          type="button"
          className={lang === 'es' ? 'lang-btn active' : 'lang-btn'}
          aria-pressed={lang === 'es'}
          aria-label={t('lang.es')}
          onClick={() => setLang('es')}
        >
          ES
        </button>
      </div>
    </header>
  );
}
