# Independent verification 3 — FAIL

- Work order: `job-liveness-proof-verify-3`
- Candidate: `fd8a53a66955aa524d076970c109cef948863dc3`
- URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-08-28 UTC
- Result: **FAIL — do not promote**

The candidate's ordinary test suite, release builds, browser basics, API validation, rate limiting, and candidate identity pass. It nevertheless fails mandatory acceptance gates and the core reliability job. `.factory/claims.json` and the demo sandbox do not exist; the cold first screen does not identify its intended user or offer a one-click sample; completed receipts are not append-only; and fresh Azure evidence shows the live SQLite database and generated signing secret have no durable volume while the app may scale to three replicas.

## Mandatory gate results

### FAIL — claims gate

The checkout began clean at the exact requested commit. Before general QA, `.factory/claims.json` was checked and was **missing**. There were therefore no listed claim commands to execute. A repository search found zero `@claim:` tests. This is release-blocking under the supplied claims contract.

The live page and README still make testable claims without registry entries, including:

- “HMAC signed”, “No payloads”, and “Offline export”.
- Signed ingest, contradiction detection, configurable retention, and every export.
- Full CSV export and a last-known offline view.
- No analytics, tracking, third-party fonts, or runtime CDN scripts.

`.factory/demo.md` and `.factory/copy-audit.md` are also missing.

### FAIL — cold first-read and demo gate

Fresh Chromium at 1440×900 showed:

- Headline: “Know the job ran. Keep the proof.”
- Supporting line: “A compact, self-hosted ledger for the gap between ‘the check is green’ and ‘the work actually finished.’”
- First actions: “Connect a job” and “Read the ledger”.

This communicates the broad function, but it does not plainly identify the intended small application teams operating cron jobs and queue workers. It does not explain what happens after the first click. Its three hero facts omit the price. Most importantly, there is no “Try it with sample data” action.

`GET /demo` returns the ordinary empty production app with `200`, no sample data, no demo banner, no reset/start-for-real controls, and no isolated demo tenant. `?demo=1` is likewise not handled. The clean-installed CLI exposes register/start/finish/snapshot only; it has no `demo` command or bundled sample workspace. This independently forces **FAIL**.

## Release-blocking defects

### Critical — C1: live evidence and signing identity are ephemeral

Fresh read-only Azure inspection contradicts the prior PASS handoff:

- `npm run verify:deployment -- fd8a53a66955aa524d076970c109cef948863dc3` exited `1`.
- Active/ready revision: `sf-job-liveness-proof--0000013`, 100% traffic, one current ready replica.
- Image: `sociobotregistry.azurecr.io/sf-job-liveness-proof:fd8a53a66955`; ACR digest `sha256:bf934d765a9fbd953b4fb3128c2b2e44f1733bd5327ec88f01a46d6dec4ff1d9`.
- Scale is `minReplicas=1`, `maxReplicas=3`, not the required fixed single replica.
- `properties.template.volumes` and container `volumeMounts` are both `null`.
- Revision startup logs state `database=defaulted`, `secret=generated`, and port `8080`.

The container therefore writes `/data/run-proof.db` and `/data/run-proof.secret` to replica-local ephemeral storage. A restart or revision replacement can discard the ledger and rotate the verification identity. Scaling can create multiple independent ledgers and secrets. This defeats the product's primary promise of a trustworthy execution record.

Required repair: deploy with the existing durable Azure Files volume mounted at `/data` and force exactly one replica, or move both ledger and signing identity to shared multi-replica-safe storage. Make the deployment verifier part of the release gate and prove survival across a real replica replacement.

### High — H1: required claim registry and claim tests are absent

No claim can be independently run from the required demo entry point because `.factory/claims.json` does not exist, no tests are tagged `@claim:*`, and the product/README contain several unlisted claims. Add one observable demo-based test for each retained claim and run every manifest command from a clean state.

### High — H2: required one-click isolated demo and first-screen contract are absent

The site cannot be tried without configuring a receiver secret and CLI. `/demo` silently falls through to the production app, and neither web nor CLI offers seeded sample evidence. Add an ephemeral, isolated demo with realistic missed/late/contradictory/completed runs, a persistent “Demo — sample data, nothing is saved” banner, Reset demo, Start for real, and documentation in `.factory/demo.md`. Rewrite the first screen to state what it does, for whom, and the first result in plain words.

### High — H3: re-registering a job mutates historical receipt intent and hash

The `jobs` table stores one current registration per `job_key`; `register_job` uses `ON CONFLICT ... DO UPDATE`. Receipt export joins every historical run to that current registration rather than the registration that governed the run.

Fresh isolated reproduction:

1. Register `billing-sweep` at 60 seconds / 30 seconds grace.
2. Record start, successful finish with count `0`, and a failed CI observation for `billing-normal`.
3. Export the receipt: hash `sha256:126eda9683a884ef9a0d883b6da7bfd7c8c844c8eed2d9d9192a6fee8b1e9926`.
4. Re-register the same job at 3600 seconds / 300 seconds grace.
5. Re-export the completed run: hash changed to `sha256:db96cdf4116a4c903aaf3193f38df6cdfde3cf913bbb34488692c30d0705de26`, and its registration body now contains the new schedule.

The signed run events remain verifiable, but the exported receipt's schedule intent and top-level receipt hash change after completion. That violates the brief's append-only receipt requirement and weakens historical auditability. Preserve versioned registrations and bind each run/derived alert to the applicable version.

## Other defects

### Medium — M1: the specified self-hosted typefaces are not applied

The generated `@font-face` declarations name `Bitter Variable` and `Atkinson Hyperlegible Next Variable`, while selectors request `Bitter` and `Atkinson Hyperlegible Next`. Browser font inspection showed the `h1` rendered with non-custom `Liberation Serif` and body copy with non-custom `DejaVu Sans`; no WOFF2 resource was requested. This diverges from `.factory/design.md` and makes the shipped font files dead weight.

### Medium — M2: required route/metadata/handoff chrome is incomplete

- An unknown browser path such as `/not-a-route` returns `200` and the home page instead of a designed 404.
- All views lack canonical, Open Graph, Twitter card, and apple-touch metadata.
- The footer omits “Built by Param Factory” and a version/build identifier.
- Route transitions do not implement the required in-app focus move/announcement; privacy and terms are full document navigations.

### Low — L1: stable/static assets do not receive long-lived immutable caching

The shell, service worker, stable `app.js`/`app.css`, content-hashed font files, and images all return `Cache-Control: no-cache`. This is update-safe but misses the supplied immutable-cache policy for content-addressed assets.

## Clean install, tests, checks, and builds

- Initial state: clean `main`, `HEAD=fd8a53a66955aa524d076970c109cef948863dc3`, equal to `origin/main`.
- `npm ci`: PASS — 60 packages installed, 0 audit vulnerabilities.
- `npm test`: PASS.
  - Vitest: 2/2.
  - Rust integration: 9/9.
  - Vite production build: PASS, `dist/` produced.
  - Playwright: 15 passed, 3 intentional desktop/mobile project skips.
- `npm run check`: PASS — TypeScript no-emit, rustfmt check, and Clippy all targets with warnings denied.
- `npm run build`: PASS.
- Clean target build: `CARGO_TARGET_DIR=/tmp/runproof-release-fd8 BUILD_SHA=fd8a... cargo build --locked --release --bins`: PASS.
- The clean release server started with an otherwise empty environment containing only `PORT=4190`, served `/` and `/health`, generated a mode-`0600` secret, reused the same secret after restart, and reported the full candidate SHA.
- Clean consumer install: `cargo install --locked --path . --root <temporary-prefix> --bins --force`: PASS; both `run-proof` and `run-proof-server` installed.
- Docker/Podman were unavailable locally. The exact Docker stages were reproduced with locked Vite and clean-target Cargo release builds; the live tagged ACR image and matching health identity confirm the factory image build completed.

Production output sizes:

- JavaScript: 17,881 bytes / 7.05 KB gzip (budget 200 KB).
- CSS: 18,464 bytes / 5.29 KB gzip (budget 50 KB).
- Hero WebP: 19,782 bytes mobile and 59,480 bytes desktop (budget 300 KB).
- Lighthouse total transfer: 98 KiB.

## End-to-end/backend evidence

An isolated receiver and clean-installed CLI exercised the smallest useful product without touching production data:

- Register → signed start → successful finish with count `0` → failed GitHub Actions snapshot produced one contradictory run and one derived missed run.
- JSON receipt v2 contained registration, two events, one snapshot, exact signed bodies/timestamps/key IDs/signatures, and an integrity hash. All four HMACs independently recomputed successfully.
- CSV export contained two records with the expected header, contradictory status, completion count `0`, source status, and receipt hashes.
- Status filtering, no-match search, Clear filters recovery, JSON download, and CSV download worked without console/page errors.
- Boundaries `60/0` and `31536000/86400` were accepted. Interval `59`, grace `86401`, invalid ID, unknown payload field, bad signature, 301-second clock skew, negative completion count, non-HTTP source URL, duplicate start, and missing receipt returned useful `400`/`401`/`409`/`404` responses. A valid CLI registration succeeded after an invalid attempt.
- The rejected payload marker was absent from SQLite/WAL files.
- Fifty concurrent signed registrations with independent forwarded client IPs all returned `201`. The 100-request health load smoke passed.
- Local restart retained the completed receipt and its count `0`.
- Local normal API limiting began at request 44 during a rapid sequential 45-request burst (43×`200`, 2×`429`); the stricter license route began at request 11 (10×`400`, 5×`429`). Every `429` carried `Retry-After: 1`.

Fresh live rate-limit evidence also passes the mandatory observable behavior:

- General API concurrent burst: 182×`200`, 68×`429` out of 250 in 1.829 seconds; every `429` had `Retry-After: 1`.
- Same-origin license proxy burst: 30×`200`, 10×`429` out of 40 in 0.512 seconds; every `429` had `Retry-After: 1`.
- Health remains explicitly exempt, as permitted.

Authentication is not part of Run Proof, so the Microsoft Entra authority check is not applicable.

## Live identity, browser, privacy, PWA, and response policy

- `GET /health` returned `200` with the exact full candidate SHA.
- Local/live SHA-256 matched for `index.html` (`83af5233…`), `app.js` (`30e5ab28…`), `app.css` (`aae76e0a…`), `sw.js` (`07ff5d8f…`), manifest, and both hero images.
- HTTP redirects to HTTPS. The hostname certificate is valid 2026-08-28 through 2027-02-28.
- Fresh 1440×900 light and 390×844 dark reviews found no horizontal overflow, including 390px at 200% root text size. No visible interactive target was below 44×44 CSS px.
- Both views have `lang=en`, one `<h1>`, one `<main>`, header/nav/footer landmarks, ordered headings, and meaningful hero alt text.
- First Tab focuses “Skip to ledger” with a visible 3px amber outline. A complete Tab sequence reached every visible action. Keyboard activation opened Restore purchase and moved focus to its labeled input; filters updated `aria-pressed`.
- Reduced-motion mode computed `scroll-behavior:auto`. Fresh Axe WCAG A/AA checks found zero serious/critical violations on desktop and mobile. Ordinary cold loads had no console/page errors.
- Ordinary home loading contacted only the product origin. No analytics, CDN fonts/scripts, cookies, or unexpected third-party browser requests were observed. An invalid returned license was stripped from the URL, stored under the required key, POSTed only to the same-origin verification proxy, cached invalid, and did not unlock Plus.
- Untrusted-origin API requests received no `Access-Control-Allow-Origin`. Responses include CSP, HSTS, `nosniff`, `Referrer-Policy: no-referrer`, and restrictive Permissions-Policy. Unknown API routes return structured `404 application/json`.
- Service worker registration used the live `/sw.js`; `registration.update()` completed and cache `run-proof-shell-v3` was active. A controlled offline reload rendered “Offline · showing last copy” with the main landmark intact. The expected failed network fetch was the only offline console/request error.
- Error recovery: aborting the first ledger request rendered “Could not reach the ledger” and keyboard-operable “Try again”; retry recovered to “Receiver connected”.
- Lighthouse 12.8.2 mobile: Performance 90, Accessibility 100, Best Practices 100, SEO 100; FCP 1.6 s, LCP 2.0 s, TBT 370 ms, CLS 0, Speed Index 1.6 s. Lighthouse warned that the verifier CPU was slower than its calibration target.

## Final disposition

**FAIL. Do not promote.** Fix the live durable-storage/single-replica topology, add the mandatory isolated demo and claim manifest/tests, preserve versioned schedule intent in historical receipts, and repair the font/route metadata gaps. Then deploy a new candidate and repeat independent verification from fresh state.
