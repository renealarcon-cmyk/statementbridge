import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';

/**
 * The production CSP in index.html is strict: no inline scripts, no inline
 * styles, no websockets. Vite's dev server needs all three (React refresh
 * preamble, HMR-injected styles, HMR websocket), so during `vite dev` only we
 * swap the meta tag for a relaxed variant. Production builds ship the strict
 * policy from index.html verbatim.
 */
function devCsp(): Plugin {
  return {
    name: 'statementbridge-dev-csp',
    apply: 'serve',
    transformIndexHtml(html) {
      return html.replace(
        /<meta[^>]*http-equiv="Content-Security-Policy"[\s\S]*?\/>/,
        '<meta http-equiv="Content-Security-Policy" content="default-src \'self\'; ' +
          "script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; " +
          "img-src 'self' data:; connect-src 'self' ws: wss:; object-src 'none'; " +
          "base-uri 'none'; form-action 'none'\" />",
      );
    },
  };
}

export default defineConfig({
  plugins: [react(), devCsp()],
  build: { target: 'es2022' },
});
