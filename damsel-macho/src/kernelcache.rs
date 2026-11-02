//! Kernelcache (MH_FILESET) support (plan feature 3, slice 1): inventory
//! and entry projection for Apple kernel collections.
//!
//! A modern kernelcache is a primary Mach-O with filetype `MH_FILESET`
//! whose load commands are `LC_FILESET_ENTRY` records — each names one
//! embedded Mach-O (the kernel itself plus KEXTs) with its vmaddr and
//! file offset. This module inventories those entries and projects any
//! of them through the standalone Mach-O pipeline (`load_bytes`), the
//! same shape as the dyld shared-cache projection but file-embedded.
//!
//! Honesty: v1 lists and projects entries; KEXT-typing (OSMetaClass
//! hierarchies, kalloc_type) is later work on this foundation.

use crate::errors::MachoError;
use damsel_core::BinaryImage;
use std::path::{Path, PathBuf};

/// MH_FILESET filetype.
const MH_FILESET: u32 = 11;
/// LC_FILESET_ENTRY.
const LC_FILESET_ENTRY: u32 = 0x34 | 0x8000_0000;

/// One embedded image (kernel or KEXT) named by an LC_FILESET_ENTRY.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KernelcacheEntry {
    pub entry_id: String,
    pub vmaddr: u64,
    pub fileoff: u64,
}

/// Parsed kernel collection inventory.
#[derive(Debug, Clone)]
pub struct KernelcacheInventory {
    pub path: PathBuf,
    pub file_size: u64,
    pub entries: Vec<KernelcacheEntry>,
}

fn le_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn le_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;
    let mut value = [0u8; 8];
    value.copy_from_slice(bytes);
    Some(u64::from_le_bytes(value))
}

/// True when the bytes start a 64-bit MH_FILESET Mach-O.
pub fn is_kernelcache_bytes(bytes: &[u8]) -> bool {
    le_u32(bytes, 0) == Some(0xfeed_facf) && le_u32(bytes, 12) == Some(MH_FILESET)
}

/// Parse a kernel collection's entries. Errors are typed for anything
/// that is not an MH_FILESET image.
pub fn inspect_kernelcache(path: &Path) -> Result<KernelcacheInventory, MachoError> {
    let bytes = std::fs::read(path).map_err(|error| {
        MachoError::MalformedSharedCache(format!("kernelcache read {}: {error}", path.display()))
    })?;
    inspect_kernelcache_bytes(path, &bytes)
}

/// Byte-level inventory parse (test surface and future mmap callers).
pub fn inspect_kernelcache_bytes(
    path: &Path,
    bytes: &[u8],
) -> Result<KernelcacheInventory, MachoError> {
    if !is_kernelcache_bytes(bytes) {
        return Err(MachoError::MalformedSharedCache(format!(
            "{} is not an MH_FILESET kernelcache",
            path.display()
        )));
    }
    let ncmds = le_u32(bytes, 16).ok_or_else(truncated())? as usize;
    let mut offset = 32usize; // mach_header_64
    let mut entries = Vec::new();
    for _ in 0..ncmds {
        let (cmd, cmdsize) = (
            le_u32(bytes, offset).ok_or_else(truncated())?,
            le_u32(bytes, offset + 4).ok_or_else(truncated())? as usize,
        );
        if cmdsize < 8 || offset + cmdsize > bytes.len() {
            return Err(truncated()());
        }
        if cmd == LC_FILESET_ENTRY && cmdsize >= 32 {
            let vmaddr = le_u64(bytes, offset + 8).unwrap_or_default();
            let fileoff = le_u64(bytes, offset + 16).unwrap_or_default();
            let entry_id_offset = le_u32(bytes, offset + 24).unwrap_or_default() as usize;
            // LcStr offsets are relative to the command start.
            let id_start = offset.saturating_add(entry_id_offset);
            let id = bytes
                .get(id_start..)
                .and_then(|tail| {
                    tail.iter()
                        .position(|byte| *byte == 0)
                        .map(|end| tail[..end].to_vec())
                })
                .filter(|raw| !raw.is_empty() && raw.len() <= 256)
                .map(|raw| String::from_utf8_lossy(&raw).into_owned())
                .unwrap_or_default();
            entries.push(KernelcacheEntry {
                entry_id: id,
                vmaddr,
                fileoff,
            });
        }
        offset += cmdsize;
    }
    Ok(KernelcacheInventory {
        path: path.to_path_buf(),
        file_size: bytes.len() as u64,
        entries,
    })
}

fn truncated() -> impl Fn() -> MachoError {
    || MachoError::MalformedSharedCache("truncated kernelcache".to_string())
}

/// Resolve an entry by id substring (exact match first, then unique
/// substring), mirroring shared-cache selector ergonomics.
pub fn resolve_entry<'a>(
    inventory: &'a KernelcacheInventory,
    selector: &str,
) -> Result<&'a KernelcacheEntry, MachoError> {
    if let Some(exact) = inventory
        .entries
        .iter()
        .find(|entry| entry.entry_id == selector)
    {
        return Ok(exact);
    }
    let matches: Vec<&KernelcacheEntry> = inventory
        .entries
        .iter()
        .filter(|entry| entry.entry_id.contains(selector))
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(MachoError::CacheImageNotFound(selector.to_string())),
        many => Err(MachoError::CacheImageAmbiguous(format!(
            "{selector} => {}",
            many.iter()
                .map(|entry| entry.entry_id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Project one embedded entry through the standalone Mach-O pipeline.
/// The slice runs from the entry's file offset to the next entry (or
/// EOF), which is how fileset entries partition the file.
pub fn project_entry(
    inventory: &KernelcacheInventory,
    selector: &str,
) -> Result<BinaryImage, MachoError> {
    let entry = resolve_entry(inventory, selector)?;
    let bytes = std::fs::read(&inventory.path).map_err(|error| {
        MachoError::MalformedSharedCache(format!(
            "kernelcache read {}: {error}",
            inventory.path.display()
        ))
    })?;
    let start = usize::try_from(entry.fileoff)
        .map_err(|_| MachoError::MalformedSharedCache("entry offset overflow".to_string()))?;
    let end = inventory
        .entries
        .iter()
        .map(|other| other.fileoff)
        .filter(|offset| *offset > entry.fileoff)
        .min()
        .unwrap_or(inventory.file_size);
    let end = usize::try_from(end)
        .map_err(|_| MachoError::MalformedSharedCache("entry end overflow".to_string()))?;
    let slice = bytes
        .get(start..end.min(bytes.len()))
        .ok_or_else(|| MachoError::MalformedSharedCache("entry slice out of range".to_string()))?;
    crate::loader::load_bytes(
        Some(format!("kernelcache:{}", entry.entry_id)),
        slice.to_vec(),
    )
}
