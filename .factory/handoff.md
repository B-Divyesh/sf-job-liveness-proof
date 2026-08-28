# Run Proof independent QA handoff — FAIL

- Work order: `job-liveness-proof-verify-2`
- Candidate: `f38ee716da6aee2448647b903cb936f9b9d8f90d`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Verification report: `.factory/verification-2.md`
- Result: **FAIL — do not promote**

## Release blockers

1. **Critical: production state and HMAC identity are ephemeral and split across replicas.** Active revision `sf-job-liveness-proof--0000009` had three ready replicas, scale `1..3`, only `PORT`, and no volume/mount. Each replica had a different generated `/data/run-proof.secret` hash. Ledger data and signing identity are therefore neither shared nor durable.
2. **High: receiver rate limiting is noncompliant.** It covers only known POST routes, keys on socket peer instead of the first `X-Forwarded-For` hop, leaves GET API routes unlimited, and emits 429 without `Retry-After`. Live evidence: a 250-request POST burst yielded 200×401 and 50×429 with no retry header; 150 GET requests all returned 200.
3. **High: Sociobot product verification was not rate limited.** A 500-request burst to the product unlock verify endpoint returned 500×200, with no 429 or `Retry-After`.

## What passed

- Clean install, full repository tests, TypeScript, rustfmt, Clippy, Vite production build, and SHA-embedded release Rust build.
- Vitest 2/2, Rust integration 8/8, Playwright 13 passed with 3 intentional skips.
- Clean consumer install and CLI-to-receiver register/start/finish/snapshot/receipt flow.
- Signed input validation and recovery, zero-count completion, contradiction detection, out-of-order finish, job-scoped receipts, HMAC recomputation, CSV export, 50 concurrent writes, and local restart persistence.
- Live candidate identity and byte match: `/health` reports the full candidate; index, JS, CSS, and service worker hashes match local output.
- Desktop/mobile, 200% text, keyboard, visible focus, reduced motion, no undersized controls, no console/page errors, and zero axe serious/critical findings.
- PWA update/offline reload, privacy and legal pages, first-party-only ordinary loading, security headers, and safe shell caching.
- Lighthouse mobile: 94 Performance, 100 Accessibility, 100 Best Practices, 100 SEO; LCP 1.3 s, CLS 0; total transfer 100,565 bytes.

## Reproduce

```sh
npm ci
npm test
npm run check
BUILD_SHA=f38ee716da6aee2448647b903cb936f9b9d8f90d cargo build --locked --release --bins
```

See `.factory/verification-2.md` for exact local/live observations, burst results, hashes, and remediation requirements. No product code was modified; only verification documentation changed.
