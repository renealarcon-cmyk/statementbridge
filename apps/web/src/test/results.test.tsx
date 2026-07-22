import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';
import App from '../App';
import * as engine from '../lib/engine';
import type { Statement } from '../lib/types';
import {
  engineInfoFixture,
  inspectFixture,
  invalidStatementFixture,
  statementFixture,
} from './fixtures';

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
let anchorClick: MockInstance<() => void>;

beforeEach(() => {
  mocked.initEngine.mockResolvedValue(engineInfoFixture);
  mocked.inspect.mockResolvedValue(inspectFixture());
  mocked.convert.mockResolvedValue(statementFixture());
  mocked.toCsv.mockResolvedValue('id,date,description\n');
  // jsdom would try to "navigate" on anchor clicks; stub it out and observe.
  anchorClick = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
});

afterEach(() => {
  anchorClick.mockRestore();
});

async function openResults(statement?: Statement) {
  if (statement) mocked.convert.mockResolvedValue(statement);
  render(<App />);
  const file = new File(['Fecha;Concepto\n'], 'semicolon_comma_es.csv', { type: 'text/csv' });
  fireEvent.change(screen.getByTestId('file-input'), { target: { files: [file] } });
  await screen.findByRole('heading', { name: 'Review detection' });
  await userEvent.click(screen.getByRole('button', { name: 'Normalize' }));
  await screen.findByRole('heading', { name: 'Results' });
}

describe('results screen', () => {
  it('renders summary cards with duplicate and balance figures', async () => {
    await openResults();
    expect(screen.getByTestId('card-transactions')).toHaveTextContent('4');
    expect(screen.getByTestId('card-debits')).toHaveTextContent('298.99');
    expect(screen.getByTestId('card-credits')).toHaveTextContent('2375.50');
    expect(screen.getByTestId('card-period')).toHaveTextContent('2026-01-02 → 2026-01-12');
    expect(screen.getByTestId('card-duplicates')).toHaveTextContent('1 exact · 2 possible');
    expect(screen.getByTestId('card-balance')).toHaveTextContent('Inconsistent');
    expect(
      screen.getByText('First mismatch at row 6: expected 2125.51, got 2130.00'),
    ).toBeInTheDocument();
    // amber status banner with text, and a duplicate chip in the table
    expect(screen.getByTestId('status-banner')).toHaveTextContent('Valid with warnings');
    expect(screen.getByText('possible duplicate')).toBeInTheDocument();
  });

  it('renders a red status and the error list for an invalid statement', async () => {
    await openResults(invalidStatementFixture());
    const banner = screen.getByTestId('status-banner');
    expect(banner).toHaveTextContent('Invalid');
    expect(banner.className).toContain('banner-invalid');
    expect(screen.getByText('invalid_amount')).toBeInTheDocument();
    expect(screen.getByText("amount 'abc' could not be parsed")).toBeInTheDocument();
  });

  it('downloads JSON through a blob URL and an anchor click', async () => {
    await openResults();
    await userEvent.click(screen.getByRole('button', { name: 'Download JSON' }));

    const createObjectURL = vi.mocked(URL.createObjectURL);
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    const blob = createObjectURL.mock.calls[0]?.[0] as Blob;
    expect(blob.type).toBe('application/json');
    expect(anchorClick).toHaveBeenCalledTimes(1);
  });

  it('downloads CSV via engine.toCsv', async () => {
    await openResults();
    await userEvent.click(screen.getByRole('button', { name: 'Download CSV' }));
    await waitFor(() => expect(mocked.toCsv).toHaveBeenCalledTimes(1));
    expect(mocked.toCsv).toHaveBeenCalledWith(expect.objectContaining({ schema_version: '1.0' }));
    await waitFor(() => expect(anchorClick).toHaveBeenCalledTimes(1));
  });

  it('start over returns home and revokes blob URLs', async () => {
    await openResults();
    await userEvent.click(screen.getByRole('button', { name: 'Download JSON' }));
    await userEvent.click(screen.getByRole('button', { name: 'Start over' }));

    expect(await screen.findByText('Your file never leaves this device')).toBeInTheDocument();
    expect(vi.mocked(URL.revokeObjectURL)).toHaveBeenCalledWith('blob:mock-url');
  });
});
