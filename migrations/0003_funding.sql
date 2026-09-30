-- KIIFB's own project status page: what it approved and what it has released, per project and per work.
-- A "project" here is a sub-project on the map dashboard; the two sources share no identifier.

INSERT INTO sources (id, name, base_url, kind)
VALUES (2, 'KIIFB project status', 'https://www.kiifb.org/prjStatus.jsp', 'scrape');

CREATE TABLE funding_projects (
  id                 INTEGER PRIMARY KEY,
  ref                TEXT NOT NULL UNIQUE,   -- KIIFB's id for the row on the status page
  name               TEXT NOT NULL,
  department         TEXT,
  spv                TEXT,
  main_project       TEXT,                   -- the budget announcement it sits under
  approved_amount    INTEGER,
  released_amount    INTEGER,
  status             TEXT,
  work_count         INTEGER NOT NULL DEFAULT 0,
  over_paid_works    INTEGER NOT NULL DEFAULT 0, -- works where more was paid than approved
  group_key          TEXT,                   -- the map sub-project (or project code) this was joined to
  match_basis        TEXT,                   -- what the join rests on, see kiifb_status::Basis
  first_seen_on      TEXT NOT NULL,
  changed_on         TEXT NOT NULL,
  missing_since      TEXT,
  snapshot_id        INTEGER NOT NULL REFERENCES snapshots(id), -- the list page
  detail_snapshot_id INTEGER REFERENCES snapshots(id),          -- the work table
  detail_checked_at  TEXT,                   -- when the work table was last read, UTC
  record_json        TEXT NOT NULL
);
CREATE INDEX funding_projects_by_group ON funding_projects(group_key);
CREATE INDEX funding_projects_by_amount ON funding_projects(approved_amount DESC);

CREATE TABLE funding_works (
  funding_project_id INTEGER NOT NULL REFERENCES funding_projects(id),
  seq                INTEGER NOT NULL,       -- position within the project, 1-based
  name               TEXT NOT NULL,
  spv                TEXT,
  approved_amount    INTEGER,
  paid_amount        INTEGER,
  status             TEXT,
  project_code       TEXT,                   -- the map package with the same published title
  PRIMARY KEY (funding_project_id, seq)
) WITHOUT ROWID;
CREATE INDEX funding_works_by_package ON funding_works(project_code);

CREATE TABLE funding_observations (
  id                 INTEGER PRIMARY KEY,
  funding_project_id INTEGER NOT NULL REFERENCES funding_projects(id),
  snapshot_id        INTEGER NOT NULL REFERENCES snapshots(id),
  field              TEXT NOT NULL,
  old_value          TEXT,
  new_value          TEXT,
  observed_on        TEXT NOT NULL
);
CREATE INDEX funding_observations_by_project ON funding_observations(funding_project_id, id DESC);
