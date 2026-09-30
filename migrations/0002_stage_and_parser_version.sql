-- Plain-language stage for filtering, and the parser version the stored records were built with.

ALTER TABLE sources ADD COLUMN parser_version INTEGER NOT NULL DEFAULT 0;

ALTER TABLE projects ADD COLUMN stage TEXT;
CREATE INDEX projects_by_stage ON projects(stage);
CREATE INDEX projects_by_sub_project ON projects(sub_project_code);
