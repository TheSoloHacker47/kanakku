-- Searches that found nothing, so spellings and place names the search misses can be fixed.
-- The words and a daily count only. No addresses, cookies or identifiers; text that looks
-- like an email address or a phone number is never written (see search::miss_key).

CREATE TABLE search_misses (
  day     TEXT NOT NULL,
  surface TEXT NOT NULL,              -- projects | funding | contractors
  lang    TEXT NOT NULL,
  q       TEXT NOT NULL,
  n       INTEGER NOT NULL,
  PRIMARY KEY (day, surface, lang, q)
) WITHOUT ROWID;

-- A contractor "name" with no letters in it ("0") is a placeholder, not a contractor.
UPDATE works SET contractor_key = NULL WHERE contractor_key IS NOT NULL AND contractor_key NOT GLOB '*[a-z]*';
UPDATE liability_works SET contractor_key = NULL WHERE contractor_key IS NOT NULL AND contractor_key NOT GLOB '*[a-z]*';
