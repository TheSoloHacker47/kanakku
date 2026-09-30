-- Every district, shared keys for contractors and agencies, and flags on status-page works.

-- A project can be filed under more than one district.
CREATE TABLE project_districts (
  project_id INTEGER NOT NULL REFERENCES projects(id),
  district   TEXT NOT NULL,
  PRIMARY KEY (project_id, district)
) WITHOUT ROWID;
CREATE INDEX project_districts_by_district ON project_districts(district);
INSERT INTO project_districts (project_id, district) SELECT id, district FROM projects WHERE district != '';

ALTER TABLE project_constituencies ADD COLUMN district TEXT;
UPDATE project_constituencies SET district = (SELECT district FROM projects p WHERE p.id = project_constituencies.project_id);
CREATE INDEX projects_by_district ON projects(district);

-- One key per implementing agency and per contractor, however a source spells the name.
ALTER TABLE projects ADD COLUMN agency_key TEXT;
CREATE INDEX projects_by_agency ON projects(agency_key);
ALTER TABLE works ADD COLUMN contractor_key TEXT;
CREATE INDEX works_by_contractor ON works(contractor_key);
ALTER TABLE funding_projects ADD COLUMN agency_key TEXT;
CREATE INDEX funding_projects_by_agency ON funding_projects(agency_key);
ALTER TABLE funding_projects ADD COLUMN flag_count INTEGER NOT NULL DEFAULT 0;

CREATE TABLE funding_districts (
  funding_project_id INTEGER NOT NULL REFERENCES funding_projects(id),
  district           TEXT NOT NULL,
  PRIMARY KEY (funding_project_id, district)
) WITHOUT ROWID;
CREATE INDEX funding_districts_by_district ON funding_districts(district);
INSERT INTO funding_districts (funding_project_id, district) SELECT id, 'Ernakulam' FROM funding_projects;

-- The district of the PWD office that handles a work, which is not always where the work is.
ALTER TABLE liability_works ADD COLUMN district TEXT;
UPDATE liability_works SET district = 'Ernakulam';
CREATE INDEX liability_by_district ON liability_works(district);

-- Flags raised on works from the status page. Same shape as `flags`.
CREATE TABLE funding_flags (
  id                 INTEGER PRIMARY KEY,
  funding_project_id INTEGER NOT NULL REFERENCES funding_projects(id),
  work_ref           TEXT,
  type               TEXT NOT NULL,
  rule_version       INTEGER NOT NULL,
  value_json         TEXT NOT NULL,
  snapshot_id        INTEGER NOT NULL REFERENCES snapshots(id),
  status             TEXT NOT NULL DEFAULT 'open',
  created_on         TEXT NOT NULL,
  cleared_on         TEXT
);
CREATE INDEX funding_flags_by_project ON funding_flags(funding_project_id, status);
