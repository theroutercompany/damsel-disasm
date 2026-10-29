//! Versioned schema migrations. `PRAGMA user_version` is the source of
//! truth; every bump ships with a migration and an N-1 fixture test
//! (evidence matrix row 4.4).

use crate::store::StoreError;
use rusqlite::Connection;
use std::path::Path;

/// Apply migrations up to [`crate::SCHEMA_VERSION`].
///
/// Answers the blueprint's five questions per bump:
/// - v1: from 0 (new database). Touches no user annotations (none exist
///   yet). Drops no derived facts. Reanalysis not required. Reversible
///   only by dropping the database (acceptable: v1 has no users).
pub fn migrate(conn: &Connection, path: &Path) -> Result<i32, StoreError> {
    let found: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > crate::SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
            supported: crate::SCHEMA_VERSION,
        });
    }
    if found < 1 {
        v1(conn)?;
    }
    Ok(crate::SCHEMA_VERSION)
}

fn v1(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        r#"
        BEGIN;
        CREATE TABLE project_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE sources (
            source_id        INTEGER PRIMARY KEY,
            kind             TEXT    NOT NULL,
            label            TEXT,
            content_hash     TEXT,
            parent_source_id INTEGER REFERENCES sources(source_id)
        );
        CREATE TABLE source_address_spaces (
            source_id  INTEGER NOT NULL REFERENCES sources(source_id),
            space_name TEXT    NOT NULL,
            ordinal    INTEGER NOT NULL,
            PRIMARY KEY (source_id, space_name)
        );
        CREATE TABLE objects (
            object_id INTEGER PRIMARY KEY,
            source_id INTEGER NOT NULL REFERENCES sources(source_id),
            kind      TEXT    NOT NULL,
            key       TEXT    NOT NULL,
            UNIQUE (source_id, kind, key)
        );
        CREATE TABLE facts (
            fact_id           INTEGER PRIMARY KEY,
            source_id         INTEGER NOT NULL REFERENCES sources(source_id),
            object_id         INTEGER REFERENCES objects(object_id),
            kind              TEXT    NOT NULL,
            payload           TEXT    NOT NULL,
            confidence        TEXT    NOT NULL,
            provenance_pass   TEXT    NOT NULL,
            analyzer_version  TEXT    NOT NULL,
            UNIQUE (source_id, object_id, kind, payload)
        );
        -- Mutation audit log. `ts` lives in the database only and is
        -- excluded from canonical exports by contract.
        CREATE TABLE events (
            seq           INTEGER PRIMARY KEY AUTOINCREMENT,
            actor         TEXT NOT NULL,
            origin        TEXT NOT NULL,
            transaction_id TEXT,
            kind          TEXT NOT NULL,
            payload       TEXT NOT NULL,
            ts            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        PRAGMA user_version = 1;
        COMMIT;
        "#,
    )?;
    Ok(())
}
