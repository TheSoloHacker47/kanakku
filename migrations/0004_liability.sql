-- Kerala PWD's defect-liability list: finished works whose contractor must still repair defects.
-- Contact numbers printed on the source page are never stored.

INSERT INTO sources (id, name, base_url, kind)
VALUES (3, 'Kerala PWD defect liability list', 'https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php', 'scrape');

CREATE TABLE liability_works (
  id             INTEGER PRIMARY KEY,
  ref            TEXT NOT NULL UNIQUE,   -- hash of wing, name, start date and contractor; PWD gives no id
  wing           TEXT NOT NULL,
  name           TEXT NOT NULL,
  contractor     TEXT,
  contractor_key TEXT,                   -- groups spellings of one contractor
  starts_on      TEXT,
  ends_on        TEXT,
  division       TEXT,
  subdivision    TEXT,
  first_seen_on  TEXT NOT NULL,
  missing_since  TEXT,                   -- set when the work drops off PWD's list
  snapshot_id    INTEGER NOT NULL REFERENCES snapshots(id)
);
CREATE INDEX liability_by_end ON liability_works(ends_on);
CREATE INDEX liability_by_contractor ON liability_works(contractor_key);
