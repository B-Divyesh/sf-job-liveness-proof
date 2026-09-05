# Run Proof demo sandbox

- URL: `https://job-liveness-proof.sociobot.in/demo` (or choose **Try it with sample data** from the landing page).
- Sample: four realistic background-job rows: a contradictory billing sweep, a missed inventory sync, a late digest mailer, and a completed invoice export.
- Isolation: `GET /api/v1/demo/ledger` generates the sample in memory. It does not query or write SQLite. The browser keeps only `demo:run-proof:last-ledger`; production data uses `run-proof:last-ledger`.
- Reset: **Reset demo** clears the demo cache, fetches a new in-memory sample, and leaves production data untouched.
- Leave: **Start for real** returns to the real ledger and discards no production data. The demo receipt and CSV downloads are locally generated sample files.
- Offline: after one online demo visit, the service worker and `demo:` cache allow an offline reload.
