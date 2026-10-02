-- Filter counts and totals per district ('' for the state, '-' for projects filed under none),
-- filed under the dashboard read they were counted from. See db::Facets.
CREATE TABLE facet_cache (
  district TEXT PRIMARY KEY,
  version  TEXT NOT NULL,  -- sources.last_scraped_at for the dashboard
  json     TEXT NOT NULL
);

-- The list's other sort orders; the default (flags) already has projects_by_rank.
CREATE INDEX projects_by_amount ON projects(headline_amount DESC, code);
CREATE INDEX projects_by_spent ON projects(COALESCE(expenditure, 0) DESC, code);
CREATE INDEX projects_by_name ON projects(title_en COLLATE NOCASE, code);
CREATE INDEX projects_by_changed ON projects(changed_on DESC, flag_count DESC, code);
