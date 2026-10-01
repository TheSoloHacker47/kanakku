-- KIIFB's guidelines place the Project Execution Document after technical sanction (stage::from_status).
UPDATE projects SET stage = 'technical_sanction' WHERE official_status LIKE 'Project Execution Document%';
