# Run Proof verification 4 handoff

- Work order: `job-liveness-proof-verify-4`
- Implementation reviewed: `e26a1fd14a84d1472b0b2f1d4aff1435bdae1041`
- Documentation commit before this report: `5326cafee0868d6bb11daccfbb4e65013568f3d1`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-09-06 UTC
- Verdict: **FAIL — 8 findings and 5 untested public claims**

## What was done

Independent verification covered the clean build and full suite, every declared
claim command, installed CLI/server workflow, first-screen phone and desktop
review, demo isolation/reset/exports, keyboard and reduced motion, Axe,
Lighthouse, offline/update behavior, route metadata and links, privacy/network
behavior, backend validation, both live rate limits, topology, and a real live
restart using an existing receipt.

The full evidence and earlier-finding disposition are in
[`.factory/verification-4.md`](verification-4.md).

## What passed

- `npm test`: 2 Vitest, 12 Rust integration, production build, 24 Playwright;
  2 intentional project skips.
- `npm run check` and a locked release build: pass.
- All nine commands in `.factory/claims.json`: pass when run separately.
- Clean installed CLI workflow: pass for normal, invalid, boundary, recovery,
  receipt, and restart paths.
- Live first screen, one-click four-row demo, reset, real-data isolation,
  downloads, offline reload, error recovery, keyboard focus, 200% text, fonts,
  security headers, and standard routes: pass.
- Axe serious/critical findings: 0. Lighthouse: 100/100/100/100; LCP 1.38 s.
- Live rate limits: general 44 accepted/106 limited; license proxy 10
  accepted/20 limited; every 429 had `Retry-After` and both recovered.
- Deployment: one replica and durable `/data`; an existing receipt kept its
  logical hash and events across a real restart.

Live `/health` reports documentation SHA `5326caf...`. The implementation is
`e26a1fd...`; the intervening commits change only this handoff document. Live
frontend artifacts byte-match the clean implementation build.

## Findings to repair

1. Register the Plus product so the advertised $29 checkout no longer returns
   404.
2. Make `RETENTION_DAYS` remove obsolete signed registration evidence, or
   disclose a separate indefinite retention rule, and add an outcome test.
3. Add declared, tagged tests for the five public promises listed in the report.
4. Remove `demo:run-proof:last-ledger` when **Start for real** is chosen.
5. Show invalid/revoked returned-license notices outside the hidden restore
   form.
6. Remove or replace the three dead sample GitHub source links.
7. Complete 404 metadata and the standard footer details.
8. Complete `.factory/copy-audit.md` with every visible landing string.

## Evidence

- Required report copy: `/work/.evidence/qa-report.md`
- Required verdict JSON: `/work/.evidence/qa-result.json`
- Desktop/phone screenshots: `/work/.evidence/run-proof-v4-desktop.png` and
  `/work/.evidence/run-proof-v4-phone.png`
- Lighthouse JSON: `/work/.evidence/run-proof-v4-lighthouse.json`
- Factory URL verification: `/work/.evidence/run-proof-v4-verify-url/`

No product code was changed. Only verification documentation is included in the
report commit.
