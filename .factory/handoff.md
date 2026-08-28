# Run Proof independent verification handoff — FAIL

- Work order: `job-liveness-proof-verify-3`
- Candidate: `fd8a53a66955aa524d076970c109cef948863dc3`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-08-28 UTC
- Result: **FAIL — do not promote**
- Full evidence: [`.factory/verification-3.md`](verification-3.md)

## Release blockers

1. **Critical — deployed evidence is not durable.** Fresh Azure state for active revision `sf-job-liveness-proof--0000013` has `minReplicas=1`, `maxReplicas=3`, no volumes, and no `/data` mount. Startup logged `database=defaulted` and `secret=generated`. The repository's deployment verifier exits `1`. The live SQLite ledger and signing identity can be lost on restart and diverge across replicas.
2. **High — `.factory/claims.json` is missing.** There are no `@claim:` tests, while the page and README make claims about signed ingest, payload privacy, offline use/export, retention, CSV/receipt exports, and tracking. This mandatory gate fails before general QA.
3. **High — no one-click isolated demo and cold first-read fails.** The first screen does not name the intended cron/queue-worker team, explain the first-click result, or offer “Try it with sample data”. `/demo` is only the ordinary empty app; `.factory/demo.md` and the CLI demo command/sample are absent.
4. **High — historical receipts are mutable.** Re-registering a job replaces the schedule registration attached to an already-completed run. In a fresh reproduction, the same receipt's hash changed from `sha256:126eda…` to `sha256:db96cd…` and displayed the later schedule. Preserve versioned registration intent per run.

## Other findings

- Medium: selectors request `Bitter` / `Atkinson Hyperlegible Next`, but bundled faces are named `Bitter Variable` / `Atkinson Hyperlegible Next Variable`. Chromium actually rendered Liberation Serif and DejaVu Sans and fetched no fonts.
- Medium: unknown browser paths return the home page with `200`; canonical/OG/Twitter/apple-touch metadata are absent; footer lacks Param Factory and build identity; route focus announcement is absent.
- Low: all static assets, including content-hashed ones, return `Cache-Control: no-cache` rather than immutable caching.

## What passed

- Clean install: 60 packages, 0 npm audit vulnerabilities.
- `npm test`: Vitest 2/2, Rust integration 9/9, Vite build, Playwright 15 passed / 3 intended skips.
- `npm run check` and standalone `npm run build`: PASS.
- Clean-target locked release build with the full candidate SHA: PASS. It starts with only `PORT`, generates a mode-`0600` secret, reuses it after restart, serves the app, and reports the candidate SHA.
- Clean consumer Cargo install: both CLI and server installed and the CLI public workflow was exercised.
- Isolated E2E: signed register/start/finish(count `0`)/failed CI snapshot, contradictory and missed states, receipt/CSV exports, independently verified HMACs, boundary and invalid inputs, recovery, 50 concurrent writes, 100 health requests, and restart persistence passed.
- Live identity: `/health` returns the exact candidate. Shell, JS, CSS, service worker, manifest, and hero assets byte-match local output.
- Live rate limiting: general burst 182×`200` + 68×`429`; license burst 30×`200` + 10×`429`; every `429` has `Retry-After: 1`.
- Desktop/390px mobile, 200% text, keyboard/focus, dark mode, reduced motion, Axe, ordinary console/network privacy, response headers/CORS, license proxy behavior, service-worker update/offline reload, and error recovery passed.
- Lighthouse mobile: Performance 90, Accessibility 100, Best Practices 100, SEO 100; LCP 2.0 s, CLS 0; transfer 98 KiB. JS 17,881 bytes and CSS 18,464 bytes.

## Reverification order

1. Deploy the candidate through `scripts/deploy-container.sh <full-sha>` and require `npm run verify:deployment -- <full-sha>` to pass; then prove data and secret survival across a real replica replacement.
2. Add `/demo` plus CLI demo/sample data, `.factory/demo.md`, `.factory/claims.json`, and exactly one tagged observable test per retained claim.
3. Version job registrations and bind historical runs/derived alerts to the registration in force at schedule time.
4. Correct font family names and complete 404, metadata, footer build identity, and route-focus behavior.
5. Repeat all commands and live checks in `.factory/verification-3.md` against a new commit/deployment.
