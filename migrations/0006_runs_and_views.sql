-- A log of every ingest run, so a silent failure is visible, and privacy-friendly visit counts.

CREATE TABLE ingest_runs (
  id          INTEGER PRIMARY KEY,
  source_id   INTEGER NOT NULL REFERENCES sources(id),
  trigger     TEXT NOT NULL,              -- cron | manual
  started_at  TEXT NOT NULL,
  finished_at TEXT NOT NULL,
  ok          INTEGER NOT NULL,
  summary     TEXT                        -- the run's report as JSON, or the error
);
CREATE INDEX ingest_runs_by_source ON ingest_runs(source_id, id DESC);

-- One counter per day, kind of page and language. No addresses, cookies or identifiers.
CREATE TABLE page_views (
  day  TEXT NOT NULL,
  kind TEXT NOT NULL,
  lang TEXT NOT NULL,
  n    INTEGER NOT NULL,
  PRIMARY KEY (day, kind, lang)
) WITHOUT ROWID;
