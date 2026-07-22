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

describe('home screen', () => {
  it('renders the privacy line in English and toggles to Spanish', async () => {
    render(<App />);
    expect(screen.getByText('Your file never leaves this device')).toBeInTheDocument();
    expect(document.documentElement.lang).toBe('en');

    await userEvent.click(screen.getByRole('button', { name: 'Spanish' }));
    expect(screen.getByText('Tu archivo nunca sale de este dispositivo')).toBeInTheDocument();
    expect(screen.getByText('Cómo funciona')).toBeInTheDocument();
    expect(document.documentElement.lang).toBe('es');
  });

  it('rejects files over 50 MiB before reading them, with a translated error', async () => {
    render(<App />);
    await waitFor(() => expect(mocked.initEngine).toHaveBeenCalled());

    const file = new File(['x'], 'big.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'size', { value: 50 * 1024 * 1024 + 1 });
    fireEvent.change(screen.getByTestId('file-input'), { target: { files: [file] } });

    const error = await screen.findByTestId('home-error');
    expect(error).toHaveTextContent('The maximum supported size is 50 MiB');
    expect(mocked.inspect).not.toHaveBeenCalled();
  });

  it('shows the engine version in the footer once the engine is ready', async () => {
    render(<App />);
    expect(await screen.findByText('Engine v0.1.0')).toBeInTheDocument();
    expect(screen.getByText('Not financial advice')).toBeInTheDocument();
    expect(screen.getByText('Open source · Apache-2.0')).toBeInTheDocument();
  });
});
