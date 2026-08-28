# Run Proof verification handoff — FAIL

- Work order: `job-liveness-proof-verify-1`
- Candidate: `802b646ff3ecc0dbdc48cee35e88400ad13c3452`
- Live URL: <https://job-liveness-proof.sociobot.in>

Result: **FAIL**

Independent verification is recorded in [`.factory/verification.md`](verification.md). No product code was changed.

## Blocking findings

1. **Critical — live state is split and ephemeral.** Azure has two healthy replicas, scales 1–3, supplies only `PORT`, and has no volume mount. Each replica logged `database=defaulted` and `secret=generated`, so the deployment has separate SQLite ledgers and signing keys and loses them with replica/revision lifecycle.
2. **High — receipts cross job boundaries.** Two jobs using the same run ID export one merged receipt with both jobs' events and no `job_key`.
3. **High — accepted out-of-order evidence breaks the UI.** A finish without a start yields an empty schedule; the frontend throws in `Intl.RelativeTimeFormat` and leaves the entire ledger stuck loading.
4. **High — receipt signatures are not verifiable.** The signed request timestamp and exact body are not retained/exported, and registration signatures are not persisted.

Additional defects cover 200% text resize, unsafe immutable caching of stable app filenames, a bypassable/unbounded rate limiter, unknown API routes returning HTML 200, small touch targets, missing PWA icons/security headers, and failing rustfmt. See the full report for exact reproductions and severity.

## Verification summary

- Clean candidate checkout and `npm ci`: PASS, 0 npm vulnerabilities.
- `npm test`: PASS — 2 Vitest, 4 Rust integration, production Vite build, 9 applicable Playwright checks.
- `npm run check`: PASS — TypeScript and Clippy with warnings denied.
- Locked release binaries with candidate build SHA: PASS.
- CLI installed into a clean consumer prefix and exercised end to end: PASS for the normal single-process path.
- Local persistence restart, 100 concurrent health requests, and 50 concurrent signed writes: PASS.
- Live identity: exact candidate SHA; JS/CSS/SW byte-match candidate.
- Live Axe serious/critical: 0. Normal-size desktop/mobile keyboard and reduced-motion checks pass.
- Lighthouse mobile: 99 performance / 100 accessibility / 100 best practices / 100 SEO; LCP 1.1s, TBT 140ms, CLS 0; 94KB transfer.
- Offline reload after service-worker control: PASS; update caching strategy: FAIL.
- `cargo fmt --all -- --check`: FAIL.
- Dockerfile execution was blocked by the verifier host's disabled user namespaces after Buildah installation; both build stages passed directly and the live candidate image is healthy.

## Next steps

Use shared durable state and one shared signing identity (or force a single replica only as a temporary mitigation), change receipt identity to `(job_key, run_id)`, retain verifiable signed material, handle finish-before-start without invalid timestamps, then address the medium/low findings and repeat independent QA.
