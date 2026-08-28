# Run Proof handoff

Work order: `job-liveness-proof-build-1`  
Completed: 2026-08-28

## What was built

- Production Rust/axum receiver backed by SQLite migrations, structured JSON logging, graceful shutdown, `/health` build metadata, secure response headers, compression, static caching, and a 100 requests/second-per-key ingest limiter.
- HMAC-SHA256 signed endpoints to register schedules, record start/finish evidence, and import CI/check snapshots. Requests have strict schemas, duplicate protection, URL and identifier validation, and configurable clock-skew handling. No job payload field is accepted or retained.
- Read-time ledger derivation for running, completed, failed, late, missed, and contradictory signals. Both observed and derived alerts export portable JSON receipts with SHA-256 hashes; the complete ledger exports as CSV.
- `run-proof` CLI with `register`, `start`, `finish`, and `snapshot` commands.
- Responsive TypeScript UI with paper-cut diorama identity, empty/loading/error/offline states, filters, offline last-known export, mobile receipt cards, and `/privacy` and `/terms`.
- One-time $29 Run Proof Plus integration through Sociobot: return-token capture, URL cleanup, once-daily verification, optimistic offline unlock, restore form, revoked-license messaging, and a saved operational view. Core evidence, retention configuration, and exports remain free.
- Original generated hero artwork in optimized 20 KB/59 KB WebP sizes, self-hosted fonts, PWA shell cache, and documented provenance.
- Multi-stage, non-root Dockerfile containing both receiver and CLI; persistent SQLite lives at `/data`.

## Run and deploy

```sh
npm ci
npm test
npm run build
RUN_PROOF_SECRET='at-least-32-random-characters' \
  DATABASE_URL='sqlite://run-proof.db?mode=rwc' \
  cargo run --bin run-proof-server
```

Exact container build command: `docker build -t run-proof .`. Frontend output is `dist/`; the container embeds it at `/app/dist` and serves API plus SPA on `PORT` (default 8080).

## Verification performed

- `npm test`: passed — 2 Vitest tests, 3 Rust API integration tests, production build, and 5 applicable Playwright checks across desktop and 390×844 mobile Chromium.
- Full signed job → start → finish → contradictory CI snapshot → receipt flow and derived missed receipt: passed.
- `npm run check`: strict TypeScript and Clippy with warnings denied passed.
- `npm audit`: 0 vulnerabilities.
- `cargo build --locked --release --bin run-proof-server --bin run-proof`: passed.
- `scripts/load-smoke.sh`: 100/100 concurrent health requests completed.
- Axe 4.13: 0 serious/critical WCAG 2 A/AA findings on home and terms in light and dark treatments, desktop and mobile.
- Playwright: title, one `<h1>`, main landmark, keyboard skip link, legal routing, no console errors, and no 390 px overflow passed.
- Security smoke: CSP, `nosniff`, no-referrer, no-cache shell, and immutable asset headers observed.
- Lighthouse 12.8.2 mobile: Performance **100**, Accessibility **100**, Best Practices **100**, SEO **100**; LCP **1.4 s**, FCP **1.0 s**, TBT **10 ms**, CLS **0**, Speed Index **1.0 s**.
- Budgets: initial JS 17.36 KB raw / 6.88 KB gzip; CSS 17.76 KB raw / 5.12 KB gzip; Latin fonts 68.07 KB; mobile hero 20 KB.

## Known gaps and next steps

- The factory must register `job-liveness-proof` with Sociobot before live purchases verify. No provider product ID is hardcoded.
- This worker image had no Docker CLI, so `docker build` could not run here; source, locked dependencies, release binaries, and the frontend build were validated independently.
- Retention cleanup runs at process startup. Restart after lowering `RETENTION_DAYS` to apply the cutoff immediately.
- Alert routing and multi-user accounts remain intentional non-goals. Next: test checkout after registration, smoke the image with a mounted `/data` volume in CI, and add overlapping key IDs if pilots need zero-downtime secret rotation.
