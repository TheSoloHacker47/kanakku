-- The corrections acknowledgement: one per sender a day. Only a SHA-256 of the address is
-- kept, and rows older than a day are deleted on the next message.
CREATE TABLE email_acks (
  sender_hash TEXT PRIMARY KEY,
  sent_at     TEXT NOT NULL
);

-- The cost guard: which usage levels have been announced each month, so each is announced once.
CREATE TABLE usage_alerts (
  month  TEXT NOT NULL,     -- YYYY-MM, UTC
  metric TEXT NOT NULL,     -- see kanakku_core::usage::Metric::key
  level  INTEGER NOT NULL,  -- percent of the allowance, or 1 for "on course to pass it"
  PRIMARY KEY (month, metric, level)
) WITHOUT ROWID;
