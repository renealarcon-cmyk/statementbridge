# Security Policy

StatementBridge parses untrusted financial files locally. Bugs in that
parsing, or leaks of the data being processed, are security issues — we take
them seriously and appreciate responsible reports.

The project is alpha software maintained by volunteers. We respond on a
best-effort basis; there is no bounty program and no guaranteed response
time.

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes — security fixes land in the latest 0.1.x release |
| < 0.1.0 | No |

## Reporting a vulnerability

**Do not report vulnerabilities in public issues, discussions, or pull
requests.**

Report privately through GitHub's private vulnerability reporting on this
repository: **Security** tab → **Report a vulnerability**. That opens a
private advisory that only you and the maintainers can see.

Please include:

- The version or commit affected (`statementbridge --version`).
- Platform (OS, and whether CLI, library, or web app).
- Steps to reproduce, ideally with a proof-of-concept input file.
- **The proof-of-concept file must be synthetic.** Never attach a real bank
  statement, real account numbers, or customer data — craft a minimal
  synthetic file that triggers the issue.
- Your assessment of the impact.

What to expect:

- Best-effort acknowledgement and triage — typically within days, but not
  guaranteed.
- We prefer **coordinated disclosure**: please give us a reasonable window to
  ship a fix before publishing details. We will credit reporters in the
  advisory and changelog unless you prefer otherwise.
- No monetary rewards; there is no bug bounty program.

## Scope

### In scope

- **Parsing untrusted CSV/XLSX input** — explicitly in scope. Panics,
  crashes, hangs, or memory exhaustion triggered by crafted files
  (malformed XLSX archives, pathological CSV, oversized fields, etc.),
  including anything that bypasses the documented file-size/row/field
  limits.
- **CSV/formula injection** — exported CSV that would execute or leak data
  when opened in a spreadsheet application despite the built-in guard.
- **Resource exhaustion** — inputs whose processing cost is wildly
  disproportionate to their size.
- **Data leakage in logs or error messages** — raw descriptions, references,
  account numbers, or amounts appearing where the documentation says they
  must not (logs, human CLI output that promises masking, error text).
- **Local-first guarantee violations** — any code path in the CLI or web app
  that transmits statement contents over the network, or persists them
  contrary to the documented behavior.
- **Path handling** — the CLI writing outside the locations the user asked
  for (path traversal via crafted file or adapter names).

### Out of scope

- Vulnerabilities that only exist in a user's own modified build of
  StatementBridge.
- Social engineering, phishing, or physical attacks.
- Vulnerabilities purely in third-party dependencies with no
  StatementBridge-specific exploit path — please report those upstream, but
  feel free to open a regular issue here so we can update the dependency.

StatementBridge runs entirely on the user's machine and the project operates
no servers, so there is no hosted infrastructure in scope.

## Threat model

The full threat model — what we defend against, what we assume, and known
limitations — is documented in [docs/threat-model.md](docs/threat-model.md).
