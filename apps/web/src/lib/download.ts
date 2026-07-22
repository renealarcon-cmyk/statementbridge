/**
 * Blob-URL download helpers. Created URLs are tracked so "Start over" can
 * revoke anything still alive; each URL is also revoked shortly after the
 * programmatic click (a short grace period lets the browser start the
 * download before the URL disappears).
 */
const liveUrls = new Set<string>();

export function downloadText(content: string, mimeType: string, fileName: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  liveUrls.add(url);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => {
    if (liveUrls.delete(url)) URL.revokeObjectURL(url);
  }, 1000);
}

export function revokeAllDownloads(): void {
  for (const url of liveUrls) URL.revokeObjectURL(url);
  liveUrls.clear();
}
