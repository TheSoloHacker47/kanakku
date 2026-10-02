-- A flag someone has questioned is marked "under review" while it is checked against the source.
-- Set by POST /admin/review; the nightly recompute updates flags in place, so the mark survives it.
ALTER TABLE flags ADD COLUMN review_since TEXT;
ALTER TABLE funding_flags ADD COLUMN review_since TEXT;
