import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// e2e runs against the production build with the REAL wasm engine (no mocks).
const here = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.resolve(here, '../../../fixtures');

test('full CSV flow: import, review, normalize, download JSON, start over', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText('Your file never leaves this device')).toBeVisible();

  await page
    .getByTestId('file-input')
    .setInputFiles(path.join(fixturesDir, 'csv', 'semicolon_comma_es.csv'));

  // Review: detection results from the real engine.
  await expect(page.getByRole('heading', { name: 'Review detection' })).toBeVisible();
  await expect(page.getByTestId('detection-adapter')).toHaveText('generic-debit-credit');
  await expect(page.getByTestId('detection-delimiter')).toContainText(';');
  await expect(page.getByTestId('mapping-select-1')).toHaveValue('date');

  await page.getByRole('button', { name: 'Normalize' }).click();

  // Results
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  await expect(page.getByTestId('status-banner')).toContainText('Valid with warnings');
  await expect(page.getByTestId('card-transactions')).toHaveText('4');
  await expect(page.getByTestId('card-balance')).toContainText('Consistent');

  // Download JSON and validate the payload.
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download JSON' }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe('semicolon_comma_es.normalized.json');
  const filePath = await download.path();
  const payload = JSON.parse(fs.readFileSync(filePath, 'utf8')) as {
    schema_version: string;
    transactions: unknown[];
  };
  expect(payload.schema_version).toBe('1.0');
  expect(payload.transactions).toHaveLength(4);

  // Start over clears everything and returns home.
  await page.getByRole('button', { name: 'Start over' }).click();
  await expect(page.getByText('Your file never leaves this device')).toBeVisible();
});

test('XLSX flow: multisheet workbook picks the transactions sheet', async ({ page }) => {
  await page.goto('/');
  await page
    .getByTestId('file-input')
    .setInputFiles(path.join(fixturesDir, 'xlsx', 'multisheet.xlsx'));

  await expect(page.getByRole('heading', { name: 'Review detection' })).toBeVisible();
  await expect(page.getByTestId('detection-sheet')).toHaveText('Movimientos');
  await expect(page.getByLabel('Sheet')).toHaveValue('Movimientos');

  await page.getByRole('button', { name: 'Normalize' }).click();
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  await expect(page.getByTestId('card-transactions')).toHaveText('4');
});
