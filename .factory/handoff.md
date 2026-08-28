# Run Proof handoff

Work order: `job-liveness-proof-repair-1`
Repair commit: `5669689b9910a2b37c5efe1a433019754bd4dc0f`

## Repair

The failed `ad11f071d046f64fbc53f14f02d835761273b416` revision was reproduced with an otherwise empty environment containing only `PORT`: `run-proof-server` exited before binding because `src/main.rs` required `RUN_PROOF_SECRET` and panicked with `RUN_PROOF_SECRET must be set to a strong random value: NotPresent`. Azure also reported the deployed revision as `ActivationFailed` with only `PORT=8080` configured.

The server now starts with only `PORT` (default 8080), binds `0.0.0.0:$PORT`, serves the built product at `/`, and responds at `/health`. If `RUN_PROOF_SECRET` is absent, first boot obtains 32 CSPRNG bytes, hex-encodes them, and persists the secret as mode `0600` next to SQLite (`/data/run-proof.secret` in the image); future starts reuse it. Supplied secrets still override the generated value. One structured startup line records whether database, key ID, and secret were supplied, defaulted, generated, or persisted without logging a secret.

The Docker image now has no runtime configuration `ENV` values. The full deployment SHA is baked into the server binary from Docker `BUILD_SHA`, so `/health` cannot depend on an additional runtime variable. The frontend service-worker cache version was advanced and now precaches the stable JS/CSS shell, including a browser regression test for offline reload.

## Build, test, and verification

- Reproduction: base commit exited with the missing-secret panic above when run with `env -i PORT=18080`.
- Focused Rust integration regression: real `run-proof-server` launched with `env_clear()` plus only `PORT`; `/health` and `/` returned `200`, the generated secret was private, and a restart reused it.
- `npm test`: passed — 2 frontend unit tests, 4 Rust integration tests, production Vite build, and 9 Playwright checks (one desktop-only overflow assertion skipped in the mobile project as intended).
- `npm run check`: passed — TypeScript and Clippy with warnings denied.
- `cargo build --locked --release --bin run-proof-server --bin run-proof`: passed.
- `RUN_PROOF_URL=http://127.0.0.1:18081 ./scripts/load-smoke.sh`: passed — 100/100 health requests.
- Playwright covers title/landmark/one-h1/console smoke, keyboard skip-link navigation, desktop and 390×844 mobile, Axe serious/critical checks in light and dark modes, legal pages, cached offline reload, and no third-party requests on the privacy page.
- Clean registry build command: `az acr build --registry sociobotregistry --image sf-job-liveness-proof:5669689b9910a2b37c5efe1a433019754bd4dc0f --file Dockerfile --build-arg BUILD_SHA=5669689b9910a2b37c5efe1a433019754bd4dc0f --timeout 1800 --no-wait --no-logs .`

## Run and deploy

```sh
npm ci
npm test
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t run-proof .
docker run --rm -e PORT=8080 -p 8080:8080 run-proof
```

The factory deployment configuration remains an external Azure Container App with target port 8080 and only `PORT=8080` set. The clean ACR build `cha0` succeeded, and image `sociobotregistry.azurecr.io/sf-job-liveness-proof:5669689b9910a2b37c5efe1a433019754bd4dc0f` is deployed as revision `sf-job-liveness-proof--0000001` (Running/Healthy).

Live probes completed at 2026-08-28T03:16Z:

- `https://sf-job-liveness-proof.orangepond-1638693f.eastus2.azurecontainerapps.io/` → `200`; `/health` → `{"build_sha":"5669689b9910a2b37c5efe1a433019754bd4dc0f","status":"ok"}`.
- `https://job-liveness-proof.sociobot.in/` → `200`; `/health` → `{"build_sha":"5669689b9910a2b37c5efe1a433019754bd4dc0f","status":"ok"}`.
- Revision startup logs show `database=defaulted`, `secret=generated`, `key_id=defaulted`, followed by `Run Proof listening` on port 8080; secret material was not logged.

## Known gaps

Container App has no mounted Azure Files volume in its current factory configuration. `/data` is still the correct persistent path for deployments that mount storage; without a volume, generated signing state survives process restarts only for the lifecycle of that revision. A durable mount is a factory infrastructure choice and was not changed here.
