ALTER TABLE jobs ADD COLUMN signed_key_id TEXT;
ALTER TABLE jobs ADD COLUMN signed_timestamp TEXT;
ALTER TABLE jobs ADD COLUMN signed_body TEXT;
ALTER TABLE jobs ADD COLUMN signature TEXT;

ALTER TABLE events ADD COLUMN signed_key_id TEXT;
ALTER TABLE events ADD COLUMN signed_timestamp TEXT;
ALTER TABLE events ADD COLUMN signed_body TEXT;

ALTER TABLE ci_snapshots ADD COLUMN signed_key_id TEXT;
ALTER TABLE ci_snapshots ADD COLUMN signed_timestamp TEXT;
ALTER TABLE ci_snapshots ADD COLUMN signed_body TEXT;

CREATE INDEX events_job_run ON events(job_key, run_id);
CREATE INDEX snapshots_job_run ON ci_snapshots(job_key, run_id, observed_at DESC);
