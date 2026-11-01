//! Persisted shared-cache inventory index (plan feature 2, slice 1).
//!
//! Every cache command pays a full member re-parse on each invocation
//! (~7s on a real macOS 27 cache: 82 members, 4,115 images). The index
//! persists that inventory next to the cache (`<cache>.didx`, SQLite) and
//! `inspect_shared_cache` loads it instead when fresh — turning cache
//! startup into milliseconds.
//!
//! Freshness is strict and cheap: same canonical root path, same member
//! set, and every member's on-disk size unchanged (stat only). Any drift
//! falls back to the full parse; `damsel cache index` rebuilds.
//!
//! The index is a local performance artifact, never an analysis input:
//! projections, symbolication, and linkage queries always re-read cache
//! bytes. Synthetic fixture caches are not indexed.

use crate::errors::MachoError;
use damsel_core::{
    Architecture, CacheImageId, CacheImageRecord, SharedCache, SharedCacheHeader,
    SharedCacheMapping, SharedCacheMember, SharedCacheMemberRole,
};
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheIndexStats {
    pub images: usize,
    pub mappings: usize,
    pub members: usize,
    pub index_path: PathBuf,
}

/// Preferred index location: beside the cache. Read-only cache volumes
/// (system caches) fall back to `DAMSEL_PROJECTS_DIR` or
/// `~/Library/Application Support/damsel/cache-indexes/`.
pub fn index_path_for(cache: &Path) -> PathBuf {
    let file_name = cache
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dyld_shared_cache".to_string());
    if let Some(parent) = cache.parent() {
        let preferred = parent.join(format!("{file_name}.didx"));
        if dir_is_writable(parent) {
            return preferred;
        }
    }
    fallback_root().join(format!("{file_name}.didx"))
}

fn fallback_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("DAMSEL_PROJECTS_DIR") {
        return PathBuf::from(dir).join("cache-indexes");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/damsel/cache-indexes");
    }
    std::env::temp_dir().join("damsel-cache-indexes")
}

fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".damsel-index-probe-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

// JSON-friendly mirrors of the inventory (order-preserving arrays).

#[derive(serde::Serialize, serde::Deserialize)]
struct MemberRow {
    role: String,
    path: String,
    file_size: u64,
    suffix: Option<String>,
    uuid: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MappingRow {
    member_index: usize,
    cache_vmaddr: u64,
    size: u64,
    member_file_offset: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ImageRow {
    cache_uuid: String,
    image_id_index: u32,
    image_index: u32,
    install_name: String,
    basename: String,
    image_base_vmaddr: u64,
    image_size: u64,
    member_index: usize,
}

fn role_to_string(role: SharedCacheMemberRole) -> &'static str {
    match role {
        SharedCacheMemberRole::Root => "root",
        SharedCacheMemberRole::Subcache => "subcache",
        SharedCacheMemberRole::Symbols => "symbols",
    }
}

fn role_from_string(value: &str) -> Option<SharedCacheMemberRole> {
    match value {
        "root" => Some(SharedCacheMemberRole::Root),
        "subcache" => Some(SharedCacheMemberRole::Subcache),
        "symbols" => Some(SharedCacheMemberRole::Symbols),
        _ => None,
    }
}

fn architecture_from_string(value: &str) -> Option<Architecture> {
    match value {
        "arm64" => Some(Architecture::Arm64),
        "arm64e" => Some(Architecture::Arm64e),
        "x86_64" => Some(Architecture::X86_64),
        _ => None,
    }
}

fn open_index(path: &Path, create: bool) -> Result<Connection, MachoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            MachoError::MalformedSharedCache(format!(
                "failed to create index directory {}: {error}",
                path.display()
            ))
        })?;
    }
    let conn = Connection::open(path).map_err(|error| {
        MachoError::MalformedSharedCache(format!(
            "failed to open cache index {}: {error}",
            path.display()
        ))
    })?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(sql_error)?;
    if create {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS members (
                 path TEXT PRIMARY KEY,
                 size INTEGER NOT NULL
             );
             PRAGMA user_version = 1;",
        )
        .map_err(sql_error)?;
    }
    Ok(conn)
}

fn sql_error(error: rusqlite::Error) -> MachoError {
    MachoError::MalformedSharedCache(format!("cache index error: {error}"))
}

/// Persist the session's inventory as an index.
pub fn build_index(cache: &SharedCache, cache_path: &Path) -> Result<CacheIndexStats, MachoError> {
    let index_path = index_path_for(cache_path);
    let mut conn = open_index(&index_path, true)?;
    let tx = conn.transaction().map_err(sql_error)?;

    let header = cache.header();
    let members: Vec<MemberRow> = cache
        .members()
        .iter()
        .map(|member| MemberRow {
            role: role_to_string(member.role).to_string(),
            path: member.path.display().to_string(),
            file_size: member.file_size,
            suffix: member.suffix.clone(),
            uuid: member.uuid.clone(),
        })
        .collect();
    let mappings: Vec<MappingRow> = cache
        .mappings()
        .iter()
        .map(|mapping| MappingRow {
            member_index: mapping.member_index,
            cache_vmaddr: mapping.cache_vmaddr,
            size: mapping.size,
            member_file_offset: mapping.member_file_offset,
        })
        .collect();
    let images: Vec<ImageRow> = cache
        .images()
        .iter()
        .map(|image| ImageRow {
            cache_uuid: image.id.cache_uuid().to_string(),
            image_id_index: image.id.image_index(),
            image_index: image.image_index,
            install_name: image.install_name.clone(),
            basename: image.basename.clone(),
            image_base_vmaddr: image.image_base_vmaddr,
            image_size: image.image_size,
            member_index: image.member_index,
        })
        .collect();

    let insert = |key: &str, value: &str| -> Result<(), MachoError> {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )
        .map_err(sql_error)?;
        Ok(())
    };

    insert("schema_version", &SCHEMA_VERSION.to_string())?;
    insert(
        "root_path",
        &std::fs::canonicalize(cache_path)
            .unwrap_or_else(|_| cache_path.to_path_buf())
            .display()
            .to_string(),
    )?;
    insert("cache_uuid", &header.cache_uuid)?;
    insert("architecture", &header.architecture.to_string())?;
    insert(
        "has_local_symbols",
        &u8::from(header.has_local_symbols).to_string(),
    )?;
    insert(
        "members_json",
        &serde_json::to_string(&members).map_err(|error| {
            MachoError::MalformedSharedCache(format!("cache index encode error: {error}"))
        })?,
    )?;
    insert(
        "mappings_json",
        &serde_json::to_string(&mappings).map_err(|error| {
            MachoError::MalformedSharedCache(format!("cache index encode error: {error}"))
        })?,
    )?;
    insert(
        "images_json",
        &serde_json::to_string(&images).map_err(|error| {
            MachoError::MalformedSharedCache(format!("cache index encode error: {error}"))
        })?,
    )?;

    tx.execute("DELETE FROM members", []).map_err(sql_error)?;
    for member in &members {
        tx.execute(
            "INSERT INTO members (path, size) VALUES (?1, ?2)",
            rusqlite::params![member.path, member.file_size as i64],
        )
        .map_err(sql_error)?;
    }

    tx.commit().map_err(sql_error)?;
    Ok(CacheIndexStats {
        images: images.len(),
        mappings: mappings.len(),
        members: members.len(),
        index_path,
    })
}

/// Load the inventory from the index when it is fresh for `cache_path`;
/// `None` falls back to the full member parse. Freshness = same canonical
/// root path, same member set, and every member size unchanged on disk.
pub fn try_load_fresh(cache_path: &Path) -> Option<SharedCache> {
    let index_path = index_path_for(cache_path);
    if !index_path.is_file() {
        return None;
    }
    let conn = match Connection::open(&index_path) {
        Ok(conn) => conn,
        Err(_) => return None,
    };
    let get = |key: &str| -> Option<String> {
        conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()
    };

    if get("schema_version")?.parse::<i64>().ok()? != SCHEMA_VERSION {
        return None;
    }
    let canonical_root = std::fs::canonicalize(cache_path)
        .unwrap_or_else(|_| cache_path.to_path_buf())
        .display()
        .to_string();
    if get("root_path")? != canonical_root {
        return None;
    }

    // Member freshness: exact set + sizes, stat only.
    let indexed_members: Vec<(String, i64)> = {
        let mut statement = conn.prepare("SELECT path, size FROM members").ok()?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .ok()?
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        rows
    };
    if indexed_members.is_empty() {
        return None;
    }
    for (path, size) in &indexed_members {
        let metadata = std::fs::metadata(path).ok()?;
        if metadata.len() as i64 != *size {
            return None;
        }
    }

    let header = SharedCacheHeader {
        cache_uuid: get("cache_uuid")?,
        architecture: architecture_from_string(&get("architecture")?)?,
        mapping_count: get("mappings_json")
            .and_then(|json| serde_json::from_str::<Vec<MappingRow>>(&json).ok())
            .map(|rows| rows.len() as u32)?,
        image_count: get("images_json")
            .and_then(|json| serde_json::from_str::<Vec<ImageRow>>(&json).ok())
            .map(|rows| rows.len() as u32)?,
        base_address: get("mappings_json")
            .and_then(|json| serde_json::from_str::<Vec<MappingRow>>(&json).ok())
            .and_then(|rows| rows.iter().map(|row| row.cache_vmaddr).min()),
        has_local_symbols: get("has_local_symbols")?.parse::<u8>().ok()? == 1,
    };

    let member_rows: Vec<MemberRow> = serde_json::from_str(&get("members_json")?).ok()?;
    let members: Vec<SharedCacheMember> = member_rows
        .into_iter()
        .map(|row: MemberRow| {
            Some(SharedCacheMember {
                role: role_from_string(&row.role)?,
                path: PathBuf::from(row.path),
                file_size: row.file_size,
                suffix: row.suffix,
                uuid: row.uuid,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if members.len() != indexed_members.len() {
        return None;
    }
    let mapping_rows: Vec<MappingRow> = serde_json::from_str(&get("mappings_json")?).ok()?;
    let mappings: Vec<SharedCacheMapping> = mapping_rows
        .into_iter()
        .map(|row: MappingRow| SharedCacheMapping {
            member_index: row.member_index,
            cache_vmaddr: row.cache_vmaddr,
            size: row.size,
            member_file_offset: row.member_file_offset,
        })
        .collect();
    let image_rows: Vec<ImageRow> = serde_json::from_str(&get("images_json")?).ok()?;
    let images: Vec<CacheImageRecord> = image_rows
        .into_iter()
        .map(|row: ImageRow| CacheImageRecord {
            id: CacheImageId::new(row.cache_uuid, row.image_id_index),
            image_index: row.image_index,
            install_name: row.install_name,
            basename: row.basename,
            image_base_vmaddr: row.image_base_vmaddr,
            image_size: row.image_size,
            member_index: row.member_index,
        })
        .collect();

    Some(SharedCache::from_file_inventory(
        PathBuf::from(canonical_root),
        header,
        members,
        mappings,
        images,
    ))
}
