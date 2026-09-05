# Run Proof repair handoff

- Work order: `job-liveness-proof-repair-4`
- Implementation SHA: `e26a1fd14a84d1472b0b2f1d4aff1435bdae1041`
- Documentation SHA: `f8e17dbd76d77d2bc5263b9efc518c1ec5d9ad12`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-09-05 UTC

## What changed

- Added immutable `job_registrations` versions and bound each event and CI
  snapshot to the schedule registration in force for that run.
  Re-registering a job now leaves completed receipt registration evidence and
  receipt hash unchanged.
- Added `/demo` and `GET /api/v1/demo/ledger`. The endpoint generates four
  realistic rows in memory and does not read or write SQLite. The browser uses
  the separate `demo:run-proof:last-ledger` storage key.
- Added the first-screen sample action, persistent demo banner, reset control,
  real-data exit, local sample CSV/receipt downloads, and plain cold-start copy.
- Added `.factory/claims.json`, exactly one tagged outcome test for every
  listed claim, `.factory/demo.md`, and the landing copy audit.
- Completed the public site work: self-hosted fonts now render, route titles
  and focus announcements update, the footer shows Param Factory and build ID,
  all required metadata is present, unknown browser paths serve a designed 404,
  and Vite-hashed assets receive immutable caching.
- Updated Docker to use `rust:1-slim`, accepted factory build identity args,
  and kept its non-root `/data` runtime contract.
- Added the catalog description and copied it to
  `/work/.evidence/catalog-description.txt`.

## Deployment and durability

The ACR implementation build `ch250` succeeded. The local deployment wrapper
timed out while its remote build was still running, so the same repository
patch was applied directly after the successful build:

- image: `sociobotregistry.azurecr.io/sf-job-liveness-proof:e26a1fd14a84`
- durable Azure Files mount: `data-job-liveness-proof` at `/data`
- replica bounds: `minReplicas=1`, `maxReplicas=1`

`npm run verify:deployment -- e26a1fd14a84d1472b0b2f1d4aff1435bdae1041`
passed before and after a real restart of revision
`sf-job-liveness-proof--0000014`. Its post-restart product log reported
`secret=persisted`; no secret value was logged.

## Verification

- `npm test`: pass — Vitest 2/2, Rust integration 12/12, production build,
  Playwright 24 passed and 2 intentional mobile/desktop skips.
- `npm run check`: pass — TypeScript, rustfmt, and Clippy with warnings denied.
- Every command in `.factory/claims.json`: pass from the documented setup.
- Clean consumer install: the installed CLI registered, started, finished with
  count `0`, added a failed CI observation, and exported a receipt from an
  isolated local receiver. Both installed binaries and the persisted consumer
  SQLite artifact were checked after clean shutdown.
- Live HTTPS: `/health` reports the implementation SHA; `/demo` returns one
  each of contradictory, missed, late, and completed sample rows; unknown pages
  return the designed HTTP 404; hashed font responses are immutable.
- Fresh live desktop and phone browser checks found no console errors or
  overflow. Before scrolling, both clearly state the job, audience, and first
  action: **Track scheduled jobs that ran**; small app teams running cron jobs
  and queue workers; **Try it with sample data**. Screenshots are at
  `/work/.evidence/run-proof-desktop.png` and
  `/work/.evidence/run-proof-phone.png`.
- Live Axe WCAG A/AA serious/critical findings: none on `/`, `/demo`,
  `/privacy`, or `/terms` at phone width.
- Live general limiter burst: 157 `200`, 93 `429`; live license-proxy burst:
  14 `200`, 6 `429`. Follow-up responses included `Retry-After: 1`.

## Prior findings disposition

The earlier cross-job receipt scoping, finish-before-start rendering,
verifiable signed receipt material, safe forwarded-IP rate limiting, PWA
updates, touch targets, header policy, and SQLite network-mount behavior remain
covered by the passing integration/browser suite. The latest verification's
four blockers and its font, routing, metadata, footer, focus, and cache
findings are repaired above.

## Known gap

Run Proof Plus remains a paid one-time deliverable. The free core works.
The live checkout endpoint returned `404` with `enabled factory product` during
this repair, so factory billing registration is still required before a buyer
can complete checkout. The public offer metadata is in
`/work/.evidence/billing-offer.json`; the page and README name the dependency
plainly. No payment credentials were added.
