#!/usr/bin/env python3
"""Deterministic generator for StatementBridge test fixtures.

Every fixture is fully synthetic: invented business names, invented account
numbers, no real bank layouts copied verbatim. Regenerating always produces
byte-identical files (fixed zip timestamps, no randomness, no clock reads).

Usage:
    python3 scripts/generate_fixtures.py            # (re)write fixtures/csv + fixtures/xlsx
    python3 scripts/generate_fixtures.py --large N  # also write fixtures/generated/large_N.csv
"""

from __future__ import annotations

import argparse
import io
import zipfile
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CSV_DIR = ROOT / "fixtures" / "csv"
XLSX_DIR = ROOT / "fixtures" / "xlsx"
GEN_DIR = ROOT / "fixtures" / "generated"


def w(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    print(f"wrote {path.relative_to(ROOT)} ({len(data)} bytes)")


# ---------------------------------------------------------------------------
# CSV fixtures
# ---------------------------------------------------------------------------

def csv_fixtures() -> None:
    # 1/7/10/14: comma delimiter, dot decimal, single signed amount, ISO dates,
    # consistent balance.
    w(CSV_DIR / "comma_dot_signed_iso.csv", (
        "Date,Description,Reference,Amount,Balance,Currency\n"
        "2026-01-02,Opening deposit,TRX-1001,1500.00,1500.00,USD\n"
        "2026-01-05,Office supplies — Acme Tools,TRX-1002,-249.99,1250.01,USD\n"
        "2026-01-09,\"Consulting invoice, project Delta\",TRX-1003,875.50,2125.51,USD\n"
        "2026-01-12,Software subscription,TRX-1004,-49.00,2076.51,USD\n"
        "2026-01-15,Utility payment,TRX-1005,-120.75,1955.76,USD\n"
    ).encode("utf-8"))

    # 2/8/16/17: semicolon delimiter, comma decimal with dot thousands,
    # DD/MM/YYYY, debit/credit columns, preamble rows and a footer total row.
    w(CSV_DIR / "semicolon_comma_es.csv", (
        "Banco Sintético S.A.;;;;\n"
        "Movimientos de la cuenta ****4821;;;;\n"
        "Fecha;Concepto;Cargo;Abono;Saldo\n"
        "02/01/2026;Depósito inicial;;1.500,00;1.500,00\n"
        "05/01/2026;Pago proveedor Ferretería Central;249,99;;1.250,01\n"
        "09/01/2026;Factura consultoría proyecto Delta;;875,50;2.125,51\n"
        "12/01/2026;Suscripción software;49,00;;2.076,51\n"
        "TOTAL;;298,99;2.375,50;\n"
    ).encode("utf-8"))

    # 3: tab delimiter.
    w(CSV_DIR / "tab_delimited.tsv", (
        "Date\tDescription\tAmount\tBalance\n"
        "2026-03-01\tOpening deposit\t900.00\t900.00\n"
        "2026-03-02\tHardware purchase\t-129.99\t770.01\n"
        "2026-03-03\tClient payment\t450.00\t1220.01\n"
    ).encode("utf-8"))

    # 3b: pipe delimiter.
    w(CSV_DIR / "pipe_delimited.csv", (
        "Date|Description|Amount\n"
        "2026-03-01|Opening deposit|900.00\n"
        "2026-03-02|Hardware purchase|-129.99\n"
    ).encode("utf-8"))

    # 4: UTF-8 with BOM.
    w(CSV_DIR / "utf8_bom.csv", b"\xef\xbb\xbf" + (
        "Fecha,Concepto,Importe\n"
        "01/02/2026,Café y almuerzo de equipo,-45.90\n"
        "02/02/2026,Ingreso por diseño gráfico,300.00\n"
    ).encode("utf-8"))

    # 5: Windows-1252 (é, ó and the € sign, which is a C1 byte in cp1252).
    w(CSV_DIR / "windows1252.csv", (
        "Fecha;Concepto;Importe\n"
        "01/02/2026;Café La Terraza — menú del día;-12,50\n"
        "02/02/2026;Depósito facturación €;250,00\n"
    ).encode("cp1252"))

    # 6/9: English debit/credit columns with MM/DD/YYYY dates.
    w(CSV_DIR / "debit_credit_mdy.csv", (
        "Date,Description,Reference,Debit,Credit,Balance\n"
        "01/15/2026,Opening deposit,DEP-01,,2000.00,2000.00\n"
        "01/16/2026,Vendor payment — Blue Widgets,CHK-1101,350.25,,1649.75\n"
        "01/20/2026,Customer payment,ACH-2201,,1200.00,2849.75\n"
        "01/28/2026,Payroll run,PAY-3301,1800.00,,1049.75\n"
    ).encode("utf-8"))

    # 11: descending order (newest first) with balances.
    w(CSV_DIR / "descending.csv", (
        "Date,Description,Amount,Balance\n"
        "2026-04-04,Client payment,450.00,1220.01\n"
        "2026-04-03,Hardware purchase,-129.99,770.01\n"
        "2026-04-01,Opening deposit,900.00,900.00\n"
    ).encode("utf-8"))

    # 12: exact duplicates (row repeated byte-identically, no balance column).
    w(CSV_DIR / "exact_duplicates.csv", (
        "Date,Description,Reference,Amount\n"
        "2026-05-02,Subscription renewal,SUB-77,-19.99\n"
        "2026-05-02,Subscription renewal,SUB-77,-19.99\n"
        "2026-05-03,Client payment,INV-201,600.00\n"
    ).encode("utf-8"))

    # 13: similar rows that are NOT duplicates (distinct references).
    w(CSV_DIR / "similar_not_duplicates.csv", (
        "Date,Description,Reference,Amount\n"
        "2026-05-10,Coffee shop,POS-9001,-4.50\n"
        "2026-05-10,Coffee shop,POS-9002,-4.50\n"
        "2026-05-10,Coffee shop,,-4.50\n"
        "2026-05-10,Coffee shop,,-4.50\n"
    ).encode("utf-8"))

    # 15: inconsistent balance chain (second row balance is wrong).
    w(CSV_DIR / "balance_inconsistent.csv", (
        "Date,Description,Amount,Balance\n"
        "2026-06-01,Opening deposit,1000.00,1000.00\n"
        "2026-06-02,Vendor payment,-200.00,850.00\n"
        "2026-06-03,Client payment,300.00,1150.00\n"
    ).encode("utf-8"))

    # 18: missing values (empty amount, empty description).
    w(CSV_DIR / "missing_values.csv", (
        "Date,Description,Amount,Balance\n"
        "2026-07-01,Opening deposit,500.00,500.00\n"
        "2026-07-02,,,\n"
        "2026-07-03,,-25.00,475.00\n"
    ).encode("utf-8"))

    # 19: invalid amounts.
    w(CSV_DIR / "invalid_amounts.csv", (
        "Date,Description,Amount\n"
        "2026-07-01,Valid row,10.00\n"
        "2026-07-02,Broken row,12abc\n"
        "2026-07-03,Also valid,-5.25\n"
    ).encode("utf-8"))

    # Extra: headerless file (needs --map with #indexes).
    w(CSV_DIR / "headerless.csv", (
        "2026-08-01,Opening deposit,700.00\n"
        "2026-08-02,Vendor payment,-100.00\n"
    ).encode("utf-8"))

    # Extra: CSV-injection attempt in descriptions (must be escaped on export).
    w(CSV_DIR / "injection_attempt.csv", (
        "Date,Description,Amount\n"
        "2026-09-01,=HYPERLINK(\"http://evil.example\"),-1.00\n"
        "2026-09-02,+cmd|' /C calc'!A0,2.00\n"
        "2026-09-03,@SUM(1+9),3.00\n"
    ).encode("utf-8"))


# ---------------------------------------------------------------------------
# XLSX fixtures (hand-built minimal OOXML so formula cells can carry cached
# values — openpyxl cannot write those).
# ---------------------------------------------------------------------------

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
{overrides}
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>
"""

ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
"""

STYLES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="1"><fill><patternFill patternType="none"/></fill></fills>
<borders count="1"><border/></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="2">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="14" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
</cellXfs>
</styleSheet>
"""


def esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def cell(ref: str, value, kind: str = "auto", style: int | None = None, formula: str | None = None) -> str:
    s_attr = f' s="{style}"' if style is not None else ""
    if formula is not None:
        v = f"<f>{esc(formula)}</f>" + (f"<v>{value}</v>" if value is not None else "")
        return f'<c r="{ref}"{s_attr}>{v}</c>'
    if kind == "auto":
        kind = "n" if isinstance(value, (int, float)) else "s"
    if kind == "n":
        return f'<c r="{ref}"{s_attr}><v>{value}</v></c>'
    return f'<c r="{ref}"{s_attr} t="inlineStr"><is><t xml:space="preserve">{esc(str(value))}</t></is></c>'


def sheet_xml(rows: list[list[str]]) -> str:
    body = "".join(f'<row r="{i + 1}">{"".join(cells)}</row>' for i, cells in enumerate(rows))
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f"<sheetData>{body}</sheetData></worksheet>"
    )


def excel_serial(d: date) -> int:
    return (d - date(1899, 12, 30)).days


def write_xlsx(path: Path, sheets: list[tuple[str, str]]) -> None:
    overrides = "\n".join(
        f'<Override PartName="/xl/worksheets/sheet{i + 1}.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
        for i in range(len(sheets))
    )
    sheets_decl = "".join(
        f'<sheet name="{esc(name)}" sheetId="{i + 1}" r:id="rId{i + 1}"/>' for i, (name, _) in enumerate(sheets)
    )
    workbook = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        f"<sheets>{sheets_decl}</sheets></workbook>"
    )
    wb_rels = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + "".join(
            f'<Relationship Id="rId{i + 1}" '
            'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" '
            f'Target="worksheets/sheet{i + 1}.xml"/>'
            for i in range(len(sheets))
        )
        + f'<Relationship Id="rId{len(sheets) + 1}" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" '
        'Target="styles.xml"/></Relationships>'
    )
    buf = io.BytesIO()
    stamp = (2026, 1, 1, 0, 0, 0)  # fixed timestamp: byte-identical output
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        def add(name: str, data: str) -> None:
            info = zipfile.ZipInfo(name, date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)

        add("[Content_Types].xml", CONTENT_TYPES.format(overrides=overrides))
        add("_rels/.rels", ROOT_RELS)
        add("xl/workbook.xml", workbook)
        add("xl/_rels/workbook.xml.rels", wb_rels)
        add("xl/styles.xml", STYLES)
        for i, (_, xml) in enumerate(sheets):
            add(f"xl/worksheets/sheet{i + 1}.xml", xml)
    w(path, buf.getvalue())


def xlsx_fixtures() -> None:
    # 20: multiple sheets — an info sheet and a denser transactions sheet with
    # real date serials and numeric cells.
    info = sheet_xml([
        [cell("A1", "Synthetic Bank Export"), ],
        [cell("A2", "Account"), cell("B2", "****4821")],
    ])
    d0 = date(2026, 2, 2)
    tx_rows = [[
        cell("A1", "Fecha"), cell("B1", "Concepto"), cell("C1", "Cargo"),
        cell("D1", "Abono"), cell("E1", "Saldo"),
    ]]
    running = 0.0
    amounts = [(None, 1500.00, "Depósito inicial"), (250.00, None, "Pago proveedor"),
               (99.50, None, "Material de oficina"), (None, 420.25, "Cobro factura 1088")]
    for i, (debit, credit, concept) in enumerate(amounts):
        running += (credit or 0.0) - (debit or 0.0)
        r = i + 2
        tx_rows.append([
            cell(f"A{r}", excel_serial(d0 + timedelta(days=i)), kind="n", style=1),
            cell(f"B{r}", concept),
            cell(f"C{r}", debit, kind="n") if debit is not None else cell(f"C{r}", ""),
            cell(f"D{r}", credit, kind="n") if credit is not None else cell(f"D{r}", ""),
            cell(f"E{r}", round(running, 2), kind="n"),
        ])
    write_xlsx(XLSX_DIR / "multisheet.xlsx", [("Información", info), ("Movimientos", sheet_xml(tx_rows))])

    # 21: formula cells. B4 has a cached value (600), B5 has a formula with no
    # cached value (reads as empty and must produce a per-row report).
    formulas = sheet_xml([
        [cell("A1", "Date"), cell("B1", "Description"), cell("C1", "Amount")],
        [cell("A2", "2026-03-01"), cell("B2", "Base fee"), cell("C2", 100.00, kind="n")],
        [cell("A3", "2026-03-02"), cell("B3", "Usage fee"), cell("C3", 500.00, kind="n")],
        [cell("A4", "2026-03-03"), cell("B4", "Total re-billed (cached formula)"),
         cell("C4", 600.00, formula="SUM(C2:C3)")],
        [cell("A5", "2026-03-04"), cell("B5", "Uncached formula"),
         cell("C5", None, formula="C4*0")],
    ])
    write_xlsx(XLSX_DIR / "formulas.xlsx", [("Sheet1", formulas)])


# ---------------------------------------------------------------------------
# Large synthetic file for benchmarks (never committed).
# ---------------------------------------------------------------------------

def large(n: int) -> None:
    GEN_DIR.mkdir(parents=True, exist_ok=True)
    path = GEN_DIR / f"large_{n}.csv"
    concepts = [
        "Vendor payment — Blue Widgets", "Client payment", "Payroll run",
        "Software subscription", "Utility payment", "Office supplies",
        "Bank fee", "Refund issued", "Interest earned", "Equipment lease",
    ]
    start = date(2024, 1, 1)
    balance = 100_000_00  # cents
    with path.open("w", encoding="utf-8", newline="\n") as f:
        f.write("Date,Description,Reference,Amount,Balance\n")
        for i in range(n):
            d = start + timedelta(days=i // 50)
            # Deterministic pseudo-amounts (no RNG).
            cents = ((i * 7349) % 250_000) - 100_000
            if cents == 0:
                cents = 1500
            balance += cents
            f.write(
                f"{d.isoformat()},{concepts[i % len(concepts)]},REF-{100000 + i},"
                f"{cents / 100:.2f},{balance / 100:.2f}\n"
            )
    print(f"wrote {path.relative_to(ROOT)}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--large", type=int, metavar="N", help="also generate fixtures/generated/large_N.csv")
    args = ap.parse_args()
    csv_fixtures()
    xlsx_fixtures()
    if args.large:
        large(args.large)


if __name__ == "__main__":
    main()
