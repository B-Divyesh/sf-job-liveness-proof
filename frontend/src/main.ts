import '@fontsource-variable/bitter/wght.css';
import '@fontsource-variable/atkinson-hyperlegible-next/wght.css';
import './styles.css';
import { contradictionLabel, filterRows, states, type Ledger, type LedgerRow } from './model';

const app = document.querySelector<HTMLDivElement>('#app')!;
const slug = 'job-liveness-proof';
const licenseKey = `sb_license:${slug}`;
const realCacheKey = 'run-proof:last-ledger';
const demoCacheKey = 'demo:run-proof:last-ledger';
let ledger: Ledger | null = null;
let selectedState = 'all';
let query = '';
let offline = false;
let demo = false;

const icon = `<svg aria-hidden="true" viewBox="0 0 48 48"><path d="M8 7h26l6 6v28H8z"/><path d="M34 7v7h7M15 22h18M15 29h12"/><circle cx="34" cy="33" r="8"/><path d="m30 33 3 3 6-7"/></svg>`;

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>'"]/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' }[character]!));
}

function formatTime(value: string | null): string {
  if (!value) return '—';
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? 'Unknown' : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
}

function relative(value: string | null): string {
  if (!value) return 'Schedule not received';
  const milliseconds = new Date(value).valueOf();
  if (!Number.isFinite(milliseconds)) return 'Schedule unavailable';
  const seconds = Math.round((milliseconds - Date.now()) / 1000);
  const absolute = Math.abs(seconds);
  const [amount, unit] = absolute < 60 ? [absolute, 'second'] : absolute < 3600 ? [Math.round(absolute / 60), 'minute'] : absolute < 86400 ? [Math.round(absolute / 3600), 'hour'] : [Math.round(absolute / 86400), 'day'];
  return new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' }).format(seconds < 0 ? -Number(amount) : Number(amount), unit as Intl.RelativeTimeFormatUnit);
}

function setRouteMetadata(path: string, title: string, description: string): void {
  document.title = title;
  const origin = location.origin;
  document.querySelector<HTMLLinkElement>('link[rel="canonical"]')?.setAttribute('href', `${origin}${path}`);
  document.querySelector<HTMLMetaElement>('meta[name="description"]')?.setAttribute('content', description);
  document.querySelector<HTMLMetaElement>('meta[property="og:title"]')?.setAttribute('content', title);
  document.querySelector<HTMLMetaElement>('meta[property="og:description"]')?.setAttribute('content', description);
  document.querySelector<HTMLMetaElement>('meta[name="twitter:title"]')?.setAttribute('content', title);
  document.querySelector<HTMLMetaElement>('meta[name="twitter:description"]')?.setAttribute('content', description);
}

function shell(content: string): string {
  return `<a class="skip-link" href="#main">Skip to content</a>
    <header class="masthead"><a class="brand" href="/" data-route aria-label="Run Proof home">${icon}<span>Run Proof</span></a>
      <nav aria-label="Primary"><a href="/demo" data-route>Demo</a><a href="/#ledger">Ledger</a><a href="/#setup">Set up</a><a href="/privacy" data-route>Privacy</a></nav>
    </header>${content}
    <footer><span>Run Proof records job evidence for small application teams.</span><span><a href="/privacy" data-route>Privacy</a><a href="/terms" data-route>Terms</a><span>Built by Param Factory</span><span id="build-id">Build: checking</span></span></footer>
    <p id="route-announcement" class="visually-hidden" aria-live="polite" aria-atomic="true"></p>`;
}

function demoBanner(): string {
  return `<aside class="demo-banner" aria-label="Demo status"><strong>Demo — sample data, nothing is saved</strong><span>This view never reads or writes your ledger.</span><button id="reset-demo" type="button">Reset demo</button><a href="/" data-route>Start for real</a></aside>`;
}

function renderLegal(kind: 'privacy' | 'terms'): void {
  const title = kind === 'privacy' ? 'Privacy — Run Proof' : 'Terms — Run Proof';
  setRouteMetadata(`/${kind}`, title, kind === 'privacy' ? 'Read how Run Proof stores job evidence and browser data.' : 'Read the terms for Run Proof and its one-time license.');
  const privacy = `<h1>Privacy</h1><p class="lede">Run Proof stores execution evidence, not the contents of your jobs.</p><h2>Data this server stores</h2><p>It stores job names and keys, run IDs, run times, terminal status, completion counts, CI observations, source links, and signed request evidence.</p><p>Ingest rejects unknown fields. Job payloads are not stored.</p><h2>Where it lives</h2><p>Your self-hosted SQLite database holds the ledger. This app has no analytics, advertising, tracking pixels, or third-party runtime scripts.</p><p>Your browser stores its last ledger for offline reading and an optional Sociobot license token.</p><h2>Control and deletion</h2><p>The operator sets <code>RETENTION_DAYS</code>. Expired evidence is removed locally when the server starts.</p><p>Clear this origin’s site data to remove the browser cache and license.</p><h2>Purchase verification</h2><p>Plus verification relays a license token to Sociobot at most once per day. The token is not written to the ledger.</p>`;
  const terms = `<h1>Terms</h1><p class="lede">Run Proof records signals supplied by your systems. It helps investigate work.</p><p>It does not execute jobs or guarantee their results.</p><h2>Use and responsibility</h2><p>You safeguard the signing secret, choose schedules and grace periods, back up SQLite, and decide how to act on the evidence.</p><h2>One-time purchase</h2><p>Run Proof Plus is a one-time $29 license for saved operational views in one self-hosted deployment.</p><p>Sociobot/Dodo is the merchant of record and handles payment and refunds. Core ingest and exports remain available.</p><h2>No warranty</h2><p>The software is provided as is. It is not an alert router, queue processor, CI replacement, or sole safety control.</p>`;
  app.innerHTML = shell(`<main id="main" class="legal"><a class="back" href="/" data-route>Back to the ledger</a>${kind === 'privacy' ? privacy : terms}<p class="updated">Effective 5 September 2026</p></main>`);
  bindRouteLinks();
  void loadBuildIdentity();
}

function renderApp(isDemo: boolean): void {
  demo = isDemo;
  const path = demo ? '/demo' : '/';
  setRouteMetadata(path, demo ? 'Demo — Run Proof' : 'Run Proof — track scheduled jobs that ran', demo ? 'Try a populated ledger for cron jobs and queue workers.' : 'Track missed, late, and contradictory cron and queue job runs.');
  const banner = demo ? demoBanner() : '';
  app.innerHTML = shell(`${banner}<main id="main">
    <section class="hero" aria-labelledby="page-title"><div class="hero-copy"><p class="eyebrow"><span></span>Signed job evidence</p><h1 id="page-title">Track scheduled jobs that ran</h1><p class="lede">For small app teams running cron jobs and queue workers. See missed, late, and conflicting runs in one ledger.</p><div class="hero-actions">${demo ? '<a class="button primary" href="#ledger">Read the sample ledger</a>' : '<a class="button primary" href="/demo" data-route>Try it with sample data</a><span class="action-note">Opens a populated ledger</span>'}<a class="button secondary" href="#setup">Connect a job</a></div><ul class="trust-line"><li>Signed event records</li><li>No job payloads stored</li><li>Free core; $29 one time for saved views</li></ul></div>
      <figure><picture><source media="(max-width: 640px)" srcset="/assets/run-proof-diorama-720.webp"><img src="/assets/run-proof-diorama.webp" width="1280" height="853" alt="Paper schedule ticket passing through a worker press and becoming a proof receipt" fetchpriority="high" decoding="async"></picture><figcaption>Schedule, run, and receipt</figcaption></figure>
    </section>
    <section id="ledger" class="ledger-section" aria-labelledby="ledger-title"><div class="section-heading"><div><p class="kicker">Current evidence</p><h2 id="ledger-title">Run ledger</h2></div><div class="connection" id="connection"><span></span>Checking receiver…</div></div>
      <div id="summary" class="summary" aria-label="Run status summary"></div>
      <div class="toolbar"><div class="filters" id="filters" aria-label="Filter ledger by status"></div><label class="search"><span class="visually-hidden">Search jobs and run IDs</span><svg aria-hidden="true" viewBox="0 0 24 24"><circle cx="10" cy="10" r="6"/><path d="m15 15 5 5"/></svg><input id="search" type="search" placeholder="Search jobs or run IDs"></label><button class="save-view" id="save-view" type="button" hidden>Save view</button><button class="button export" id="export" type="button">Export ledger CSV</button></div>
      <div id="notice" class="notice" role="status" aria-live="polite"></div><div id="ledger-content" aria-live="polite" aria-busy="true"><div class="loading-paper"><span></span><span></span><span></span><p>Reading signed receipts…</p></div></div>
    </section>
    <section id="setup" class="setup" aria-labelledby="setup-title"><div><p class="kicker">How it works</p><h2 id="setup-title">Connect one recurring job</h2><p>Register its schedule. Send start and finish records. Add a CI observation when you need to compare signals.</p><ol><li><strong>Register intent</strong><span>Name the cadence and grace window.</span></li><li><strong>Record the run</strong><span>Send start and finish with the same run ID.</span></li><li><strong>Compare a source</strong><span>Add a CI observation to find conflicts.</span></li></ol></div><div class="code-slip"><div><span>Shell</span><button id="copy-code" type="button">Copy</button></div><pre tabindex="0" aria-label="Command line setup example"><code id="setup-code">export RUN_PROOF_URL=https://proof.example.com
export RUN_PROOF_SECRET=&lt;your-32+-character-secret&gt;

run-proof register billing-sweep \\
  --name "Billing sweep" --every 3600 --grace 300

RUN_ID="billing-$(date +%s)"
run-proof start billing-sweep "$RUN_ID" \\
  --scheduled "$(date -u +%FT%TZ)"

run-proof finish billing-sweep "$RUN_ID" \\
  --status success --count 428</code></pre></div></section>
    <section id="plus" class="plus" aria-labelledby="plus-title"><div class="plus-stamp" aria-hidden="true">PLUS</div><div><p class="kicker">Pricing</p><h2 id="plus-title">Save a filtered view</h2><p>The free core includes signed ingest and exports. Plus remembers a filtered operational view on this device.</p><div class="price"><strong>$29</strong><span>one time</span></div><div class="plus-actions"><a class="button primary" href="https://api.sociobot.in/api/v1/products/job-liveness-proof/checkout" rel="external">Buy Run Proof Plus</a><button class="button secondary" id="restore-toggle" type="button">Restore purchase</button></div><form id="restore" class="restore" hidden><label for="license">License token</label><div><input id="license" required autocomplete="off" spellcheck="false"><button class="button primary" type="submit">Verify license</button></div><p id="license-status" role="status" aria-live="polite"></p></form><p class="fine">Sociobot/Dodo is the merchant of record. <a href="/terms" data-route>Terms apply.</a></p></div></section>
  </main>`);
  bindRouteLinks();
  bindEvents();
  void loadLedger();
  if (!demo) void initialiseLicense();
  void loadBuildIdentity();
}

function bindRouteLinks(): void {
  document.querySelectorAll<HTMLAnchorElement>('a[data-route]').forEach(link => link.addEventListener('click', event => {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    navigate(link.pathname);
  }));
}

function navigate(path: string, replace = false): void {
  const normalised = path === '/' ? '/' : path.replace(/\/$/, '');
  if (location.pathname !== normalised) history[replace ? 'replaceState' : 'pushState']({}, '', normalised);
  if (normalised === '/privacy' || normalised === '/terms') renderLegal(normalised.slice(1) as 'privacy' | 'terms');
  else renderApp(normalised === '/demo');
  const heading = document.querySelector<HTMLElement>('h1');
  if (heading) { heading.tabIndex = -1; heading.focus({ preventScroll: true }); document.querySelector('#route-announcement')!.textContent = `${heading.textContent?.trim()}.`; }
  window.scrollTo({ top: 0, behavior: 'auto' });
}

function bindEvents(): void {
  document.querySelector('#search')?.addEventListener('input', event => { query = (event.target as HTMLInputElement).value; renderRows(); });
  document.querySelector('#export')?.addEventListener('click', exportLedger);
  document.querySelector('#save-view')?.addEventListener('click', () => { localStorage.setItem('run-proof:plus-view', JSON.stringify({ selectedState, query })); showNotice('This view is saved on this device.', 'success'); });
  document.querySelector('#copy-code')?.addEventListener('click', async event => { const button = event.currentTarget as HTMLButtonElement; try { await navigator.clipboard.writeText(document.querySelector('#setup-code')?.textContent ?? ''); button.textContent = 'Copied'; setTimeout(() => { button.textContent = 'Copy'; }, 1800); } catch { button.textContent = 'Select code to copy'; } });
  document.querySelector('#restore-toggle')?.addEventListener('click', () => { const form = document.querySelector<HTMLFormElement>('#restore')!; form.hidden = !form.hidden; if (!form.hidden) document.querySelector<HTMLInputElement>('#license')?.focus(); });
  document.querySelector('#restore')?.addEventListener('submit', event => { event.preventDefault(); const token = document.querySelector<HTMLInputElement>('#license')!.value.trim(); if (token) { localStorage.setItem(licenseKey, token); void verifyLicense(token, true); } });
  document.querySelector('#reset-demo')?.addEventListener('click', () => { localStorage.removeItem(demoCacheKey); selectedState = 'all'; query = ''; void loadLedger(true); });
}

async function loadLedger(resetDemo = false): Promise<void> {
  const content = document.querySelector('#ledger-content')!;
  const connection = document.querySelector('#connection')!;
  const cacheKey = demo ? demoCacheKey : realCacheKey;
  const endpoint = demo ? '/api/v1/demo/ledger' : '/api/v1/ledger';
  try {
    const response = await fetch(endpoint, { headers: { accept: 'application/json' } });
    if (!response.ok) throw new Error(`Receiver returned ${response.status}`);
    ledger = await response.json() as Ledger;
    localStorage.setItem(cacheKey, JSON.stringify(ledger));
    offline = false;
    connection.innerHTML = demo ? '<span></span>Sample ledger loaded' : '<span></span>Receiver connected';
    connection.className = 'connection online';
    if (resetDemo) showNotice('The sample ledger has been reset. Your real ledger was not changed.', 'success');
  } catch (error) {
    const cached = localStorage.getItem(cacheKey);
    if (cached) {
      ledger = JSON.parse(cached) as Ledger;
      offline = true;
      connection.innerHTML = '<span></span>Offline · showing last copy';
      connection.className = 'connection offline';
      showNotice('The receiver is unavailable. This is the last ledger saved on this device; export still works.', 'warning');
    } else {
      content.innerHTML = `<div class="state-panel error"><div class="state-icon">!</div><h3>Could not reach the ledger</h3><p>${esc(error instanceof Error ? error.message : 'Check the receiver and try again.')}</p><button class="button primary" id="retry" type="button">Try again</button></div>`;
      content.setAttribute('aria-busy', 'false');
      document.querySelector('#retry')?.addEventListener('click', () => { void loadLedger(); });
      return;
    }
  }
  renderSummary(); renderFilters(); renderRows(); content.setAttribute('aria-busy', 'false');
}

function renderSummary(): void {
  const summary = document.querySelector('#summary')!;
  const priority = ['contradictory', 'missed', 'late', 'completed'] as const;
  summary.innerHTML = priority.map(state => `<button class="summary-slip ${state}" data-state="${state}" type="button"><span>${state === 'contradictory' ? '!' : state === 'missed' ? '×' : state === 'late' ? '◷' : '✓'}</span><strong>${ledger?.summary[state] ?? 0}</strong><small>${state}</small></button>`).join('');
  summary.querySelectorAll('button').forEach(button => button.addEventListener('click', () => { selectedState = (button as HTMLElement).dataset.state!; renderFilters(); renderRows(); document.querySelector('#ledger-title')?.scrollIntoView({ behavior: 'smooth' }); }));
}

function renderFilters(): void {
  const filters = document.querySelector('#filters')!;
  filters.innerHTML = ['all', ...states].map(state => `<button type="button" data-state="${state}" aria-pressed="${selectedState === state}">${esc(state)}</button>`).join('');
  filters.querySelectorAll('button').forEach(button => button.addEventListener('click', () => { selectedState = (button as HTMLElement).dataset.state!; renderFilters(); renderRows(); }));
}

function renderRows(): void {
  const content = document.querySelector('#ledger-content')!;
  const rows = filterRows(ledger?.rows ?? [], selectedState, query);
  if (!ledger?.rows.length) { content.innerHTML = `<div class="state-panel empty"><div class="empty-art" aria-hidden="true"><span></span><span></span><span></span></div><h3>The ledger is ready</h3><p>Register a job and send its first signed start record. Your ledger will show it here.</p><a class="button primary" href="#setup">Connect the first job</a></div>`; return; }
  if (!rows.length) { content.innerHTML = `<div class="state-panel"><h3>No receipts match</h3><p>Clear the status filter or search to see the full record.</p><button class="button secondary" id="clear-filter" type="button">Clear filters</button></div>`; document.querySelector('#clear-filter')?.addEventListener('click', () => { selectedState = 'all'; query = ''; (document.querySelector('#search') as HTMLInputElement).value = ''; renderFilters(); renderRows(); }); return; }
  content.innerHTML = `<div class="table-wrap"><table><thead><tr><th>Run</th><th>Schedule</th><th>Evidence</th><th>Source view</th><th>Status</th><th><span class="visually-hidden">Actions</span></th></tr></thead><tbody>${rows.map(rowTemplate).join('')}</tbody></table></div>`;
  content.querySelectorAll<HTMLButtonElement>('[data-receipt]').forEach(button => button.addEventListener('click', () => { void downloadReceipt(button.dataset.job!, button.dataset.receipt!, button); }));
}

function rowTemplate(row: LedgerRow): string {
  const label = contradictionLabel(row);
  const schedule = row.scheduled_at ? `<time datetime="${esc(row.scheduled_at)}">${esc(formatTime(row.scheduled_at))}</time>` : '<span class="muted">Not received</span>';
  return `<tr class="row-${esc(row.state)}"><td data-label="Run"><strong>${esc(row.display_name)}</strong><code>${esc(row.run_id)}</code></td><td data-label="Schedule">${schedule}<small>${esc(relative(row.scheduled_at))}</small></td><td data-label="Evidence"><span>${row.started_at ? 'Started' : 'No start'}</span><span>${row.finished_at ? `Finished${row.completion_count !== null ? ` · ${row.completion_count.toLocaleString()} items` : ''}` : 'No finish'}</span></td><td data-label="Source view">${row.source ? `${row.source_url ? `<a href="${esc(row.source_url)}" target="_blank" rel="noreferrer">${esc(row.source)} ↗</a>` : `<span>${esc(row.source)}</span>`}<small>${esc(row.observed_status)}</small>` : '<span class="muted">Not imported</span>'}</td><td data-label="Status"><span class="status ${esc(row.state)}"><i aria-hidden="true"></i>${esc(label)}</span></td><td class="action-cell"><button class="receipt-button" type="button" data-job="${esc(row.job_key)}" data-receipt="${esc(row.run_id)}" aria-label="Export ${row.is_virtual ? 'derived ' : ''}receipt for ${esc(row.display_name)}">${row.is_virtual ? 'Derived receipt' : 'Receipt'} ↓</button></td></tr>`;
}

async function downloadReceipt(jobKey: string, runId: string, button: HTMLButtonElement): Promise<void> {
  button.disabled = true; button.textContent = 'Preparing…';
  try {
    if (demo) { const row = ledger?.rows.find(item => item.job_key === jobKey && item.run_id === runId); download(new Blob([JSON.stringify({ format: 'run-proof-demo-receipt/v1', sample: true, run: row }, null, 2)], { type: 'application/json' }), `run-proof-sample-${safeName(jobKey)}-${safeName(runId)}.json`); showNotice('Sample receipt exported. Your real ledger was not changed.', 'success'); }
    else { const response = await fetch(`/api/v1/jobs/${encodeURIComponent(jobKey)}/runs/${encodeURIComponent(runId)}/receipt`); if (!response.ok) throw new Error('Receipt is not available'); download(await response.blob(), `run-proof-${safeName(jobKey)}-${safeName(runId)}.json`); showNotice('Receipt exported with verifiable signed request material.', 'success'); }
  } catch (error) { showNotice(error instanceof Error ? error.message : 'Could not export this receipt.', 'error'); } finally { button.disabled = false; button.textContent = 'Receipt ↓'; }
}

function exportLedger(): void {
  if (demo) { const lines = ['job_key,display_name,run_id,state,completion_count']; for (const row of ledger?.rows ?? []) lines.push([row.job_key, row.display_name, row.run_id, row.state, row.completion_count ?? ''].map(value => `"${String(value).replaceAll('"', '""')}"`).join(',')); download(new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' }), 'run-proof-sample-ledger.csv'); showNotice('Sample ledger CSV exported. Your real ledger was not changed.', 'success'); }
  else if (offline && ledger) { download(new Blob([JSON.stringify(ledger, null, 2)], { type: 'application/json' }), 'run-proof-offline-ledger.json'); showNotice('Offline ledger copy exported as JSON.', 'success'); }
  else location.href = '/api/v1/exports/ledger.csv';
}

function download(blob: Blob, name: string): void { const url = URL.createObjectURL(blob); const link = document.createElement('a'); link.href = url; link.download = name; link.click(); setTimeout(() => { URL.revokeObjectURL(url); }, 1000); }
function safeName(value: string): string { return value.replace(/[^a-z0-9_.-]/gi, '-').slice(0, 80); }
function showNotice(message: string, type: string): void { const node = document.querySelector('#notice'); if (node) { node.textContent = message; node.className = `notice visible ${type}`; } }

async function loadBuildIdentity(): Promise<void> { try { const health = await fetch('/health'); const value = await health.json() as { build_sha?: string }; const target = document.querySelector('#build-id'); if (target) target.textContent = `Build: ${(value.build_sha ?? 'unknown').slice(0, 12)}`; } catch { const target = document.querySelector('#build-id'); if (target) target.textContent = 'Build: unavailable'; } }

async function initialiseLicense(): Promise<void> {
  const params = new URLSearchParams(location.search); const returned = params.get('license');
  if (returned) { localStorage.setItem(licenseKey, returned); params.delete('license'); history.replaceState({}, '', `${location.pathname}${params.size ? `?${params}` : ''}${location.hash}`); }
  const token = returned ?? localStorage.getItem(licenseKey); if (!token) return;
  const cache = localStorage.getItem(`${licenseKey}:verdict`);
  if (cache) { const parsed = JSON.parse(cache) as { valid: boolean; checked_at: number }; if (parsed.valid) markUnlocked(); if (Date.now() - parsed.checked_at < 86_400_000) return; }
  await verifyLicense(token, false);
}

async function verifyLicense(token: string, announce: boolean): Promise<void> {
  const status = document.querySelector('#license-status'); if (status && announce) status.textContent = 'Checking license…';
  try { const response = await fetch(`/api/v1/products/${slug}/verify`, { method: 'POST', headers: { accept: 'application/json', 'content-type': 'application/json' }, body: JSON.stringify({ license: token }) }); if (!response.ok) throw new Error(`Verification returned ${response.status}`); const verdict = await response.json() as { valid: boolean; reason: string }; localStorage.setItem(`${licenseKey}:verdict`, JSON.stringify({ ...verdict, checked_at: Date.now() })); if (verdict.valid) { markUnlocked(); if (status) status.textContent = 'Plus is active on this device.'; } else if (status) status.textContent = 'This license is no longer active. You can purchase a new license above.'; }
  catch { if (status && announce) status.textContent = 'Could not verify right now. A previously verified license will keep working offline.'; }
}

function markUnlocked(): void { document.querySelector('#plus')?.classList.add('unlocked'); const stamp = document.querySelector('.plus-stamp'); if (stamp) stamp.textContent = 'ACTIVE'; const button = document.querySelector<HTMLButtonElement>('#save-view'); if (button) button.hidden = false; const saved = localStorage.getItem('run-proof:plus-view'); if (saved) { const view = JSON.parse(saved) as { selectedState: string; query: string }; selectedState = view.selectedState || 'all'; query = view.query || ''; const search = document.querySelector<HTMLInputElement>('#search'); if (search) search.value = query; renderFilters(); renderRows(); } }

window.addEventListener('popstate', () => navigate(location.pathname, true));
const route = location.pathname.replace(/\/$/, '') || '/';
if (route === '/privacy' || route === '/terms') renderLegal(route.slice(1) as 'privacy' | 'terms'); else renderApp(route === '/demo');
if ('serviceWorker' in navigator) window.addEventListener('load', () => navigator.serviceWorker.register('/sw.js', { updateViaCache: 'none' }).catch(() => undefined));
