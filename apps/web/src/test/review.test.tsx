import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';
import * as engine from '../lib/engine';
import { engineInfoFixture, inspectFixture, statementFixture } from './fixtures';

vi.mock('../lib/engine', () => ({
  EngineFailure: class EngineFailure extends Error {
    code: string;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
  },
  initEngine: vi.fn(),
  inspect: vi.fn(),
  convert: vi.fn(),
  toCsv: vi.fn(),
}));

const mocked = vi.mocked(engine);

beforeEach(() => {
  mocked.initEngine.mockResolvedValue(engineInfoFixture);
  mocked.inspect.mockResolvedValue(inspectFixture());
  mocked.convert.mockResolvedValue(statementFixture());
  mocked.toCsv.mockResolvedValue('id,date\n');
});

async function openReview() {
  render(<App />);
  const file = new File(['Fecha;Concepto\n'], 'semicolon_comma_es.csv', { type: 'text/csv' });
  fireEvent.change(screen.getByTestId('file-input'), { target: { files: [file] } });
  await screen.findByRole('heading', { name: 'Review detection' });
}

describe('review screen', () => {
  it('renders the detection summary and prefilled mapping selects', async () => {
    await openReview();
    expect(screen.getByTestId('detection-adapter')).toHaveTextContent('generic-debit-credit');
    expect(screen.getByTestId('detection-delimiter')).toHaveTextContent(';');
    expect(screen.getByTestId('mapping-select-1')).toHaveValue('date');
    expect(screen.getByTestId('mapping-select-2')).toHaveValue('description');
    expect(screen.getByTestId('mapping-select-3')).toHaveValue('debit');
    expect(screen.getByTestId('mapping-select-5')).toHaveValue('balance');
    // sample preview
    expect(screen.getByText('Depósito inicial')).toBeInTheDocument();
  });

  it('re-runs inspection with the updated mapping when a select changes', async () => {
    await openReview();
    expect(mocked.inspect).toHaveBeenCalledTimes(1);

    await userEvent.selectOptions(screen.getByTestId('mapping-select-2'), 'reference');
    await waitFor(() => expect(mocked.inspect).toHaveBeenCalledTimes(2));

    const lastCall = mocked.inspect.mock.calls.at(-1);
    expect(lastCall?.[2].mapping).toEqual({
      date: '#1',
      reference: '#2',
      debit: '#3',
      credit: '#4',
      balance: '#5',
    });
  });

  it('re-runs inspection when a setting changes (delimiter)', async () => {
    await openReview();
    await userEvent.selectOptions(screen.getByLabelText('Delimiter'), 'tab');
    await waitFor(() => expect(mocked.inspect).toHaveBeenCalledTimes(2));
    expect(mocked.inspect.mock.calls.at(-1)?.[2].delimiter).toBe('tab');
  });

  it('blocks Normalize while a no_date_column error persists', async () => {
    mocked.inspect.mockResolvedValue(
      inspectFixture({
        errors: [{ code: 'no_date_column', message: 'no date column was detected' }],
        columns: [
          { index: 1, header: 'Texto', field: null },
          { index: 2, header: 'Importe', field: 'amount' },
        ],
      }),
    );
    await openReview();
    expect(screen.getByRole('button', { name: 'Normalize' })).toBeDisabled();
    expect(screen.getByTestId('review-blocked')).toBeInTheDocument();
  });
});
