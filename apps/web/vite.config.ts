import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';

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

/**
 * Single-file demo build (scripts/build-demo.sh, VITE_EMBED_WASM=1): the app
 * — JS, CSS and the wasm engine (base64) — is inlined into one HTML document
 * that works when opened via file://. Inlined scripts/styles require an
 * adjusted CSP; connect-src stays 'none' so the no-network guarantee is even
 * visible in the policy itself.
 */
function demoCsp(): Plugin {
  return {
    name: 'statementbridge-demo-csp',
    apply: 'build',
    transformIndexHtml(html) {
      return html.replace(
        /<meta[^>]*http-equiv="Content-Security-Policy"[\s\S]*?\/>/,
        '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; ' +
          "script-src 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'unsafe-inline'; " +
          "img-src data:; connect-src 'none'; object-src 'none'; base-uri 'none'; " +
          "form-action 'none'\" />",
      );
    },
  };
}

const singleFileDemo = process.env.VITE_EMBED_WASM === '1';

export default defineConfig({
  plugins: [
    react(),
    devCsp(),
    ...(singleFileDemo ? [demoCsp(), viteSingleFile({ removeViteModuleLoader: true })] : []),
  ],
  build: {
    target: 'es2022',
    ...(singleFileDemo
      ? { outDir: 'dist-demo', rollupOptions: { output: { inlineDynamicImports: true } } }
      : {}),
  },
});
