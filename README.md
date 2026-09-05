# Run Proof

Track scheduled jobs that ran.

Run Proof is for small app teams with cron jobs and queue workers.
It records schedule, start, finish, and CI observations.
The ledger shows missed, late, failed, and conflicting runs.

Try the isolated sample at `/demo`.
It shows a populated ledger without reading or writing your data.

## What it does

- Receives HMAC-signed job registration, start, finish, and CI records.
- Stores evidence in SQLite and keeps exact signed request bytes.
- Exports each run as a JSON receipt and the ledger as CSV.
- Keeps completed receipts tied to their original schedule registration.
- Retains no job payload fields.
- Shows a last-known ledger after an offline reload.

It does not execute jobs, route alerts, process queues, or replace CI.

## Run locally

Install Node 22+, Rust 1.88+, and SQLite development libraries.

```sh
npm ci
npm run build
cargo run --bin run-proof-server
```

Open <http://localhost:8080>.
Use `npm run dev` in another terminal for frontend hot reload.

## Try the demo

Open <http://localhost:8080/demo>.
The demo has four realistic job records.
It uses `GET /api/v1/demo/ledger`, which creates sample data in memory.
It never reads or writes the production SQLite ledger.

The banner stays visible in demo mode.
Choose **Reset demo** to clear `demo:run-proof:last-ledger` and reload samples.
Choose **Start for real** to return to the real ledger.
See [`.factory/demo.md`](.factory/demo.md) for the full sandbox contract.

## Connect a job

Build the CLI with `cargo build --release --bin run-proof`.
Set a receiver URL and shared secret first.

```sh
export RUN_PROOF_URL='http://localhost:8080'
export RUN_PROOF_SECRET='replace-with-at-least-32-random-characters'

run-proof register billing-sweep --name 'Billing sweep' --every 3600 --grace 300
run-proof start billing-sweep billing-1700000000 --scheduled '2026-08-28T01:00:00Z'
run-proof finish billing-sweep billing-1700000000 --status success --count 428
run-proof snapshot billing-sweep billing-1700000000 \
  --source 'GitHub Actions' --status failed --source-url 'https://github.com/example/actions/runs/1'
```

The CLI signs `unix_timestamp + "." + exact_JSON_body` with HMAC-SHA256.
Receipts include those exact signed bytes, timestamp, key ID, and signature.
You can recompute every HMAC independently.
Duplicate start or finish records return `409`.
Out-of-window timestamps return `400`.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8080` | HTTP listen port |
| `DATABASE_URL` | local SQLite | SQLite URL; mount its directory durably |
| `RUN_PROOF_SECRET` | generated | Shared signing secret; at least 32 characters |
| `RUN_PROOF_KEY_ID` | `default` | Key identifier expected from senders |
| `RETENTION_DAYS` | `30` | Evidence retention; 1–3650 days |
| `CLOCK_SKEW_SECONDS` | `300` | Accepted clock difference; 30–3600 seconds |

The default container uses `/data/run-proof.db` when `/data` exists.
It also stores its generated secret under `/data`.
Mount `/data` on durable storage.
Use exactly one replica with SQLite.
Use PostgreSQL before horizontal scaling.

## Test and build

```sh
npm test
npm run build
npm run check
RUN_PROOF_URL=http://localhost:8080 ./scripts/load-smoke.sh
```

Run each public claim from a clean checkout with the command in
[`.factory/claims.json`](.factory/claims.json).

The release command is `scripts/deploy-container.sh <full-commit-sha>`.
It builds the image and reasserts the Azure Files `/data` mount.
It also sets both replica bounds to one.

```sh
npm run verify:deployment -- <full-commit-sha>
```

## Privacy and purchase

Run Proof has no analytics, tracking, third-party fonts, or runtime CDN scripts.
The browser stores a last-known ledger, an optional saved view, and a license token.
See `/privacy` and `/terms` in the running app.

Run Proof Plus is a $29 one-time license for saved operational views.
Core ingest and exports stay free.
Sociobot/Dodo is the merchant of record.
The currently advertised checkout needs factory billing registration before purchase works.

The original paper-desk artwork was generated for this repository.
See [`.factory/design.md`](.factory/design.md) for its prompt and provenance.

## License

MIT — see [`LICENSE`](LICENSE).
