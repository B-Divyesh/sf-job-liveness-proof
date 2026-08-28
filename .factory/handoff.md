# Run Proof repair handoff — PASS

- Work order: `job-liveness-proof-repair-3`
- Failed report commit: `8c55a19cd8fe4b1586dbe9f149e410682953bec6`
- Failed candidate: `f38ee716da6aee2448647b903cb936f9b9d8f90d`
- Verified implementation commit: `5581bee62088fe23c1af5de0621a51321c66c541`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Artifact/deployment class preserved: `web-with-backend`, Rust/axum + SQLite + Vite/TypeScript, one container on `PORT=8080`

All three release blockers in `.factory/verification-2.md` are repaired. The brief, paper-cut visual system, signed ingest, verifiable receipts, offline ledger, exports, and free/Plus behavior that previously passed are preserved.

## Repairs and regression coverage

### C1 — durable evidence and one signing identity

- `scripts/deploy-container.sh <full-sha>` is now the factory release path. It builds the repository Dockerfile with the full `BUILD_SHA`, then atomically reasserts the existing `data-job-liveness-proof` Azure Files volume at `/data` and `minReplicas=1` / `maxReplicas=1` on every image rollout. A generic image update can no longer silently omit the product's SQLite topology requirements.
- `scripts/verify-deployment.sh <full-sha>` fails unless the live image reports that exact SHA, both scale bounds equal one, the `AzureFile` volume exists, and the container mounts it at `/data`. `npm run verify:deployment -- <sha>` exposes the same check.
- SQLite and the generated HMAC secret remain together under `/data`; startup uses the network-filesystem-safe `unix-dotfile` VFS and one connection. Existing runtime regressions still prove atomic mode-`0600` secret creation/reuse and durable-mount database writes.
- Live durability proof: revision `0000010` accepted signed job `repair3-bb58c22` and run `before-restart`; a real revision restart replaced replica `...txk4g` with `...gml7t`; the secret SHA-256 remained `fe80b04d…` on both replicas and the signed row remained readable. The later `0000011` rollout also logged `secret=persisted` and retained that row, proving restart and revision-replacement persistence.

### H1 — complete receiver rate-limit contract

- Every `/api` route, including reads, writes, receipt/export routes, the license proxy, and unknown API paths, passes through one bounded token-bucket middleware. `/health` is the only explicit exemption so platform probes cannot be starved.
- Client identity is the first valid `X-Forwarded-For` hop, falling back to the socket peer only when the ingress header is absent/invalid. Buckets are capped at 4,096 entries and evicted after five idle minutes.
- Ordinary APIs admit 20 requests/second with burst 40. License verification is stricter at 5 requests/second with burst 10. Every product-generated `429` is structured JSON with `Retry-After: 1`.
- `every_non_health_route_uses_first_forwarded_ip_and_returns_retry_after` covers a GET burst, first-hop selection across a multi-hop header, rotating unauthenticated key IDs, POST limiting, the retry header, structured errors, and health exemption.
- Final live bursts: `/api/v1/config` returned 84×`200` and 16×`429`; the license proxy returned 13×`200` and 27×`429`; every one of the 43 limited responses had `Retry-After: 1`. The refill explains admission above the initial burst. A simultaneous 60-request health burst returned 60×`200`.

### H2 — paid verification is product-rate-limited

- The browser now sends a same-origin `POST /api/v1/products/job-liveness-proof/verify` JSON body. The receiver validates it, applies the stricter bucket, and calls the required Sociobot `GET .../verify?license=` server-side with a 10-second timeout.
- License tokens no longer appear in the browser URL, product request logs, referrers, or third-party browser requests. Proxy verdicts use `Cache-Control: no-store`; the browser still verifies at most daily, unlocks optimistically from a cached valid verdict, and keeps the free experience available offline.
- `license_verification_is_same_origin_proxied_no_store_and_rate_limited` uses an isolated fake Sociobot service to prove forwarding, the JSON verdict, `no-store`, malformed-input handling, 429 behavior, and `Retry-After`.
- Playwright regression `returned licenses use the rate-limited same-origin verification proxy` proves the returned token is stripped from the page URL, stored under the required key, sent only in a same-origin POST body, and never requested from a third-party browser origin.

## Complete verification evidence

Executed from `/work/repo` on 2026-08-28 UTC:

```sh
npm ci
npm test
npm run check
BUILD_SHA=5581bee62088fe23c1af5de0621a51321c66c541 cargo build --locked --release --bins
cargo install --locked --path . --root <clean-temporary-prefix> --bins --force
scripts/deploy-container.sh 5581bee62088fe23c1af5de0621a51321c66c541
scripts/verify-deployment.sh 5581bee62088fe23c1af5de0621a51321c66c541
```

- Clean `npm ci`: PASS — 60 packages, 0 audit vulnerabilities.
- `npm test`: PASS — Vitest 2/2; Rust integration 9/9; Vite production build; Playwright 15 passed with 3 intentional desktop/mobile project skips.
- `npm run check`: PASS — TypeScript no-emit, rustfmt check, and Clippy all targets with warnings denied.
- SHA-embedded locked release build: PASS for both CLI and server.
- Clean consumer package install: PASS; installed binaries completed register → start → finish with count `0` → job-scoped v2 receipt with two signed events.
- Runtime with only `PORT`: PASS; generated a private secret, restarted with `secret=persisted`, retained its database, served the frontend, and returned the embedded build SHA.
- Load/concurrency: 100 health requests passed; the previously verified 50 concurrent signed writes remain covered by the unchanged ingest/database path.
- Production sizes: JS 17,881 bytes, CSS 18,464 bytes, mobile hero WebP 19,782 bytes, desktop hero WebP 59,480 bytes. Fonts remain self-hosted; no analytics/CDN scripts were introduced.

## Browser, accessibility, privacy, offline, and performance

- Automated and visual review covered 1440×900 light mode and 390×844 dark mode. Both have one `<h1>`, one `<main>`, `lang=en`, no horizontal overflow, no visible target below 44×44 px, and first Tab focus on “Skip to ledger”. The product-specific paper ledger layout remains intact.
- Playwright covers keyboard actions, 390px reflow at 200% text, out-of-order evidence, legal pages, error recovery, cache headers, install metadata, reduced motion, service-worker update/offline reload, and first-party-only ordinary loading.
- Fresh live Axe WCAG A/AA: zero serious/critical findings on desktop and mobile. No console errors, page errors, or ordinary third-party requests occurred. Reduced motion reported `scroll-behavior:auto`.
- Live service-worker offline reload rendered `Offline · showing last copy` with the main landmark intact. Stable assets remain `no-cache`; the final shell, JS, CSS, and service worker byte-matched local output.
- Live Lighthouse 12.8.2 mobile: Performance 100, Accessibility 100, Best Practices 100, SEO 100; FCP 0.8 s, LCP 0.9 s, TBT 40 ms, CLS 0, total transfer 101,057 bytes.
- Live response policy: HTTPS, CSP restricted to self, HSTS, nosniff, no-referrer, restrictive Permissions-Policy, structured API 404s, `no-cache` stable shell assets, `no-store` license verdicts, and no receiver CORS grant to untrusted origins.

## Deployment evidence and operations

- ACR built the exact multi-stage Dockerfile from a source archive without `.git`; the non-root runtime image for implementation commit `5581bee…` has digest `sha256:6fb33e53d10e736a3e33b428b250f949ecaa499f738f3f2a42d9fe6a02d45a85`.
- Live revision `sf-job-liveness-proof--0000011` used that image, one ready replica, `minReplicas=1`, `maxReplicas=1`, volume `data-job-liveness-proof`, and `/data` mount. `/health` returned the full source SHA.
- Startup logged `database=defaulted`, `secret=persisted`, and `port=8080` without printing secret material.
- Keep SQLite at exactly one replica and back up `/data/run-proof.db` with `/data/run-proof.secret`. Horizontal scale requires migrating both ledger and signing identity to shared multi-replica-safe storage.

## Known gaps

No release-blocking QA gaps remain from the independent verifier report. The checkout and authoritative license verdict remain owned by the required Sociobot billing service; Run Proof now bounds and sanitizes its own verification path.
