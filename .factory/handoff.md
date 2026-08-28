# Run Proof repair handoff — PASS

- Work order: `job-liveness-proof-repair-2`
- Repaired candidate: `802b646ff3ecc0dbdc48cee35e88400ad13c3452`
- Verifier report: `e2a60b8a3a821de4dc2b7ade54de093fd724690f`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Artifact: `web-with-backend`, Rust/axum + SQLite + Vite/TypeScript, one container on `PORT`

All critical, high, medium, and low findings in `.factory/verification.md` are repaired. Existing brief behavior remains intact.

## Repairs and exact regression coverage

### C1 — one durable ledger and signing identity

- Azure Container Apps now has an Azure Files volume mounted at `/data` and is constrained to exactly one replica (`minReplicas=1`, `maxReplicas=1`), which is the supported topology for this SQLite product.
- The database remains `/data/run-proof.db`. The generated HMAC secret remains `/data/run-proof.secret`, is created atomically with mode `0600`, and is reused on later boots.
- SQLite uses the built-in `unix-dotfile` VFS on `/data` with one pooled connection. This avoids unsupported POSIX byte-range locks on the Azure Files SMB mount while preserving SQLite's lock protocol.
- `durable_mount_uses_a_network_filesystem_safe_vfs` starts a server against a `/data`-style filesystem, writes evidence, restarts it, and proves both the secret and database persist.
- Live proof: register and start were accepted, the active revision was forcibly restarted, startup logged `secret=persisted`, finish was accepted with the same signature key, and the receipt retained both events.

### H1 — receipts cannot cross job boundaries

- Receipt identity and route are now `(job_key, run_id)`: `/api/v1/jobs/:job_key/runs/:run_id/receipt`.
- Every receipt event includes `job_key`; normal and derived missed-run lookups scope all SQL by both fields.
- The UI exports from the job-scoped route.
- `receipts_are_scoped_by_job_when_run_ids_match` registers two jobs with the same run ID and proves each export contains only its own events and values.

### H2 — finish-before-start is renderable

- Missing schedule times serialize as JSON `null`, not an empty date string.
- The frontend checks for a valid finite date before relative-time formatting and renders “Schedule not received”.
- API regression `finish_before_start_is_renderable_and_scoped` and Playwright regression `finish-before-start evidence renders without a page error` cover the persisted response and full browser rendering.

### H3 — exported signatures are independently reproducible

- Receipt format v2 stores and exports each accepted request's signing key ID, Unix timestamp, exact UTF-8 request body, and supplied HMAC signature.
- Registration signatures are persisted and exported too.
- Receipt instructions state the precise signed bytes: `<timestamp>.<exact_body_utf8>`. Derived missed receipts identify their signed source observations and label the receipt hash as an integrity checksum rather than an HMAC.
- `signed_events_create_a_contradictory_receipt` recomputes every exported registration/event HMAC from the receipt fields and proves the receipt checksum.

### Medium and low findings

- **200% resize:** responsive grid/table/search rules now reflow at 390px with a 32px root font. Browser coverage asserts `scrollWidth === clientWidth`.
- **Safe updates:** stable JS/CSS and the shell use `Cache-Control: no-cache`; service worker v3 installs with `cache: 'reload'`; registration uses `updateViaCache: 'none'`. Online-to-offline regression proves the controlled shell and ledger remain usable.
- **Rate limiting:** ingest buckets use the peer IP, not attacker-controlled key headers; inactive entries are evicted and total sources are bounded at 4,096. API coverage sends rotating fake key IDs and proves the shared peer bucket reaches `429`.
- **Unknown API routes:** `/api` and `/api/*` return structured JSON `404`; covered by API and browser response assertions.
- **Touch/install/security:** all visible links and buttons meet 44×44 CSS px; the manifest includes original 192px and 512px icons; HSTS and Permissions-Policy are sent with the existing CSP, nosniff, and no-referrer policy.
- **Formatting:** `cargo fmt --all -- --check` is part of `npm run check` and passes.

## Verification evidence

Run from `/work/repo`:

```sh
npm ci
npm test
npm run check
cargo fmt --all -- --check
BUILD_SHA="$(git rev-parse HEAD)" cargo build --locked --release --bins
```

Results on 2026-08-28 UTC:

- `npm ci`: PASS, lockfile-clean install, 0 npm vulnerabilities.
- `npm test`: PASS — Vitest 2/2; Rust API integration 8/8; Vite production build; Playwright 13 passed with 3 intentional project skips.
- `npm run check`: PASS — TypeScript no-emit, rustfmt, and Clippy all targets with warnings denied.
- Clean consumer `cargo install --locked --path . --root <temporary-prefix>`: PASS; installed CLI and server completed register → start → finish → v2 receipt.
- Runtime with only `PORT`: PASS; generated and persisted its private configuration and served the embedded build SHA.
- Load/concurrency: 100 health requests passed; 50 concurrent signed writes returned `201`.
- Production build budgets: JS 17,713 bytes; CSS 18,464 bytes; mobile hero WebP 19,782 bytes; desktop hero WebP 59,480 bytes. Fonts remain self-hosted.
- Factory URL verifier: HTTPS 200; title, `lang`, one `h1`, `main`, alt text, button names, and console checks all passed.

Live Playwright exercised desktop 1440px and mobile 390px, both light and dark:

- one `h1` and visible `main`; first Tab focuses “Skip to ledger”;
- no horizontal overflow, including 390px at 200% text;
- no targets below 44×44 px;
- zero console/page errors and zero third-party requests;
- Axe WCAG A/AA: zero serious or critical findings;
- controlled service-worker offline reload: PASS.

Live Lighthouse 12.8.2 mobile:

- Performance 100, Accessibility 100, Best Practices 100, SEO 100;
- FCP 0.77s, LCP 0.90s, TBT 26ms, CLS 0;
- total transfer 100,816 bytes.

## Deployment evidence

- The exact multi-stage Dockerfile built successfully in Azure Container Registry as a non-root image.
- The deployment retains the original `container` class and port 8080 contract.
- Azure state was inspected after rollout: one running/ready replica, `/data` volume and mount present, scale fixed at one.
- `/health` returned the full deployed source SHA; live `app.js`, `app.css`, and `sw.js` byte-matched the local production output.
- Stable assets returned `no-cache`; live responses returned HSTS, Permissions-Policy, CSP, nosniff, and no-referrer; an unknown API path returned `404 application/json`.
- Restart durability was verified against the mounted production volume before handoff.

## Operations

- Keep this SQLite deployment at one replica. A future multi-replica deployment requires migration to a shared PostgreSQL ledger; it must not be enabled by only raising the scale ceiling.
- Back up both `/data/run-proof.db` and `/data/run-proof.secret` together. The secret is the CLI signing identity; losing it prevents new signed ingest, while changing it invalidates verification with that deployment identity.
- Health: `GET /health`. Readiness and liveness probes use that endpoint.
- No secrets are committed or printed in logs. Startup reports only whether database, secret, and key ID were generated, persisted/defaulted, or supplied.

## Known gaps

No release-blocking product-QA gaps remain from the independent verifier report. External checkout behavior remains owned by the Sociobot billing API as designed and was not altered by this repair.
