# Independent verification — FAIL

- Work order: `job-liveness-proof-verify-1`
- Candidate: `802b646ff3ecc0dbdc48cee35e88400ad13c3452`
- URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-08-28 UTC
- Result: **FAIL**

The live site is healthy and byte-matches the candidate frontend, and the ordinary happy path works in a single local process. It is not ready for the real job-to-be-done. The live deployment cannot maintain one signing identity or one ledger across replicas, receipts are not scoped to a job, and an accepted out-of-order event can crash the ledger UI.

## Release identity and deployment

- `GET /health` returned `200` and `{"build_sha":"802b646ff3ecc0dbdc48cee35e88400ad13c3452","status":"ok"}`.
- Azure revision `sf-job-liveness-proof--0000002` was `Running` / `Healthy`, image `sociobotregistry.azurecr.io/sf-job-liveness-proof:802b646ff3ec`.
- Live `app.js`, `app.css`, and `sw.js` SHA-256 values exactly matched the clean candidate build.
- HTTP redirects to HTTPS. Live responses included the candidate CSP, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, and cache policies.

## Defects

### Critical

#### C1 — The live deployment has two independent ephemeral ledgers and signing keys

Fresh Azure inspection showed `minReplicas=1`, `maxReplicas=3`, two running replicas, only `PORT=8080`, and no volume or volume mount. Console logs for both replicas independently reported `database=defaulted` and `secret=generated` (starts at 03:23:38Z and 03:53:21Z). The binary therefore creates a different `/data/run-proof.db` and CSPRNG signing secret in each replica's ephemeral filesystem.

Impact:

- An operator has no supplied/shared secret with which to configure the CLI.
- Even if one generated file were recovered, traffic can reach the other replica with a different secret.
- Reads and writes can hit different SQLite ledgers.
- Scale-down, restart, or revision replacement can discard evidence and rotate the signing identity.

This blocks signed ingest and durable evidence on the advertised live product and violates the required persistence boundary.

### High

#### H1 — Receipts merge different jobs that reuse a run ID

Run IDs are unique only within a job in SQLite, but the receipt route is `/api/v1/runs/:run_id/receipt` and queries only `WHERE run_id=?`.

Reproduction:

1. Register `job-a` and `job-b`.
2. Send start and finish events for `shared-run` to each job.
3. Fetch `/api/v1/runs/shared-run/receipt`.

Observed: one receipt contained four events from both jobs, omitted `job_key` from every event, and combined success/count `10` for Job A with failed/count `2` for Job B. Both UI rows export the same mixed receipt. Derived missed-run IDs can collide the same way because they use only `missing:<timestamp>`.

This defeats the requirement to export a receipt for each alert and can attribute evidence to the wrong job.

#### H2 — A valid finish-before-start event crashes all ledger row rendering

The API accepts a signed finish for a registered job even when no start exists. The resulting ledger row has `scheduled_at: ""`. The frontend passes that value to `Intl.RelativeTimeFormat`, which throws `Value need to be finite number for Intl.RelativeTimeFormat.prototype.format()`.

Observed in Chromium: two page errors, the summary counts updated, but the ledger remained forever on “Reading signed receipts…” and no rows or recovery action rendered. The bad row persisted across server restart and continued to break the ledger.

Out-of-order delivery is a realistic receiver condition; one accepted event must not make every receipt inaccessible.

#### H3 — Exported HMAC signatures cannot be verified from the receipt

Ingest signs `unix_timestamp + "." + exact_JSON_body`, but persisted/exported events omit the request timestamp and exact raw body. Registration signatures are not persisted at all. The receipt nevertheless instructs the reader to recompute the HMAC using the deployment secret.

Without the signed timestamp and exact bytes, recomputation is impossible; reconstructed JSON is not guaranteed byte-identical. The exported receipt hash is also an unsigned digest. Thus the portable artifact cannot prove the signed schedule/start/finish observations it claims to carry.

### Medium

#### M1 — 200% text resize loses the responsive layout

At a 390px viewport, setting the root text size from 16px to 32px produced `scrollWidth=537` for `clientWidth=390`. The ledger occupied only the left part of the viewport, status/filter content was clipped, and horizontal scrolling was required.

#### M2 — Service-worker updates can retain stale application code for one year

Vite deliberately emits stable names `/assets/app.js` and `/assets/app.css`, while the server sends `Cache-Control: public, max-age=31536000, immutable` for every `/assets/` response. The service worker manually bumps a cache name and calls `cache.addAll` with those same stable URLs. A new worker can therefore populate its new cache from the browser's still-immutable old response.

Offline reload works after an online controlled reload, but the update strategy is unsafe. Immutable caching must be limited to content-hashed filenames or the shell assets must revalidate.

#### M3 — The ingest rate limiter is unauthenticated, bypassable, and unbounded

The limiter buckets on the attacker-controlled `X-Run-Proof-Key` before signature validation and never removes map entries. A burst using one key returned 100×`401` then 5×`429`; the same 105 requests with distinct key values returned 105×`401` and created 105 persistent buckets. An unauthenticated client can bypass the limit and grow server memory without bound.

#### M4 — Unknown API paths return the SPA with `200 text/html`

`GET /api/v1/does-not-exist` returned the app HTML with status `200`, rather than a structured `404`. API clients can treat a missing route as success and then fail while parsing HTML.

### Low

- Several links are below the contract's 44×44 CSS-pixel target: the 36px-tall brand and 16–24px-tall legal/footer links. Lighthouse's less strict target-spacing audit still passed.
- The PWA manifest has `"icons":[]`; offline reload works, but normal install surfaces lack required app icons.
- Live responses omit HSTS and Permissions-Policy. HTTPS redirect, CSP `frame-ancestors 'none'`, nosniff, and no-referrer are present.
- `cargo fmt --all -- --check` fails across the committed Rust sources and tests. The repository's declared TypeScript/Clippy check passes.

## Clean checkout and build evidence

A detached, clean checkout was created at `/tmp/run-proof-qa.F29glm` from the candidate before installing or testing.

- `npm ci`: PASS; 60 packages installed; npm audit reported 0 vulnerabilities.
- `npm test`: PASS.
  - Vitest: 2/2.
  - Rust integration: 4/4.
  - Vite production build: PASS, `dist/` produced.
  - Playwright: 9 passed, 1 intentionally skipped (desktop copy of the mobile-only overflow test).
- `npm run check`: PASS (`tsc --noEmit`; Clippy all targets with warnings denied).
- `BUILD_SHA=802b646... cargo build --locked --release --bins`: PASS.
- `cargo fmt --all -- --check`: FAIL (formatting diffs).
- `cargo install --locked --path . --root /tmp/run-proof-qa-consumer`: PASS; both CLI and server installed into a clean consumer prefix.
- A second release build with the full candidate `BUILD_SHA` started with an otherwise empty environment containing only `PORT`, served `/` and `/health`, generated a 65-byte secret file with mode `0600`, and reported the full candidate SHA.
- The exact Dockerfile could not be executed in this verifier container: Docker/Podman were absent, and an installed Buildah failed immediately because the host forbids `CLONE_NEWUSER`. Both Docker build stages were reproduced directly, and the live candidate image proves the factory image build completed.

## Product and backend exercises

Passed in a clean local deployment unless noted:

- Signed register → start → successful finish with completion count `0` → failed CI snapshot → contradictory ledger row.
- Derived missed run, per-run JSON download, full CSV download, search/no-match/clear recovery, and status filtering.
- Browser receipt and CSV downloads contained the expected normal run.
- Boundary values: interval/grace `60/0` and `31536000/86400` accepted; interval `59`, negative completion count, invalid source scheme, short secret, unknown job, duplicate event, stale clock, bad HMAC, unsigned request, and unknown payload all rejected with useful errors; a valid request succeeded after each error sequence.
- Unknown payload data was rejected and the schema contains no payload column.
- Persistence across a local server restart retained all rows.
- 100 concurrent health requests completed; 50 concurrent signed SQLite writes all returned `201`; the one-key rate limit returned `429` after request 100.
- `/health` exposes build identity; `/api/v1/config` exposed retention `30`, skew `300`, and `payload_storage:false`.
- The installed CLI produced useful nonzero exits and error text for invalid input and receiver errors.

## Live browser, accessibility, privacy, and performance

- Manually reviewed full-page screenshots at 1440×900 light/dark and 390×844 mobile. Normal-size layouts had no document overflow; mobile ledger rows switch to tickets.
- Chromium desktop, dark, mobile, and mobile privacy page: one `<h1>`, one `<main>`, `lang=en`, correct titles, no console errors, no page errors, no failed requests.
- Axe WCAG A/AA: 0 serious or critical findings on those live views.
- Keyboard: first Tab focused “Skip to ledger”; focus style was a visible 3px amber solid outline. Filters, search, retry, restore, receipt export, and CSV export were native keyboard-operable controls.
- Reduced motion: smooth scrolling disabled; transitions/animations reduced to 0.01ms with one iteration.
- Error recovery: with the first ledger request aborted, the explicit error panel appeared; after restoring the request, “Try again” reached “Receiver connected”.
- PWA: cache `run-proof-shell-v2` installed; after an online controlled reload, an offline reload showed `Offline · showing last copy`, one main landmark, and no page error.
- Privacy: ordinary home/privacy loads contacted only the product origin; no analytics, CDN fonts, tracking, or third-party scripts. A supplied invalid license was stored under `sb_license:job-liveness-proof`, removed from the URL, verified only against `https://api.sociobot.in`, cached as invalid, and did not unlock Plus. The API returned the expected invalid verdict with origin-scoped CORS and `no-store`.
- Lighthouse 12.8.2 mobile: performance 99, accessibility 100, best practices 100, SEO 100; FCP 0.9s, LCP 1.1s, TBT 140ms, CLS 0, Speed Index 0.9s; 8 requests / 94,220 bytes; no failed binary audits.
- Candidate build sizes: JS 17,352 bytes; CSS 17,764 bytes; mobile hero WebP 19,782 bytes; desktop hero WebP 59,480 bytes. Initial network transfer stayed well below the supplied budgets.

## Required disposition

Do not promote this candidate. At minimum, use a single durable shared datastore/signing configuration in deployment, scope receipts by both job and run, preserve all material needed to verify signatures, and make malformed/out-of-order rows render safely. Re-run the full verification after repair.
