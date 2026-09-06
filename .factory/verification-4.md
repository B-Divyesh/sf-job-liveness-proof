# Verify scheduled jobs that ran — verification 4

- Work order: `job-liveness-proof-verify-4`
- Implementation reviewed: `e26a1fd14a84d1472b0b2f1d4aff1435bdae1041`
- Documentation commit: `5326cafee0868d6bb11daccfbb4e65013568f3d1`
- Live URL: <https://job-liveness-proof.sociobot.in>
- Verified: 2026-09-06 UTC
- Verdict: **FAIL**
- Findings: **8**
- Untested public claims: **5**

Run Proof's free core works end to end. The release fails the required
zero-finding gate because its advertised paid checkout is unavailable, its
retention behavior leaves expired signed registration evidence behind, and
five public promises are not covered by complete declared claim tests.

## First screen

Fresh 1440×900 desktop and 390×844 phone contexts showed all three required
facts before scrolling:

- Job: **Track scheduled jobs that ran**.
- Audience: small app teams running cron jobs and queue workers.
- First action: **Try it with sample data**, followed by “Opens a populated
  ledger”.

Both views had one `h1`, one `main`, `lang="en"`, no horizontal overflow, and
no console, page, or request errors. Screenshots are
`/work/.evidence/run-proof-v4-desktop.png` and
`/work/.evidence/run-proof-v4-phone.png`.

## Findings

### High — H1: the advertised paid checkout returns 404

The landing page advertises “Free core; $29 one time for saved views” and
offers **Buy Run Proof Plus**. Its link returned HTTP 404 with the billing
engine's `enabled factory product` error. A visitor cannot buy the advertised
feature. This is the known billing-registration gap, but it is still a live,
broken public path and fails the paid-unlock and no-dead-links contracts.

The free ingest, ledger, receipts, and CSV export remain usable.

### High — H2: retention leaves expired signed registration evidence behind

The privacy page says, “Expired evidence is removed locally when the server
starts.” The declared `retention-setting` test checks only that `/api/v1/config`
reports the configured number; it does not test deletion.

In an isolated installed server, I created one job, two immutable registration
versions, two run events, and one CI snapshot. I aged all records beyond a
one-day retention window and restarted with `RETENTION_DAYS=1`. The restart
removed both events and the snapshot, but retained the job and both historical
`job_registrations`, including both exact signed request bodies. The cleanup
function deletes only `events` and `ci_snapshots`.

This is incomplete implementation and test coverage for a brief-level privacy
requirement. Either define and disclose that registration evidence is retained
indefinitely, or apply the configured policy to obsolete registration evidence
and test the observable deletion outcome.

### High — H3: five public claims lack complete declared tests

Every command currently listed in `.factory/claims.json` passes, and each listed
ID appears on exactly one test. Cross-checking the live copy and README found
five additional or incompletely tested promises:

1. A **$29 one-time Plus license for saved operational views**. There is no
   claim entry or valid-license fixture flow, and checkout currently fails.
2. Plus verification happens **at most once per day**. The browser implements a
   cache, but no declared claim test asserts the one-day behavior.
3. **Expired evidence is removed when the server starts**. The existing claim
   test checks only configuration, and the outcome is incomplete as H2 shows.
4. **Duplicate start or finish records return 409**. This is documented but is
   absent from the claim registry and tagged suite.
5. **Out-of-window timestamps return 400**. This is documented but is absent
   from the claim registry and tagged suite.

Manual checks are not a substitute for the required repeatable claim commands.
The untested claim count is therefore 5.

### Medium — M1: leaving the demo keeps its sample cache

The demo uses the separate `demo:run-proof:last-ledger` namespace and never
changed the production ledger or its browser cache. Reset also worked and kept
the persistent demo banner visible. However, choosing **Start for real** left
`demo:run-proof:last-ledger` in local storage. The demo-sandbox contract says
leaving demo mode discards demo data unless the user is offered an explicit
keep action. Neither happens.

### Medium — M2: an invalid returned license notice is hidden

A fresh visit with an invalid returned license correctly removed the token from
the URL, stored it under the required product key, made one same-origin proxy
request, cached the invalid verdict, and kept Plus locked. The text “This
license is no longer active” was written inside the still-hidden restore form,
so it had zero layout size and was not visible. On reload, the daily cache
prevented another request but no notice was rendered at all. The paid-unlock
contract requires a quiet visible notice with the buy link when verification is
invalid.

### Low — L1: all three demo source links are dead

The populated sample links its three GitHub Actions source observations to
`github.com/acme/.../runs/demo`. All three links returned HTTP 404. They are
prominent “Source view” evidence links in the required demo, so they violate the
site's no-dead-links rule. Use working neutral examples or render sample source
labels without outbound links.

### Low — L2: the designed 404 omits required site metadata and footer details

`/not-a-route` correctly returns an intentional HTTP 404 and a usable designed
page; the 404 itself is not a defect. The page has its own title, one `h1`, one
`main`, and a route home. It lacks the required meta description, canonical,
Open Graph and Twitter metadata, apple-touch link, footer Privacy/Terms links,
and build identifier used by the standard site skeleton.

### Low — L3: the required landing copy audit is incomplete

`.factory/copy-audit.md` reports that it extracted every landing sentence, but
it omits visible landing copy such as the primary and secondary actions,
section headings, ledger states, empty/error text, purchase actions, and footer
copy. The visible copy reviewed in this verification is plain and no banned
word was found; the issue is that the mandatory audit artifact does not support
its completeness claim.

## Declared claim commands

All nine commands were run from a detached clean checkout of the implementation
candidate after `npm ci`:

| Claim | Result | Evidence |
| --- | --- | --- |
| `demo-ledger` | Pass | Desktop and mobile each showed four sample rows, reset, banner, and isolated production cache. |
| `csv-export` | Pass | Download contained the header plus four sample rows. |
| `sample-receipt` | Pass | Download was `run-proof-demo-receipt/v1` for `billing-sweep`. |
| `offline-reload` | Pass | Dedicated contexts reloaded four demo rows offline. |
| `no-tracking` | Pass | Demo load, reset, and export made only same-origin requests. |
| `signed-evidence` | Pass | Receipt HMAC material recomputed in the Rust integration test. |
| `no-job-payloads` | Pass | Correctly signed unknown payload input was rejected. |
| `receipt-history` | Pass | Re-registration preserved original schedule data and receipt hash. |
| `retention-setting` | Command passes, claim incomplete | Test asserts the reported setting, not retention behavior; see H2. |

## Clean checkout and installed artifact

- `npm ci`: pass; 60 packages installed, 0 audit vulnerabilities.
- `npm test`: pass — Vitest 2/2, Rust integration 12/12, production build,
  Playwright 24 passed with 2 intentional project skips.
- `npm run check`: pass — TypeScript, rustfmt, and Clippy with warnings denied.
- `npm run build`: pass; `dist/` produced.
- `BUILD_SHA=e26a1fd... cargo build --locked --release --bins`: pass.
- Every declared claim command: pass as listed above.
- `cargo install --locked --path . --root <isolated-prefix> --bins --force`:
  pass; both `run-proof` and `run-proof-server` were installed.

The installed CLI and installed server completed register → start → finish with
count `0` → failed CI snapshot → contradictory ledger → receipt export. An
interval below the minimum, a duplicate start, a negative count, and an `ftp:`
source were rejected. A maximum interval/grace registration
(`31536000`/`86400`) passed, and valid writes succeeded after the errors.
Restarting the isolated receiver preserved its rows. A fresh database rendered
“The ledger is ready” and **Connect the first job**.

No Docker-compatible executable was available in the verifier container. The
locked frontend and release backend stages passed directly, and the matching
live artifact proves the image build completed.

## Live demo, routes, accessibility, and privacy

- The sample contained one contradictory, missed, late, and completed run with
  realistic names and counts. The persistent banner, reset, receipt download,
  and CSV download worked on phone and desktop.
- Production ledger rows and `run-proof:last-ledger` remained unchanged through
  both demo flows. The backend demo endpoint is in-memory and separate from
  SQLite.
- Normal, empty, no-match, invalid-input, and network-error recovery paths
  passed. Aborting the first ledger request showed **Could not reach the
  ledger**; **Try again** recovered to **Receiver connected**.
- `/`, `/demo`, `/privacy`, and `/terms` returned 200 with route-specific
  titles. `/not-a-route` returned the expected designed 404. Unknown API paths
  returned structured `404 application/json`.
- Fresh keyboard navigation focused **Skip to content** with a visible 3 px
  amber outline. In-app Privacy navigation and browser Back focused and
  announced the new `h1`. No keyboard trap or undersized visible action was
  found.
- At 390 px and 200% root text size, `scrollWidth` equaled `clientWidth`.
  Reduced-motion mode computed `scroll-behavior: auto`.
- Axe WCAG A/AA found zero serious or critical violations in light and dark
  treatment across the home, demo, legal, and 404 pages. The factory
  `verify-url.sh` passed with one `h1`, one main landmark, alt text, and no
  console errors.
- The self-hosted Bitter and Atkinson Hyperlegible Next variable faces loaded.
  No analytics, CDN scripts/fonts, cookies, or unexpected third-party requests
  appeared in ordinary or demo flows. An untrusted Origin received no CORS
  allowance.
- Offline demo reload retained the banner, four rows, and main landmark. The
  service worker uses cache `run-proof-shell-v4`; stable app assets revalidate,
  while hashed fonts are immutable.
- Security responses included CSP, HSTS, `nosniff`, no-referrer, and a
  restrictive Permissions-Policy.

## Backend and deployment

- Local `/health`-only startup with no required configuration passed in the
  integration suite. Generated signing material was mode `0600` and reused on
  restart.
- Live `/health` reports
  `5326cafee0868d6bb11daccfbb4e65013568f3d1`, the documentation commit. The
  only difference from implementation `e26a1fd...` is `.factory/handoff.md`.
  Live `index.html`, app JS, app CSS, service worker, and manifest byte-match the
  clean implementation build, so this is accepted as a later report-only image
  identity rather than a product mismatch.
- `npm run verify:deployment -- 5326caf...` passed before and after a real
  restart: one replica and the product-owned Azure Files mount at `/data`.
- An existing live receipt retained the same logical receipt hash and two
  events across that restart. No new production ledger data was written.
- General live burst: 44×`200`, 106×`429`; license-proxy burst: 10×`200`,
  20×`429`. Every 429 carried `Retry-After`; both allowances recovered to 200.
- Live `/api/v1/config` reports 30-day retention, 300-second clock skew, and
  `payload_storage:false`. Health remains exempt from limiting as allowed.

## Performance

Lighthouse 12.8.2 mobile results:

- Performance 100, Accessibility 100, Best Practices 100, SEO 100.
- FCP 1.23 s, LCP 1.38 s, TBT 27 ms, CLS 0, Speed Index 1.23 s.
- Total transfer 170,612 bytes; no binary audit failed.
- Candidate output: JS 20.85 KB (7.82 KB gzip), CSS 19.41 KB (5.47 KB gzip),
  mobile hero 19.78 KB, desktop hero 59.48 KB.

The Lighthouse JSON is `/work/.evidence/run-proof-v4-lighthouse.json`.

## Earlier finding disposition

| Earlier finding | Current disposition |
| --- | --- |
| Ephemeral multi-replica ledger/signing key | Fixed: one replica, `/data` mount, receipt survived real restart. |
| Receipts mixed jobs sharing a run ID | Fixed: scoped receipt integration test passes. |
| Finish-before-start crashed ledger | Fixed: API and browser tests render the row. |
| Receipt lacked verifiable signed material | Fixed: exact bodies/timestamps/signatures export and HMAC test passes. |
| 200% text overflow | Fixed at 390 px. |
| Unsafe service-worker update caching | Fixed: stable assets revalidate and shell cache is version 4. |
| Bypassable or incomplete rate limiting | Fixed locally and live, including forwarded IP and `Retry-After`. |
| Unknown API route returned SPA HTML | Fixed: structured JSON 404. |
| Small touch targets, missing PWA icons/security headers, rustfmt failure | Fixed. |
| Missing claim registry and one-click demo | Partly fixed: both exist and declared commands run, but H3 remains. |
| Mutable historical registration | Fixed: receipt history claim passes. |
| Fonts not applied | Fixed: both self-hosted faces render. |
| Missing SPA metadata/footer/focus/cache behavior | Fixed on standard routes; 404 residue remains in L2. |
| Live storage/replica topology | Fixed and freshly proven across restart. |

## Final disposition

**FAIL. Do not promote.** Resolve all eight findings, add complete tagged claim
coverage for the five untested public promises, redeploy the product changes,
and run a fresh independent verification. The deliberate HTTP 404 for an
unknown route was expected and was not counted as a defect.
