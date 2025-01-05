DEFINE TABLE IF NOT EXISTS repo_tag TYPE ANY SCHEMALESS PERMISSIONS NONE;

-- ------------------------------
-- FIELDS
-- ------------------------------ 

DEFINE FIELD comps ON repo_tag FLEXIBLE TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD id ON repo_tag TYPE string PERMISSIONS FULL;
DEFINE FIELD name ON repo_tag TYPE string PERMISSIONS FULL;

-- ------------------------------
-- INDEXES
-- ------------------------------ 

DEFINE INDEX unique_id ON repo_tag FIELDS id;