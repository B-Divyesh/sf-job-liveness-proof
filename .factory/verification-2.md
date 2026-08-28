# Independent verification 2 — FAIL

- Work order: `job-liveness-proof-verify-2`
- Candidate: `f38ee716da6aee2448647b903cb936f9b9d8f90d`
- URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-08-28 UTC
- Result: **FAIL**

The candidate source passes its full local test/build suite and the live frontend is accessible, fast, private by default, and byte-identical to the candidate. The release nevertheless fails its core reliability job. Fresh Azure evidence shows that the live service is running three replicas with three different generated signing secrets and three ephemeral SQLite ledgers. It also fails the mandatory API rate-limit contract: protected responses omit `Retry-After`, read endpoints are unlimited, client identity ignores `X-Forwarded-For`, and the Sociobot license verification endpoint did not limit a 500-request burst.

## Release identity and deployment match

- The checkout began clean on `main` at exactly `f38ee716da6aee2448647b903cb936f9b9d8f90d`.
- Live `GET /health` returned `200` with `{"build_sha":"f38ee716da6aee2448647b903cb936f9b9d8f90d","status":"ok"}`.
- The active healthy revision was `sf-job-liveness-proof--0000009`, serving 100% of traffic from image `sociobotregistry.azurecr.io/sf-job-liveness-proof:f38ee716da6a`; the ACR manifest digest was `sha256:192e70fecbcf0ee0e2d423b2619b7358da00d7f0a22be7ae5e241b9bfe6edf00`.
- Live and local SHA-256 values matched exactly: `index.html` `83af5233…`, `assets/app.js` `c2f6ef80…`, `assets/app.css` `aae76e0a…`, and `sw.js` `07ff5d8f…`.
- HTTP redirected to HTTPS. The TLS certificate covered the product hostname and was valid 2026-08-28 through 2027-02-28.

## Release-blocking defects

### Critical — C1: production evidence and signing identity are replica-local and ephemeral

Fresh Azure inspection contradicts the previous repair handoff:

- Active revision `sf-job-liveness-proof--0000009` had `minReplicas=1`, `maxReplicas=3`, and was observed `RunningAtMaxScale` with three ready replicas.
- Its only environment variable was `PORT=8080`.
- `properties.template.volumes` and the container's `volumeMounts` were both `null`.
- Startup logs on a newly scaled replica reported `database=defaulted` and `secret=generated`.
- Read-only inspection of `/data/run-proof.secret` on the three live replicas returned three different SHA-256 hashes: `319aef84…`, `35efd53c…`, and `85f065fd…`.

The Docker image therefore writes `/data/run-proof.db` and `/data/run-proof.secret` to each container's ephemeral filesystem. Requests can reach different ledgers and cannot share one HMAC identity. Scale-down, restart, or revision replacement discards receipts and rotates signing identity. This directly defeats signed ingest, durable append-only evidence, and the core job-to-be-done.

Required disposition: mount durable storage for both database and secret and constrain this SQLite deployment to exactly one replica, or migrate the ledger and signing identity to shared multi-replica-safe storage. Verify durability across a real replica restart before release.

### High — H1: backend rate limiting violates the mandatory contract

The candidate middleware limits only `POST` requests on known API routes, keys buckets from the socket peer `ConnectInfo`, ignores the first `X-Forwarded-For` hop, and returns `429` without `Retry-After`.

Fresh observations:

- Local 130-request POST burst: 111×`401`, 19×`429`; every `429` had no `Retry-After` header. The source threshold is 100 requests per socket IP per one-second window.
- Live 250-request concurrent POST burst completed in 1,005 ms: 200×`401`, 50×`429`; every `429` lacked `Retry-After`. The observed aggregate admission threshold was 200 because ingress connections were split across peer-IP buckets/replicas.
- Live 150-request POST burst with the same `X-Forwarded-For: 198.51.100.77`: 150×`401`, proving the required forwarded client identity was not used.
- Live 150-request burst to `GET /api/v1/config`: 150×`200`; read API routes are not rate limited.

This both permits bypass and can make unrelated ingress traffic share a bucket. Apply rate limiting to every server-side endpoint except an explicitly exempt health check, use the first trusted `X-Forwarded-For` hop behind factory ingress, and attach a valid `Retry-After` value to every `429`.

### High — H2: the product-unlock verification endpoint has no observable rate limit

The browser's paid unlock calls `GET https://api.sociobot.in/api/v1/products/job-liveness-proof/verify`. Fresh bursts using an invalid test token produced:

- 150 concurrent requests: 150×`200`, no `429`.
- 500 concurrent requests in 3,610 ms: 500×`200`, no `429` and no `Retry-After`.

The endpoint returned correct origin-scoped CORS and `Cache-Control: no-store`, but it fails the work order's explicit rate-limit requirement for factory product-unlock calls. This is an external Sociobot API defect, but it remains an acceptance failure for this release.

## Clean install, checks, and production build

- `npm ci`: PASS; 60 packages installed, 0 npm audit vulnerabilities.
- `npm test`: PASS.
  - Vitest: 2/2.
  - Rust integration: 8/8.
  - Vite production build: PASS; `dist/` produced.
  - Playwright: 13 passed, 3 intentional project skips.
- `npm run check`: PASS (`tsc --noEmit`, `cargo fmt --all -- --check`, and Clippy all targets with warnings denied).
- `BUILD_SHA=f38ee716da6aee2448647b903cb936f9b9d8f90d cargo build --locked --release --bins`: PASS.
- Clean consumer `cargo install --locked --path . --root /tmp/run-proof-consumer.K7FVWG --bins --force`: PASS; installed both `run-proof` and `run-proof-server`, and the CLI public commands were exercised against an isolated receiver.
- No Docker/Podman/Buildah executable was available in the verifier. Both Dockerfile build stages were reproduced with the locked frontend and release backend commands; the live tagged candidate image confirms the factory image build completed.
- A release server started with an otherwise empty environment containing only `PORT`, reused its mode-`0600` 65-byte generated secret on restart, retained 54 ledger rows, and reported the full candidate SHA after the final release build.

Production artifact sizes:

- JavaScript: 17,713 bytes (budget 200 KB).
- CSS: 18,464 bytes (budget 50 KB).
- Mobile hero WebP: 19,782 bytes; desktop hero WebP: 59,480 bytes (budget 300 KB).
- Fonts are self-hosted, unicode-ranged WOFF2 files. The measured initial live transfer was 100,565 bytes across 9 requests.

## End-to-end product and backend evidence

Fresh manual testing used the installed CLI and isolated release receiver:

- Signed register → start → successful finish with completion count `0` → failed GitHub Actions snapshot produced a `contradictory` ledger row.
- The maximum schedule interval/grace (`31536000`/`86400`) and minimum (`60`/`0`) were accepted.
- Finish-before-start was accepted and serialized with `scheduled_at: null`; the ledger remained usable.
- Two jobs using `shared-run` exported separate job-scoped receipts with the correct event counts (`10` versus `2`) and no cross-job records.
- Receipt v2 exported registration/event/snapshot signed timestamps, exact bodies, key IDs, and signatures. All four HMACs in the normal receipt were independently recomputed successfully.
- CSV export contained the expected contradiction, count `0`, status, source, and receipt hash.
- Interval `59`, negative completion count, unknown job, `ftp:` source URL, duplicate start, bad secret, signed unknown `payload`, and stale signed timestamp were rejected with nonzero exits or `400`/`401`/`409`; a valid signed request succeeded after the failures.
- The database schema and strict request deserialization have no job payload field; a signed payload field was rejected.
- 50 concurrent signed SQLite writes all returned `201`; 100 concurrent health requests completed successfully.
- Restarting the isolated server preserved the same secret hash and all 50 concurrent run rows.
- Unknown API routes returned structured `404 application/json`, not the SPA shell.

## Live browser, accessibility, privacy, PWA, and performance

Fresh Chromium audits covered 1440×900 desktop light mode and 390×844 mobile dark mode:

- Correct title and `lang=en`; exactly one `<h1>` and one `<main>`; meaningful hero alt text.
- No horizontal overflow at normal size or at 390px with a 32px root font (200% text).
- No visible interactive target below 44×44 CSS px.
- First Tab focused “Skip to ledger” with a visible 3px solid amber outline. Keyboard Enter/Space operated purchase restore (and moved focus into the token input), filters, and CSV download.
- Reduced-motion media produced `scroll-behavior:auto` and 0.01 ms transitions.
- Axe WCAG A/AA found 0 serious or critical violations on home, privacy, and terms views.
- No console errors, page errors, failed requests, or unexpected third-party requests occurred on ordinary pages. Home/privacy data loads contacted only the product origin.
- Error recovery passed: an aborted first ledger request rendered “Could not reach the ledger”; keyboard-operable “Try again” recovered to “Receiver connected”.
- An invalid returned license was saved under `sb_license:job-liveness-proof`, removed from the URL, checked only against `api.sociobot.in`, cached invalid, and did not unlock Plus.
- PWA update check passed: registration used `updateViaCache:'none'`, `registration.update()` completed, and only `run-proof-shell-v3` remained. A controlled offline reload rendered `Offline · showing last copy` with no page error.
- Stable shell assets returned `Cache-Control: no-cache`, preventing stale stable-name assets. The manifest has 192px and 512px icons.
- Security policy was present on live responses: CSP, HSTS, `nosniff`, `Referrer-Policy: no-referrer`, and restrictive Permissions-Policy. An untrusted Origin received no CORS allowance from the receiver.
- Lighthouse 12.8.2 mobile: Performance 94, Accessibility 100, Best Practices 100, SEO 100; FCP 1.0 s, LCP 1.3 s, TBT 300 ms, CLS 0, Speed Index 1.0 s. No binary audit failed.

Authentication is not part of this product, so the Microsoft Entra tenant check is not applicable.

## Final disposition

**Do not promote.** Repair the production storage/replica topology and both rate-limit paths, then repeat independent verification from a new candidate commit and fresh deployment state.
