import { useRef, useState } from 'react';
import { useI18n } from '../lib/i18n';

interface DropZoneProps {
  disabled: boolean;
  onFile: (file: File) => void;
}

export function DropZone({ disabled, onFile }: DropZoneProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  const openPicker = () => {
    if (!disabled) inputRef.current?.click();
  };

  return (
    <div
      role="button"
      tabIndex={disabled ? -1 : 0}
      aria-label={t('home.dropAria')}
      aria-disabled={disabled}
      className={dragging ? 'dropzone dragging' : 'dropzone'}
      onClick={openPicker}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          openPicker();
        }
      }}
      onDragOver={(event) => {
        event.preventDefault();
        if (!disabled) setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(event) => {
        event.preventDefault();
        setDragging(false);
        if (disabled) return;
        const file = event.dataTransfer.files[0];
        if (file) onFile(file);
      }}
    >
      <svg viewBox="0 0 24 24" width="36" height="36" aria-hidden="true" focusable="false">
        <path
          d="M12 16V4m0 0 4 4m-4-4L8 8M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"
          stroke="currentColor"
          strokeWidth="1.6"
          fill="none"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <p className="dropzone-title">{t('home.dropTitle')}</p>
      <p className="dropzone-hint">{t('home.dropHint')}</p>
      <p className="dropzone-hint">{t('home.maxSize')}</p>
      <input
        ref={inputRef}
        data-testid="file-input"
        type="file"
        accept=".csv,.tsv,.txt,.xlsx"
        className="visually-hidden"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file) onFile(file);
          event.target.value = '';
        }}
      />
    </div>
  );
}
