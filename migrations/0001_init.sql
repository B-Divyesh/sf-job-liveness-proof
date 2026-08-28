CREATE TABLE IF NOT EXISTS jobs (
  job_key TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  expected_interval_seconds INTEGER NOT NULL CHECK (expected_interval_seconds BETWEEN 60 AND 31536000),
  grace_seconds INTEGER NOT NULL CHECK (grace_seconds BETWEEN 0 AND 86400),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_key TEXT NOT NULL REFERENCES jobs(job_key),
  run_id TEXT NOT NULL,
  event_type TEXT NOT NULL CHECK (event_type IN ('start', 'finish')),
  scheduled_at TEXT,
  occurred_at TEXT NOT NULL,
  received_at TEXT NOT NULL,
  status TEXT CHECK (status IN ('success', 'failed', 'cancelled')),
  completion_count INTEGER CHECK (completion_count >= 0),
  signature TEXT NOT NULL,
  UNIQUE(job_key, run_id, event_type)
);

CREATE INDEX IF NOT EXISTS events_job_received ON events(job_key, received_at DESC);
CREATE INDEX IF NOT EXISTS events_run ON events(run_id);

CREATE TABLE IF NOT EXISTS ci_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_key TEXT NOT NULL REFERENCES jobs(job_key),
  run_id TEXT NOT NULL,
  source TEXT NOT NULL,
  observed_status TEXT NOT NULL CHECK (observed_status IN ('passed', 'failed', 'pending', 'missing')),
  source_url TEXT,
  observed_at TEXT NOT NULL,
  received_at TEXT NOT NULL,
  signature TEXT NOT NULL,
  UNIQUE(job_key, run_id, source, observed_at)
);

CREATE INDEX IF NOT EXISTS snapshots_run ON ci_snapshots(run_id, observed_at DESC);
