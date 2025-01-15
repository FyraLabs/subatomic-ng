DEFINE TABLE IF NOT EXISTS rpm_package TYPE ANY SCHEMALESS PERMISSIONS NONE;
LET $timeout = 5m;

-- ------------------------------
-- FIELDS
-- ------------------------------ 

DEFINE FIELD arch ON rpm_package TYPE string PERMISSIONS FULL;
DEFINE FIELD available ON rpm_package TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD epoch ON rpm_package TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD id ON rpm_package TYPE string PERMISSIONS FULL;
DEFINE FIELD name ON rpm_package TYPE string PERMISSIONS FULL;
DEFINE FIELD object_key ON rpm_package TYPE string PERMISSIONS FULL;
DEFINE FIELD provides ON rpm_package FLEXIBLE TYPE array<object> PERMISSIONS FULL;
DEFINE FIELD provides[*] ON rpm_package FLEXIBLE TYPE object PERMISSIONS FULL;
DEFINE FIELD requires ON rpm_package FLEXIBLE TYPE array<object> PERMISSIONS FULL;
DEFINE FIELD requires[*] ON rpm_package FLEXIBLE TYPE object PERMISSIONS FULL;
DEFINE FIELD tag ON rpm_package TYPE record<repo_tag> PERMISSIONS FULL;
DEFINE FIELD timestamp ON rpm_package TYPE datetime PERMISSIONS FULL;


--- EVENTS

DEFINE EVENT OVERWRITE package_enabled ON TABLE rpm_package WHEN $before.available != $after.available AND $after.available = true THEN {
    LET $ulid = rand::ulid();
    LET $timeout = 5m;
    CREATE log SET 
        action = 'package_enabled',
        data = {
            package_id: $after.id,
            ulid: $ulid
        };
        ttl = time::now() + $timeout;
};

DEFINE EVENT OVERWRITE package_disabled ON TABLE rpm_package WHEN $before.available != $after.available AND $after.available = false THEN {
    LET $ulid = rand::ulid();
    LET $timeout = 5m;
    CREATE log SET 
        action = 'package_disabled',
        data = {
            package_id: $after.id,
            ulid: $ulid
        };
        ttl = time::now() + $timeout;
};

DEFINE EVENT OVERWRITE package_created ON TABLE rpm_package
WHEN $event = "CREATE" THEN {
    LET $ulid = rand::ulid();
    LET $timeout = 5m;
    CREATE log SET 
        action = 'package_created',
        data = {
            package_id: $after.id,
            ulid: $ulid
        };
        ttl = time::now() + $timeout;
};