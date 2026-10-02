-- Sulekha, the local bodies' plan system: its public list of plan projects. Read slowly over
-- several nights once switched on (SULEKHA); see docs/sulekha-spike.md.

INSERT INTO sources (id, name, base_url, kind)
VALUES (4, 'Sulekha plan projects', 'https://plan.lsgkerala.gov.in/formulation/Public.aspx', 'scrape');

CREATE TABLE local_bodies (
  id            INTEGER PRIMARY KEY,
  district      TEXT NOT NULL,
  kind          TEXT NOT NULL,          -- dp, bp, m, c, gp; see kanakku_core::sulekha::Kind
  name          TEXT NOT NULL,          -- as Sulekha prints it
  first_seen_on TEXT NOT NULL,
  UNIQUE (district, kind, name)
);

CREATE TABLE plan_projects (
  local_body_id INTEGER NOT NULL REFERENCES local_bodies(id),
  year          TEXT NOT NULL,          -- the form's year value (29 = 2025-26)
  number        TEXT NOT NULL,          -- the number the list prints for the project
  name          TEXT NOT NULL,          -- usually Malayalam
  planned       INTEGER,                -- "Formulation", rupees
  spent         INTEGER,                -- "Expense", rupees
  first_seen_on TEXT NOT NULL,
  changed_on    TEXT NOT NULL,
  missing_since TEXT,
  snapshot_id   INTEGER NOT NULL REFERENCES snapshots(id),
  PRIMARY KEY (local_body_id, year, number)
) WITHOUT ROWID;

-- Where a read spread over several nights has got to, per source.
CREATE TABLE crawl_state (
  source_id   INTEGER PRIMARY KEY REFERENCES sources(id),
  cursor_json TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  finished_at TEXT                       -- when the last full pass ended
);
