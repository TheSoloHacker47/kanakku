-- Kanakku v1: KIIFB projects for one pilot district. All money is whole rupees.

CREATE TABLE sources (
  id              INTEGER PRIMARY KEY,
  name            TEXT NOT NULL,
  base_url        TEXT NOT NULL,
  kind            TEXT NOT NULL,
  license         TEXT,
  last_scraped_at TEXT
);

INSERT INTO sources (id, name, base_url, kind)
VALUES (1, 'KIIFB integrated dashboard', 'https://gis.kiifb.org/', 'scrape');

-- One row per distinct copy of a source page. The page itself lives in R2.
CREATE TABLE snapshots (
  id         INTEGER PRIMARY KEY,
  source_id  INTEGER NOT NULL REFERENCES sources(id),
  url        TEXT NOT NULL,
  r2_key     TEXT NOT NULL,
  sha256     TEXT NOT NULL,
  bytes      INTEGER NOT NULL,
  fetched_at TEXT NOT NULL
);
CREATE INDEX snapshots_by_source ON snapshots(source_id, id DESC);

CREATE TABLE projects (
  id                     INTEGER PRIMARY KEY,
  code                   TEXT NOT NULL UNIQUE,
  title_en               TEXT NOT NULL,
  department             TEXT,
  sector                 TEXT,
  executing_agency       TEXT,
  funding_source         TEXT NOT NULL DEFAULT 'KIIFB',
  district               TEXT NOT NULL,
  estimated_amount       INTEGER,            -- stated per sub-project; sibling packages repeat it
  sub_project_code       TEXT,
  estimate_shared_by     INTEGER NOT NULL DEFAULT 1, -- packages statewide that carry this same estimate
  expenditure            INTEGER,
  works_amount           INTEGER,            -- sum of works' financial sanction, when KIIFB lists only works
  headline_amount        INTEGER NOT NULL DEFAULT 0, -- the figure the list shows and sorts by
  official_status        TEXT,
  flag_count             INTEGER NOT NULL DEFAULT 0,
  first_estimated_amount INTEGER,            -- the estimate in the first snapshot we recorded
  first_seen_on          TEXT NOT NULL,      -- yyyy-mm-dd, India
  changed_on             TEXT NOT NULL,      -- last day any reported field changed
  missing_since          TEXT,               -- set when the project drops off the dashboard
  snapshot_id            INTEGER NOT NULL REFERENCES snapshots(id),
  record_json            TEXT NOT NULL       -- the normalised source record, used to detect changes
);
CREATE INDEX projects_by_rank ON projects(flag_count DESC, headline_amount DESC, code);
CREATE INDEX projects_by_department ON projects(department);
CREATE INDEX projects_by_status ON projects(official_status);

CREATE TABLE project_constituencies (
  project_id  INTEGER NOT NULL REFERENCES projects(id),
  name        TEXT NOT NULL,
  name_ml     TEXT,
  mla_name    TEXT,
  mla_name_ml TEXT,
  PRIMARY KEY (project_id, name)
) WITHOUT ROWID;
CREATE INDEX constituencies_by_name ON project_constituencies(name);

CREATE TABLE sites (
  project_id INTEGER NOT NULL REFERENCES projects(id),
  lat        REAL NOT NULL,
  lng        REAL NOT NULL
);
CREATE INDEX sites_by_project ON sites(project_id);

CREATE TABLE works (
  project_id      INTEGER NOT NULL REFERENCES projects(id),
  seq             INTEGER NOT NULL,          -- position within the project, 1-based
  work_ref        TEXT NOT NULL,             -- road name, or '#seq' when KIIFB gives none
  road_name       TEXT,
  spv             TEXT,
  contractor_name TEXT,
  as_amount       INTEGER,
  fs_amount       INTEGER,
  ts_amount       INTEGER,
  tender_amount   INTEGER,
  loa_amount      INTEGER,
  contract_amount INTEGER,
  paid_amount     INTEGER,
  paid_contractor INTEGER,
  scheduled_start TEXT,
  scheduled_end   TEXT,
  progress_note   TEXT,
  physical_pct    REAL,
  financial_pct   REAL,
  status          TEXT,
  lat             REAL,
  lng             REAL,
  PRIMARY KEY (project_id, seq)
) WITHOUT ROWID;

-- A field that changed between two snapshots.
CREATE TABLE observations (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER NOT NULL REFERENCES projects(id),
  snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
  field       TEXT NOT NULL,
  old_value   TEXT,
  new_value   TEXT,
  observed_on TEXT NOT NULL
);
CREATE INDEX observations_by_project ON observations(project_id, id DESC);

CREATE TABLE flags (
  id           INTEGER PRIMARY KEY,
  project_id   INTEGER NOT NULL REFERENCES projects(id),
  work_ref     TEXT,
  type         TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  value_json   TEXT NOT NULL,
  snapshot_id  INTEGER NOT NULL REFERENCES snapshots(id),
  status       TEXT NOT NULL DEFAULT 'open', -- open | cleared
  created_on   TEXT NOT NULL,
  cleared_on   TEXT
);
CREATE INDEX flags_by_project ON flags(project_id, status);
CREATE INDEX flags_by_type ON flags(type, status);

CREATE TABLE audit_log (
  id        INTEGER PRIMARY KEY,
  actor     TEXT NOT NULL,
  action    TEXT NOT NULL,
  entity    TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  diff_json TEXT,
  at        TEXT NOT NULL
);

-- Substring search over the fields people search by. rowid = projects.id.
CREATE VIRTUAL TABLE projects_fts USING fts5(
  code, title, agency, constituencies, contractors, roads,
  tokenize = 'trigram'
);
