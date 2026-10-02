-- Where readers come from, for knowing which outreach works. Daily counts only:
--   kind = 'source' : the referring site's name (never the full address), see visits::referrer_source
--   kind = 'tag'    : a ?ref= tag on a link we handed out, see visits::campaign_tag
--   kind = 'page'   : one project, payment, district, contractor or agency page, see visits::page_key
-- No addresses, cookies or identifiers.

CREATE TABLE visit_counts (
  day  TEXT NOT NULL,
  kind TEXT NOT NULL,
  key  TEXT NOT NULL,
  n    INTEGER NOT NULL,
  PRIMARY KEY (day, kind, key)
) WITHOUT ROWID;
CREATE INDEX visit_counts_by_kind ON visit_counts(kind, day);
