DEFINE TABLE overwrite available_packages AS SELECT 
    *
FROM rpm_package WHERE available = true;