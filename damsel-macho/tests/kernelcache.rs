//! Kernelcache (MH_FILESET) tests (plan feature 3, slice 1). A synthetic
//! fileset is crafted around a real fixture binary — the same shape as an
//! Apple kernel collection (primary MH_FILESET header + LC_FILESET_ENTRY
//! records naming embedded Mach-O images) — plus an env-gated hook for
//! real hosts (`DAMSEL_KERNELCACHE_PATH`).

use damsel_macho::{inspect_kernelcache, is_kernelcache_bytes, project_entry, resolve_entry};
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

/// Craft an MH_FILESET kernel collection embedding `payload` images.
fn craft_fileset(payloads: &[(&str, &Path)]) -> PathBuf {
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!(
        "damsel-kc-{}-{}",
        std::process::id(),
        payloads.len()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("kernelcache.test");

    let mut embedded_offsets = Vec::new();
    let mut body = Vec::new();
    for (_, payload) in payloads {
        let bytes = std::fs::read(payload).unwrap();
        // 16 KiB alignment like real fileset entries.
        while body.len() % 0x4000 != 0 {
            body.push(0);
        }
        embedded_offsets.push(body.len() as u64);
        body.extend_from_slice(&bytes);
    }

    // mach_header_64 + one LC_FILESET_ENTRY per payload.
    let mut commands = Vec::new();
    for (index, (entry_id, _)) in payloads.iter().enumerate() {
        let id_bytes = entry_id.as_bytes();
        let mut cmdsize = 32 + id_bytes.len() + 1;
        cmdsize = (cmdsize + 7) & !7; // 8-byte align
        let fileoff = embedded_offsets[index];
        // cmd, cmdsize, vmaddr, fileoff, entry_id(offset from cmd start),
        // reserved
        commands.extend_from_slice(&0x8000_0034u32.to_le_bytes());
        commands.extend_from_slice(&(cmdsize as u32).to_le_bytes());
        commands.extend_from_slice(&(0xffff_fff0_0000_0000u64 + index as u64).to_le_bytes());
        commands.extend_from_slice(&fileoff.to_le_bytes());
        commands.extend_from_slice(&32u32.to_le_bytes());
        commands.extend_from_slice(&0u32.to_le_bytes());
        commands.extend_from_slice(id_bytes);
        commands.push(0);
        while commands.len() % 8 != 0 {
            commands.push(0);
        }
    }

    let mut file = Vec::new();
    // magic cputype cpusubtype filetype=MH_FILESET ncmds sizeofcmds flags
    file.extend_from_slice(&0xfeed_facfu32.to_le_bytes());
    file.extend_from_slice(&0x0100_000cu32.to_le_bytes()); // arm64
    file.extend_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&11u32.to_le_bytes()); // MH_FILESET
    file.extend_from_slice(&(payloads.len() as u32).to_le_bytes());
    file.extend_from_slice(&(commands.len() as u32).to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes()); // reserved
    file.extend_from_slice(&commands);
    while file.len() % 0x4000 != 0 {
        file.push(0);
    }
    // Fix file offsets: entries start after the aligned header block.
    let header_end = file.len() as u64;
    let mut writer = std::fs::File::create(&path).unwrap();
    writer.write_all(&file).unwrap();
    writer.write_all(&body).unwrap();
    drop(writer);

    // Patch each entry's fileoff to header_end + embedded offset.
    let bytes = std::fs::read(&path).unwrap();
    let mut patched = bytes.clone();
    let mut cursor = 32usize;
    for index in 0..payloads.len() {
        let cmdsize =
            u32::from_le_bytes(patched[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let absolute = header_end + embedded_offsets[index];
        patched[cursor + 16..cursor + 24].copy_from_slice(&absolute.to_le_bytes());
        cursor += cmdsize;
    }
    std::fs::write(&path, patched).unwrap();
    path
}

#[test]
fn synthetic_fileset_inventories_and_projects() {
    let kc_path = craft_fileset(&[
        ("com.apple.kernel", &fixture_path("arm64-symbolized")),
        ("com.apple.driver.Foo", &fixture_path("objc-sample")),
    ]);

    let bytes = std::fs::read(&kc_path).unwrap();
    assert!(is_kernelcache_bytes(&bytes));

    let inventory = inspect_kernelcache(&kc_path).expect("inspect kernelcache");
    assert_eq!(inventory.entries.len(), 2, "two fileset entries");
    assert_eq!(inventory.entries[0].entry_id, "com.apple.kernel");
    assert_eq!(inventory.entries[1].entry_id, "com.apple.driver.Foo");
    assert!(inventory.entries[0].vmaddr < inventory.entries[1].vmaddr);

    // Selector ergonomics: exact, substring, ambiguity, miss.
    assert_eq!(
        resolve_entry(&inventory, "com.apple.driver.Foo")
            .unwrap()
            .entry_id,
        "com.apple.driver.Foo"
    );
    assert_eq!(
        resolve_entry(&inventory, "Foo").unwrap().entry_id,
        "com.apple.driver.Foo"
    );
    // "com.apple" is ambiguous across both entries.
    assert!(resolve_entry(&inventory, "com.apple").is_err());
    assert!(resolve_entry(&inventory, "missing").is_err());

    // Projection runs the embedded image through the full pipeline.
    let kernel = project_entry(&inventory, "com.apple.kernel").expect("project kernel");
    assert!(kernel.symbols().iter().any(|symbol| symbol.name == "_main"));
    let driver = project_entry(&inventory, "driver.Foo").expect("project driver");
    assert!(!driver.objc().classes.is_empty(), "objc metadata survives");

    // Non-fileset binaries are refused with a typed error.
    let plain = fixture_path("arm64-symbolized");
    assert!(inspect_kernelcache(&plain).is_err());
}

/// Real-host hook: point DAMSEL_KERNELCACHE_PATH at a boot kernelcache
/// (e.g. /System/Volumes/Preboot/<UUID>/System/Library/KernelCollections/
/// kernelcache) to validate against Apple's own bytes.
#[test]
fn real_kernelcache_env_smoke() {
    let Some(path) = std::env::var_os("DAMSEL_KERNELCACHE_PATH") else {
        return;
    };
    let path = PathBuf::from(path);
    let inventory = inspect_kernelcache(&path).expect("inspect real kernelcache");
    assert!(!inventory.entries.is_empty(), "entries present");
    println!(
        "kernelcache entries: {} (first: {})",
        inventory.entries.len(),
        inventory.entries[0].entry_id
    );
    let first = project_entry(&inventory, &inventory.entries[0].entry_id.clone())
        .expect("project first entry");
    assert!(!first.sections().is_empty());
}
