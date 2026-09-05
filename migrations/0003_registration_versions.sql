-- A schedule registration is signed evidence in its own right. Keep every
-- version so an already-recorded run continues to point at the intent that
-- governed it, even after its job is registered again.
CREATE TABLE job_registrations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_key TEXT NOT NULL,
  display_name TEXT NOT NULL,
  expected_interval_seconds INTEGER NOT NULL CHECK (expected_interval_seconds BETWEEN 60 AND 31536000),
  grace_seconds INTEGER NOT NULL CHECK (grace_seconds BETWEEN 0 AND 86400),
  created_at TEXT NOT NULL,
  signed_key_id TEXT,
  signed_timestamp TEXT,
  signed_body TEXT,
  signature TEXT
);

CREATE INDEX job_registrations_job_created ON job_registrations(job_key, id DESC);

-- Existing installations had one mutable registration per job. Preserve that
-- current value as their historical baseline before new records are bound to
-- immutable versions.
INSERT INTO job_registrations(
  job_key, display_name, expected_interval_seconds, grace_seconds, created_at,
  signed_key_id, signed_timestamp, signed_body, signature
)
SELECT
  job_key, display_name, expected_interval_seconds, grace_seconds, created_at,
  signed_key_id, signed_timestamp, signed_body, signature
FROM jobs;

ALTER TABLE events ADD COLUMN registration_id INTEGER REFERENCES job_registrations(id);
ALTER TABLE ci_snapshots ADD COLUMN registration_id INTEGER REFERENCES job_registrations(id);

UPDATE events
SET registration_id = (
  SELECT id FROM job_registrations
  WHERE job_registrations.job_key = events.job_key
  ORDER BY id ASC LIMIT 1
);

UPDATE ci_snapshots
SET registration_id = (
  SELECT id FROM job_registrations
  WHERE job_registrations.job_key = ci_snapshots.job_key
  ORDER BY id ASC LIMIT 1
);

CREATE INDEX events_registration ON events(registration_id);
CREATE INDEX snapshots_registration ON ci_snapshots(registration_id);
