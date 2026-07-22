# StatementBridge

Normaliza exportaciones de estados de cuenta bancarios (CSV/XLSX) a un formato canónico y validado — completamente en tu máquina.

**Estado: alpha (v0.1.0).** El flujo principal funciona de extremo a extremo y tiene pruebas automatizadas, pero el proyecto es joven: revisa las [limitaciones](#limitaciones) antes de depender de él.

*English version: [README.md](README.md).*

## El problema

Cada banco, procesador de pagos y sistema empresarial exporta las transacciones de forma distinta: nombres de columnas diferentes ("Fecha", "Transaction Date", "F. Valor"), orden de fecha ambiguo (`01/02/2026`: ¿enero o febrero?), separadores decimales distintos (`1.234,56` frente a `1,234.56`), codificaciones diversas (UTF-8, Windows-1252), débito y crédito en dos columnas o en una sola columna con signo, filas informativas antes del encabezado y filas de totales al final. Contadores, despachos y equipos financieros pierden horas limpiando estos archivos a mano — y la limpieza manual introduce errores silenciosos en datos financieros.

## La solución

StatementBridge es una herramienta open source y local-first que lee esas exportaciones CSV y XLSX y produce un único formato canónico y validado:

- Detecta la codificación, el delimitador, el encabezado real, el rol de cada columna, el formato de fecha y el formato numérico — y reporta cada suposición que hizo.
- Normaliza cada transacción a un esquema fijo: fechas ISO, cantidades decimales exactas (nunca floats), convención de signo canónica (débitos negativos, créditos positivos) y procedencia hacia la fila original.
- Valida el resultado: coherencia de la cadena de saldos, detección de duplicados, errores y advertencias por fila con códigos estables legibles por máquina.
- Exporta JSON canónico (validado por un JSON Schema publicado) o CSV normalizado.
- Se usa de tres formas: como librería Rust, como herramienta de línea de comandos y como aplicación web local compilada a WebAssembly.
- Todo es determinista: el mismo archivo con las mismas opciones produce siempre una salida idéntica byte a byte.

## Tus datos no salen de tu equipo

Los estados de cuenta son información sensible. StatementBridge está construido para que nunca abandonen tu máquina:

- El motor central **no realiza acceso a red ni entrada/salida de archivos** — recibe bytes y devuelve datos.
- El CLI solo lee y escribe los archivos que tú indicas. Sin telemetría, sin verificaciones de actualización, sin conexiones externas.
- La aplicación web procesa los archivos **en memoria, dentro de tu navegador**, mediante WebAssembly. No hay backend ni subida de archivos; la interfaz muestra "Your file never leaves this device" porque así está construida.
- Los números de cuenta se enmascaran como `****NNNN` en la salida; los números completos nunca se emiten. Los mensajes de error y los logs jamás citan descripciones, referencias ni números de cuenta.

Detalles: [docs/privacy.md](docs/privacy.md) y [docs/threat-model.md](docs/threat-model.md).

## Funcionalidad (v0.1.0)

- **Entrada**: CSV y XLSX. El formato se detecta por el contenido (bytes mágicos), nunca por la extensión. El `.xls` antiguo se rechaza con un mensaje claro.
- **CSV**: detección automática de delimitador (`,` `;` tabulación `|`); codificaciones UTF-8, UTF-8 con BOM, UTF-16 LE/BE (con BOM), Windows-1252 e ISO-8859-1 (heurística, siempre reportada como advertencia).
- **XLSX**: lectura mediante calamine; solo valores — las macros y los enlaces externos nunca se ejecutan. Selección automática de hoja, con opción de forzarla.
- **Detección**: encuentra el encabezado real aunque haya filas informativas antes; identifica columnas mediante una tabla de alias en inglés y español; omite filas de totales con advertencia; permite mapeo manual cuando la detección no basta.
- **Normalización**: fechas en muchos formatos (día-primero frente a mes-primero resuelto con evidencia), cantidades con decimales exactos, separadores inferidos de forma determinista, limpieza Unicode del texto, moneda solo como ISO 4217 (nunca inventada).
- **Modos de cantidad**: columnas separadas de cargo/abono o una sola columna con signo, detectado automáticamente o forzado.
- **Validación**: verificación de la cadena de saldos con reporte del primer desajuste, detección de orden ascendente/descendente, errores y advertencias por fila con códigos estables.
- **Duplicados**: huellas deterministas distinguen duplicados exactos, posibles duplicados (marcados, nunca eliminados automáticamente) y transacciones legítimamente repetidas. Los duplicados exactos solo se eliminan si tú lo pides.
- **Exportación**: JSON canónico (validado contra el esquema) y CSV normalizado con protección contra inyección de fórmulas.
- **Adaptadores**: documentos declarativos `adapter.json` que describen el formato de exportación de un banco — sin escribir código.
- **Seguridad**: límites de tamaño de archivo, filas, columnas y campos; enmascaramiento de cuentas; redacción opcional de secuencias de dígitos; sanitización de nombres de archivo hostiles.

## Instalación

### Requisitos

- Rust 1.79 o superior (para el CLI y la librería).
- Node.js 20 o superior y [pnpm](https://pnpm.io) (solo para la aplicación web).

Desarrollado y probado en Linux. El código no tiene dependencias específicas de plataforma, pero considera macOS y Windows como menos ejercitados hasta que la matriz de CI lleve un tiempo funcionando.

### CLI (desde el código fuente)

```sh
git clone https://github.com/renealarcon-cmyk/statementbridge
cd statementbridge
cargo install --path crates/statementbridge-cli
# o, sin instalar:
cargo build --release   # binario en target/release/statementbridge
```

### Aplicación web (desarrollo)

```sh
scripts/build-wasm.sh              # compila el motor a WebAssembly
pnpm --dir apps/web install
pnpm --dir apps/web dev            # servidor local; el procesamiento ocurre en el navegador
```

## Ejemplo en sesenta segundos

El repositorio incluye fixtures completamente sintéticos. Este imita una exportación bancaria en español: punto y coma como delimitador, coma decimal, filas informativas antes del encabezado, columnas de cargo/abono y una fila de totales al final.

Primero inspecciona — mira qué detecta el motor antes de convertir nada:

```text
$ statementbridge inspect fixtures/csv/semicolon_comma_es.csv
File:      semicolon_comma_es.csv
Format:    csv · encoding utf-8 · delimiter ';'
Adapter:   generic-debit-credit v1.0.0
Rows:      8 total, 5 data
Header:    row 3
Dates:     %d/%m/%Y
Numbers:   decimal ',', thousands '.'
Columns:
    1  Fecha                        → date
    2  Concepto                     → description
    3  Cargo                        → debit
    4  Abono                        → credit
    5  Saldo                        → balance
Sample (masked):
  02/01/2026 | Depósito inicial |  | 1.500,00 | 1.500,00
  05/01/2026 | Pago proveedor Ferretería Central | 249,99 |  | 1.250,01
  09/01/2026 | Factura consultoría proyecto Delta |  | 875,50 | 2.125,51
  12/01/2026 | Suscripción software | 49,00 |  | 2.076,51
  TOTAL |  | 298,99 | 2.375,50 |
warning [preamble_rows_skipped]: 2 informational row(s) before the header were skipped
```

Convierte a JSON canónico:

```text
$ statementbridge convert fixtures/csv/semicolon_comma_es.csv --output normalized.json
File:         semicolon_comma_es.csv
Adapter:      generic-debit-credit v1.0.0
Status:       valid (with warnings)
Period:       2026-01-02 → 2026-01-12
Transactions: 4
Debits:       298.99
Credits:      2375.50
Balance:      consistent (3 pairs checked)
Duplicates:   0 exact, 0 possible
Errors:       0
Warnings:     2
  warning [preamble_rows_skipped]: 2 informational row(s) before the header were skipped
  warning [footer_row_skipped] row 8: footer/summary row skipped

Wrote normalized.json (3057 bytes)
```

Así queda la primera transacción (`02/01/2026` con `1.500,00` en la columna *Abono* se convirtió en fecha ISO y crédito decimal exacto):

```json
{
  "id": "sb1-7b78fb4a32dfa75a",
  "date": "2026-01-02",
  "posted_at": null,
  "description": "Depósito inicial",
  "reference": null,
  "debit": null,
  "credit": "1500.00",
  "amount": "1500.00",
  "balance": "1500.00",
  "currency": null,
  "original_row": 4,
  "fingerprint": "4a123e9e170bd955508d8866060121c594c55a1bed8fa1b234d469ef0ca86b6f",
  "warnings": []
}
```

## Uso del CLI

```text
statementbridge inspect  <archivo>                 # muestra qué detecta el motor
statementbridge convert  <archivo> [--output f]    # JSON canónico (por defecto) o --format csv
statementbridge validate <archivo>                 # reporta errores/advertencias sin exportar
statementbridge adapters list                      # adaptadores integrados + --adapters-dir
statementbridge adapters validate <adapter.json>
statementbridge init-adapter <nombre>              # crea la plantilla de un adaptador
```

Opciones frecuentes (la lista completa con `statementbridge <comando> --help`):

| Opción | Propósito |
|---|---|
| `--adapter <id-o-ruta>` | Usar un adaptador concreto en lugar de la detección automática |
| `--map date=Fecha --map amount=#4` | Mapeo explícito de columnas (nombre de encabezado o `#N`, base 1) |
| `--date-format %d/%m/%Y` | Fijar el formato de fecha (repetible; se prueban en orden) |
| `--decimal-separator , --thousands-separator .` | Fijar el formato numérico |
| `--currency EUR` | Moneda a asumir cuando el archivo no la trae |
| `--locale en\|es` | Pista para desempatar fechas/números genuinamente ambiguos |
| `--transaction-order auto\|ascending\|descending` | Forzar el orden de las transacciones |
| `--strict` | Tratar las advertencias como fallo (código de salida 4) |
| `--redact` | Enmascarar secuencias largas de dígitos en descripciones/referencias |
| `--remove-exact-duplicates` | Eliminar duplicados exactos (se conserva la primera aparición) |
| `--imported-at 2026-01-01T00:00:00Z` | Timestamp fijo para salida reproducible byte a byte |
| `--json` | Envoltura JSON legible por máquina `{"ok":…,"data":…}` en stdout |
| `--quiet` | Suprimir mensajes no esenciales |

Códigos de salida (contrato estable, pensado para scripts y CI):

| Código | Significado |
|---|---|
| 0 | Éxito (válido, o válido con advertencias sin `--strict`) |
| 1 | Fallo de ejecución (archivo ilegible, adaptador incorrecto, E/S) |
| 2 | Error de uso (flag o argumento desconocido) |
| 3 | Validación fallida (`validation.status == "invalid"`) |
| 4 | Hay advertencias y se pasó `--strict` |

`convert` escribe el documento en stdout y el resumen humano en stderr, de modo que las tuberías (`|`) quedan limpias; `--output archivo` escribe el documento y se niega a sobrescribir el archivo de entrada.

## Adaptadores

Un adaptador es un único archivo JSON declarativo que describe la exportación de un banco o procesador: encabezados esperados, alias adicionales, formatos de fecha, separadores, modo de cantidad, patrones de pie de página. Sin código. Se incluyen tres adaptadores genéricos:

| id | Cubre |
|---|---|
| `generic-csv` | Detección automática completa (la mayoría de archivos) |
| `generic-debit-credit` | Columnas separadas de cargo/abono (debe/haber, withdrawal/deposit) |
| `generic-signed-amount` | Una columna de importe con valores positivos y negativos |

El adaptador adecuado se detecta automáticamente a partir del encabezado; siempre puedes forzar uno con `--adapter`. Para contribuir un adaptador para tu banco:

```sh
statementbridge init-adapter acmebank-checking
# edita adapters/acmebank-checking/adapter.json
statementbridge adapters validate adapters/acmebank-checking/adapter.json
```

La plantilla generada incluye documentación y las reglas de contribución (solo fixtures sintéticos — nunca estados de cuenta reales). Guía completa: [docs/adapter-authoring.md](docs/adapter-authoring.md) (en inglés).

## La aplicación web

`apps/web` contiene una aplicación web local-first (React + TypeScript + Vite) que ejecuta el mismo motor Rust compilado a WebAssembly. El flujo: arrastra un archivo, revisa la detección automática y las primeras filas, ajusta el mapeo de columnas y la configuración de fecha/moneda si hace falta, normaliza, revisa el resumen (transacciones válidas, errores, advertencias, duplicados, diferencias de saldo), examina la tabla de resultados, descarga JSON o CSV y borra el archivo de la memoria con un clic. Interfaz en inglés y español. Sin backend, sin analítica, sin peticiones de red durante el procesamiento y sin persistencia de tus documentos.

## Limitaciones

Lista honesta, vigente para v0.1.0:

- **Software alpha.** El esquema canónico aspira a ser estable, pero las API pueden cambiar entre versiones 0.x (ver [docs/versioning.md](docs/versioning.md)).
- **Sin PDF ni OCR.** Solo CSV y XLSX. El soporte de PDF está previsto en el diseño (un lector futuro en `statementbridge-formats`) pero no implementado. El `.xls` antiguo se rechaza — guárdalo como `.xlsx` o CSV.
- **Sin conexión con bancos.** Sin Open Banking, sin almacenamiento de credenciales, sin scraping. Tú exportas el archivo desde tu banco; StatementBridge lo normaliza.
- **Las heurísticas pueden equivocarse.** La detección de delimitador, codificación, orden de fecha y separadores es determinista y basada en evidencia, pero un archivo genuinamente ambiguo recibe una suposición documentada más una advertencia — revisa las advertencias o fija los formatos con flags o un adaptador.
- **Los números de XLSX pasan por floats una vez.** El propio formato XLSX almacena los números como dobles IEEE-754. Los valores se convierten con formato de ida y vuelta más corto y después se tratan como decimales exactos; es inherente a XLSX y está documentado en [docs/canonical-schema.md](docs/canonical-schema.md).
- **Las celdas de fórmula usan el valor en caché.** Las fórmulas nunca se evalúan; una celda de fórmula sin valor en caché se lee vacía y se reporta.
- **Un estado de cuenta por ejecución.** Sin fusión de varios archivos, sin conciliación con facturas, sin clasificación contable.
- **Aún sin publicar.** Sin paquetes en crates.io/npm hasta v0.2; instala desde el código fuente.
- **No es asesoría financiera.** StatementBridge normaliza datos. No los interpreta, y su salida debe revisarse antes de usarse con fines contables o fiscales.

## Seguridad y privacidad

Analizar archivos no confiables está dentro del alcance: los estados de cuenta provienen de muchas fuentes y pasan por muchas manos. Las defensas incluyen protección contra inyección de fórmulas CSV en la exportación, límites de tamaño/filas/columnas/campos, cero ejecución de macros o enlaces en XLSX, sanitización de nombres de archivo, enmascaramiento de cuentas y mensajes de error que nunca citan el contenido de campos de texto libre. Lee [docs/privacy.md](docs/privacy.md) y [docs/threat-model.md](docs/threat-model.md). Reporta vulnerabilidades de forma privada según [SECURITY.md](SECURITY.md).

## Hoja de ruta

- **v0.1 (actual)**: CSV, XLSX, esquema canónico, adaptadores declarativos, CLI, web local, validación, duplicados, exportación.
- **v0.2**: mejor detección de formatos, API estable de plugins, más adaptadores comunitarios, integración con n8n, paquetes npm y crates.io.
- **v0.3**: PDF con capa de texto, OCR opcional y local, importación de procesadores de pago, reglas de conciliación.
- **v1.0**: API estable, compatibilidad documentada, suite de conformidad para adaptadores, ecosistema de integraciones, política de soporte.

## Contribuir

Las contribuciones son bienvenidas — los adaptadores para nuevos bancos son la forma más fácil de ayudar y no requieren Rust. Empieza por [CONTRIBUTING.md](CONTRIBUTING.md) y [docs/adapter-authoring.md](docs/adapter-authoring.md). Un comando ejecuta todas las pruebas y otro todos los linters:

```sh
scripts/test-all.sh
scripts/lint-all.sh
```

Nunca incluyas datos bancarios reales en issues, fixtures o pull requests. Todos los fixtures deben ser sintéticos.

## Licencia

Apache License 2.0. Ver [LICENSE](LICENSE) y [NOTICE](NOTICE).
