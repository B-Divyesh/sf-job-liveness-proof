# Run Proof

Run Proof is a compact, self-hosted evidence ledger for cron jobs and queue workers. It records signed schedule, start, finish, completion-count, and CI-source observations, then calls out missed, late, failed, and contradictory runs on one page. It is for small application teams who need stronger evidence than URL uptime or a green status rollup.

It does **not** execute jobs, retain job payloads, replace CI, or route alerts.

## What ships

- Rust/axum receiver with SQLite, HMAC-SHA256 verification, clock-skew checks, validation, rate limiting, structured logs, and graceful shutdown.
- `run-proof` CLI for job registration, start/finish receipts, and CI snapshots.
- Responsive ledger with per-run JSON receipts, full CSV export, last-known offline view, and explicit empty/error/offline states.
- Configurable retention and no application payload field in any ingest schema.
- Run Proof Plus, a $29 one-time license that unlocks a saved operational view and supports maintenance. Core evidence, retention configuration, and all exports remain free.

## Run locally

Prerequisites: Node 22+, Rust 1.88+, and SQLite development libraries.

```sh
npm ci
npm run build
cargo run --bin run-proof-server
```

Open <http://localhost:8080>. For frontend hot reload, keep the server running and use `npm run dev` in another terminal; Vite proxies `/api` and `/health` to port 8080.

## Connect a job

Build the CLI with `cargo build --release --bin run-proof`, or use the CLI copied into the container:

```sh
export RUN_PROOF_URL='http://localhost:8080'
export RUN_PROOF_SECRET='replace-with-at-least-32-random-characters'

run-proof register billing-sweep --name 'Billing sweep' --every 3600 --grace 300
run-proof start billing-sweep billing-1700000000 --scheduled '2026-08-28T01:00:00Z'
run-proof finish billing-sweep billing-1700000000 --status success --count 428
run-proof snapshot billing-sweep billing-1700000000 \
  --source 'GitHub Actions' --status failed --source-url 'https://github.com/example/actions/runs/1'
```

The CLI signs `unix_timestamp + "." + exact_JSON_body` with HMAC-SHA256 and sends `X-Run-Proof-Key`, `X-Run-Proof-Timestamp`, and `X-Run-Proof-Signature: v1=<hex>`. Run receipts are scoped by both job and run (`/api/v1/jobs/:job_key/runs/:run_id/receipt`) and retain those exact signed bytes, timestamp, key ID, and signature so the HMAC can be independently recomputed. Duplicate start/finish events return `409`; timestamps outside `CLOCK_SKEW_SECONDS` return `400`.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8080` | HTTP listen port |
| `DATABASE_URL` | `sqlite://run-proof.db?mode=rwc` | SQLite URL; mount its directory persistently |
| `RUN_PROOF_SECRET` | generated and persisted | Shared signing secret, minimum 32 characters; generated with the OS CSPRNG on first boot when unset |
| `RUN_PROOF_KEY_ID` | `default` | Public key identifier expected from senders |
| `RETENTION_DAYS` | `30` | Evidence retention, 1–3650 days; cleanup runs at startup |
| `CLOCK_SKEW_SECONDS` | `300` | Accepted clock difference, 30–3600 seconds |
| `RUST_LOG` | receiver defaults | `tracing` filter for JSON logs |

Back up SQLite before upgrades. Rotate a compromised secret on the receiver and all senders together. Run behind TLS in production. Production must mount the database directory durably. Because SQLite is a single-node datastore, run exactly one application replica; use PostgreSQL instead before scaling horizontally.

## Test and build

```sh
npm test              # web + Rust tests
npm run build         # reproducible frontend output in dist/
npm run check         # TypeScript + clippy
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t run-proof .
RUN_PROOF_URL=http://localhost:8080 ./scripts/load-smoke.sh
```

The load smoke sends 100 concurrent health requests. Integration tests cover a full signed job → start → finish → contradictory CI observation → receipt path, invalid signatures, rejected payload fields, and health metadata.

## Container deployment

```sh
docker run --rm -p 8080:8080 \
  -v run-proof-data:/data run-proof
```

The image runs as UID/GID `10001`, starts with only `PORT` (default `8080`), serves frontend and API on `0.0.0.0:$PORT`, and stores SQLite plus a generated signing secret under `/data`. Mount `/data` on durable storage and keep the deployment at one replica; the generated secret is atomically created and reused across restarts and rolling revisions. Build it with the full commit SHA command above so `/health` reports the deployed commit. The factory owns infrastructure, DNS, TLS, and product registration.

## Privacy, purchases, and license

No analytics, tracking, third-party fonts, or runtime CDN scripts are included. The browser stores only a last-known ledger, optional saved view, and optional Sociobot license. Checkout and once-daily verification use the Sociobot billing API; Dodo/Sociobot is merchant of record. See `/privacy` and `/terms` in the running app.

Artwork was generated for this repository using the factory image model. Its prompt, checksum, and provenance are in [`.factory/design.md`](.factory/design.md) and [`assets/src/run-proof-diorama.json`](assets/src/run-proof-diorama.json).

## License

MIT — see [`LICENSE`](LICENSE).
