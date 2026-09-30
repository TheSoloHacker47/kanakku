-- KIIFB's list counts a project's payments once for every district it is filed under, so its
-- "Payment Released" figure is a multiple of the true one for projects that span districts.
-- released_amount now holds the sum of the works' paid amounts; released_listed keeps KIIFB's figure.

ALTER TABLE funding_projects ADD COLUMN released_listed INTEGER;
UPDATE funding_projects SET released_listed = released_amount;
UPDATE funding_projects
   SET released_amount = (SELECT COALESCE(SUM(w.paid_amount), 0) FROM funding_works w WHERE w.funding_project_id = funding_projects.id)
 WHERE work_count > 0;
