-- The agreed contract amount of a work under defect liability. It is in PWD's page markup
-- but commented out there, so PWD's own page does not display it.

ALTER TABLE liability_works ADD COLUMN agreed_amount INTEGER;
