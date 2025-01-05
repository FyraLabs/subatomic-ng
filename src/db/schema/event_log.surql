DEFINE TABLE IF NOT EXISTS log TYPE ANY SCHEMALESS PERMISSIONS NONE;

-- ------------------------------
-- FIELDS
-- ------------------------------
DEFINE FIELD action ON log TYPE string PERMISSIONS FULL;
DEFINE FIELD id ON log TYPE string PERMISSIONS FULL;
DEFINE FIELD data ON log FLEXIBLE TYPE object PERMISSIONS FULL;


--- EVENTS

DEFINE EVENT OVERWRITE log_ttl ON TABLE log
    WHEN $event = "CREATE"
    THEN {
        DELETE log where ttl < time::now();
};