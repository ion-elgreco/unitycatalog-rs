-- The catalog's tables. Every row is keyed by its UUID (`id_string`) and
-- links to its parent by id, so renames are field updates. Names are unique
-- within the parent. The table and column names are those of the backup
-- format (`Snapshot` in src/admin.rs).

CREATE TABLE catalog (
    id_string        TEXT PRIMARY KEY,
    name             TEXT NOT NULL UNIQUE,
    comment          TEXT,
    properties       TEXT,
    owner            TEXT,
    created_at       BIGINT NOT NULL,
    created_by       TEXT,
    updated_at       BIGINT NOT NULL,
    updated_by       TEXT,
    storage_root     TEXT,
    storage_location TEXT
);

CREATE TABLE schema (
    id_string        TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    catalog_id       TEXT NOT NULL,
    comment          TEXT,
    properties       TEXT,
    owner            TEXT,
    created_at       BIGINT NOT NULL,
    created_by       TEXT,
    updated_at       BIGINT NOT NULL,
    updated_by       TEXT,
    storage_root     TEXT,
    storage_location TEXT,
    UNIQUE (catalog_id, name)
);

-- `table` is a reserved word.
CREATE TABLE tbl (
    id_string                                 TEXT PRIMARY KEY,
    name                                      TEXT NOT NULL,
    schema_id                                 TEXT NOT NULL,
    table_type                                TEXT NOT NULL,
    data_source_format                        TEXT,
    columns                                   TEXT,
    storage_location                          TEXT,
    comment                                   TEXT,
    properties                                TEXT,
    owner                                     TEXT,
    created_at                                BIGINT NOT NULL,
    created_by                                TEXT,
    updated_at                                BIGINT NOT NULL,
    updated_by                                TEXT,
    view_definition                           TEXT,
    view_dependencies                         TEXT,
    iceberg_metadata_location                 TEXT,
    uniform_iceberg_converted_delta_version   BIGINT,
    uniform_iceberg_converted_delta_timestamp BIGINT,
    UNIQUE (schema_id, name)
);

-- Upstream `uc_staging_tables`: a managed location reserved for a table a
-- client is about to create; creating the MANAGED table commits it.
CREATE TABLE staging_table (
    id_string           TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    schema_id           TEXT NOT NULL,
    staging_location    TEXT NOT NULL UNIQUE,
    created_at          BIGINT NOT NULL,
    created_by          TEXT,
    accessed_at         BIGINT NOT NULL,
    stage_committed     BOOLEAN NOT NULL,
    stage_committed_at  BIGINT,
    purge_state         BIGINT NOT NULL,
    num_cleanup_retries BIGINT NOT NULL,
    last_cleanup_at     BIGINT
);
CREATE INDEX staging_table_by_schema ON staging_table (schema_id);

-- Upstream `uc_delta_commits`.
CREATE TABLE delta_commit (
    table_id                           TEXT NOT NULL,
    commit_version                     BIGINT NOT NULL,
    commit_filename                    TEXT NOT NULL,
    commit_filesize                    BIGINT NOT NULL,
    commit_file_modification_timestamp BIGINT NOT NULL,
    commit_timestamp                   BIGINT NOT NULL,
    is_backfilled_latest_commit        BOOLEAN NOT NULL,
    PRIMARY KEY (table_id, commit_version)
);

CREATE TABLE volume (
    id_string        TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    schema_id        TEXT NOT NULL,
    volume_type      TEXT NOT NULL,
    comment          TEXT,
    owner            TEXT,
    created_at       BIGINT NOT NULL,
    created_by       TEXT,
    updated_at       BIGINT NOT NULL,
    updated_by       TEXT,
    storage_location TEXT,
    UNIQUE (schema_id, name)
);

-- `function` is a reserved word.
CREATE TABLE func (
    id_string            TEXT PRIMARY KEY,
    name                 TEXT NOT NULL,
    schema_id            TEXT NOT NULL,
    input_params         TEXT,
    data_type            TEXT,
    full_data_type       TEXT,
    return_params        TEXT,
    routine_body         TEXT NOT NULL,
    routine_definition   TEXT,
    routine_dependencies TEXT,
    parameter_style      TEXT NOT NULL,
    is_deterministic     BOOLEAN,
    sql_data_access      TEXT,
    is_null_call         BOOLEAN,
    security_type        TEXT NOT NULL,
    specific_name        TEXT,
    comment              TEXT,
    properties           TEXT,
    external_language    TEXT,
    owner                TEXT,
    created_at           BIGINT NOT NULL,
    created_by           TEXT,
    updated_at           BIGINT NOT NULL,
    updated_by           TEXT,
    UNIQUE (schema_id, name)
);

CREATE TABLE registered_model (
    id_string          TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    schema_id          TEXT NOT NULL,
    comment            TEXT,
    owner              TEXT,
    created_at         BIGINT NOT NULL,
    created_by         TEXT,
    updated_at         BIGINT NOT NULL,
    updated_by         TEXT,
    storage_location   TEXT,
    max_version_number BIGINT,
    UNIQUE (schema_id, name)
);

CREATE TABLE model_version (
    id_string           TEXT PRIMARY KEY,
    registered_model_id TEXT NOT NULL,
    version             BIGINT NOT NULL,
    source              TEXT NOT NULL,
    run_id              TEXT,
    status              TEXT NOT NULL,
    storage_location    TEXT,
    comment             TEXT,
    created_at          BIGINT NOT NULL,
    created_by          TEXT,
    updated_at          BIGINT NOT NULL,
    updated_by          TEXT,
    UNIQUE (registered_model_id, version)
);

-- One identity per credential: an AWS role, an Azure service principal or
-- an Azure managed identity.
CREATE TABLE credential (
    id_string                 TEXT PRIMARY KEY,
    name                      TEXT NOT NULL UNIQUE,
    comment                   TEXT,
    owner                     TEXT,
    purpose                   TEXT,
    aws_role_arn              TEXT,
    aws_external_id           TEXT,
    azure_directory_id        TEXT,
    azure_application_id      TEXT,
    azure_client_secret       TEXT,
    azure_managed_identity    BOOLEAN NOT NULL,
    azure_managed_identity_id TEXT,
    created_at                BIGINT NOT NULL,
    created_by                TEXT,
    updated_at                BIGINT NOT NULL,
    updated_by                TEXT
);

CREATE TABLE external_location (
    id_string     TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    url           TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    comment       TEXT,
    owner         TEXT,
    created_at    BIGINT NOT NULL,
    created_by    TEXT,
    updated_at    BIGINT NOT NULL,
    updated_by    TEXT
);
CREATE INDEX external_location_by_url ON external_location (url);
CREATE INDEX external_location_by_credential ON external_location (credential_id);

-- The grants of one principal on one securable, named by id.
CREATE TABLE permission (
    id_string      TEXT PRIMARY KEY,
    securable_type TEXT NOT NULL,
    resource_id    TEXT NOT NULL,
    principal      TEXT NOT NULL,
    privileges     TEXT NOT NULL,
    UNIQUE (resource_id, principal)
);
CREATE INDEX permission_by_principal ON permission (principal);

-- One row: the metastore's id and owner.
CREATE TABLE metastore (
    singleton    BIGINT NOT NULL DEFAULT 1 PRIMARY KEY CHECK (singleton = 1),
    metastore_id TEXT NOT NULL,
    owner        TEXT
);

-- Personal access tokens, as the SHA-256 of the token.
CREATE TABLE pat (
    id_string    TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    token_hash   TEXT NOT NULL UNIQUE,
    token_prefix TEXT NOT NULL,
    owner        TEXT NOT NULL,
    created_at   BIGINT NOT NULL,
    expires_at   BIGINT
);
CREATE INDEX pat_by_owner ON pat (owner);

CREATE TABLE scim_user (
    id_string    TEXT PRIMARY KEY,
    user_name    TEXT NOT NULL UNIQUE,
    display_name TEXT,
    external_id  TEXT,
    emails       TEXT,
    active       BOOLEAN NOT NULL,
    picture_url  TEXT,
    created_at   BIGINT,
    updated_at   BIGINT
);
CREATE INDEX scim_user_by_external_id ON scim_user (external_id);

CREATE TABLE scim_group (
    id_string    TEXT PRIMARY KEY,
    display_name TEXT NOT NULL UNIQUE,
    members      TEXT
);
-- The user ids of each group's members, written with `scim_group.members`:
-- the groups whose grants a user gets.
CREATE TABLE scim_group_member (
    group_id TEXT NOT NULL,
    user_id  TEXT NOT NULL,
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX scim_group_member_by_user ON scim_group_member (user_id);

-- Delta Sharing.
CREATE TABLE share (
    id_string  TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    comment    TEXT,
    owner      TEXT,
    created_at BIGINT NOT NULL,
    created_by TEXT,
    updated_at BIGINT NOT NULL,
    updated_by TEXT,
    objects    TEXT
);

CREATE TABLE recipient (
    id_string           TEXT PRIMARY KEY,
    name                TEXT NOT NULL UNIQUE,
    authentication_type TEXT NOT NULL,
    comment             TEXT,
    owner               TEXT,
    created_at          BIGINT NOT NULL,
    created_by          TEXT,
    updated_at          BIGINT,
    updated_by          TEXT,
    token_hash          TEXT UNIQUE,
    token_prefix        TEXT,
    shares              TEXT
);

-- Upstream `uc_storage_cleanup_tasks`: the files of a dropped managed
-- table, volume, model or model version, still to delete. Times are epoch
-- millis from the database clock.
CREATE TABLE storage_cleanup_task (
    id_string        TEXT PRIMARY KEY,
    resource_type    TEXT NOT NULL,
    name             TEXT NOT NULL,
    storage_location TEXT NOT NULL,
    deleted_at       BIGINT NOT NULL,
    lease_expires_at BIGINT,
    lease_token      TEXT,
    failure_count    BIGINT NOT NULL,
    last_error       TEXT
);
CREATE INDEX storage_cleanup_task_by_deleted_at ON storage_cleanup_task (deleted_at);

-- Every mutating request (the audit layer). Append-only.
CREATE TABLE audit_event (
    id_string TEXT PRIMARY KEY,
    principal TEXT NOT NULL,
    method    TEXT NOT NULL,
    path      TEXT NOT NULL,
    status    BIGINT NOT NULL,
    timestamp BIGINT NOT NULL
);
CREATE INDEX audit_event_by_time ON audit_event (timestamp, id_string);
CREATE INDEX audit_event_by_principal ON audit_event (principal);
