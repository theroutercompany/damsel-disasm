use crate::errors::{MachoError, Result};
use crate::loader::{load, load_bytes};
use crate::shared_cache_query::{
    CacheLookupResult as QueryLookupResult, ProjectedCacheImage, SharedCacheQueryEngine,
    SharedCacheQueryError,
};
use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, CacheDependentRecord, CacheImageDependencyRecord,
    CacheImageId, CacheImageRecord, CacheLookupResult, CacheMappingContext, CacheReexportRecord,
    CacheSymbolImporterRecord, CacheSymbolProviderKind, CacheSymbolProviderRecord,
    CacheSymbolSource, ExportFlags, ObjcMetadata, ProjectedBinaryImage, ProjectedImageProvenance,
    SharedCache, SharedCacheHeader, SharedCacheMapping, SharedCacheMember, SharedCacheMemberRole,
    SharedCacheSource, SliceInfo, SymbolicationMatch,
};
use object::macho::DyldCacheHeader;
use object::read::macho::DyldCache;
use object::read::{Export, Object, ObjectSegment, ObjectSymbol};
use object::{Architecture as ObjectArchitecture, Endianness as ObjectEndianness, FileKind};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DYLD_MAGIC_ARM64: [u8; 16] = *b"dyld_v1   arm64\0";
const DYLD_MAGIC_ARM64E: [u8; 16] = *b"dyld_v1  arm64e\0";

const SYNTHETIC_CACHE_MAGIC: &str = "SYNTHETIC_DYLD_CACHE";
const SYNTHETIC_SUBCACHE_MAGIC: &str = "SYNTHETIC_DYLD_CACHE_SUBCACHE";
const SYNTHETIC_SYMBOLS_MAGIC: &str = "SYNTHETIC_DYLD_CACHE_SYMBOLS";
const DEFAULT_SYNTHETIC_IMAGE_SIZE: u64 = 0x10_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCacheExportRecord {
    pub name: String,
    pub cache_vmaddr: Option<u64>,
    pub kind: String,
    pub flags: String,
}

#[derive(Debug)]
pub struct SharedCacheSession {
    cache: SharedCache,
    kind: SharedCacheSessionKind,
    import_relations: RefCell<Option<CacheImportRelations>>,
    export_relations: RefCell<Option<CacheExportRelations>>,
}

#[derive(Debug)]
enum SharedCacheSessionKind {
    Synthetic {
        engine: SharedCacheQueryEngine,
        projections: RefCell<BTreeMap<CacheImageId, ProjectedBinaryImage>>,
    },
    Real {
        root_path: PathBuf,
        projections: RefCell<BTreeMap<CacheImageId, ProjectedBinaryImage>>,
    },
}

#[derive(Debug)]
struct RealCacheDiscovery {
    root_path: PathBuf,
    source: SharedCacheSource,
    root_uuid: [u8; 16],
    architecture: Architecture,
    members: Vec<RealCacheMember>,
}

#[derive(Debug)]
struct RealCacheMember {
    path: PathBuf,
    role: SharedCacheMemberRole,
    suffix: Option<String>,
    bytes: Arc<[u8]>,
    uuid: [u8; 16],
}

#[derive(Debug)]
struct ParsedMemberHeader {
    architecture: Architecture,
    uuid: [u8; 16],
}

#[derive(Debug, Clone)]
struct SyntheticMemberSpec {
    role: SharedCacheMemberRole,
    path: PathBuf,
    suffix: Option<String>,
    text: String,
}

pub fn load_shared_cache<P: AsRef<Path>>(path: P) -> Result<SharedCache> {
    inspect_shared_cache(path).map(|session| session.into_cache())
}

pub fn inspect_shared_cache<P: AsRef<Path>>(path: P) -> Result<SharedCacheSession> {
    let input_path = path.as_ref();
    let root_candidate = derive_root_candidate(input_path);
    let root_bytes = read_member_bytes(&root_candidate)?;
    if is_synthetic_cache_bytes(root_bytes.as_ref()) {
        return build_synthetic_session(input_path, &root_candidate, root_bytes);
    }
    let (cache, root_path) = load_real_shared_cache_inventory(input_path, &root_candidate)?;
    Ok(SharedCacheSession {
        cache,
        kind: SharedCacheSessionKind::Real {
            root_path,
            projections: RefCell::new(BTreeMap::new()),
        },
        import_relations: RefCell::new(None),
        export_relations: RefCell::new(None),
    })
}

impl SharedCacheSession {
    pub fn cache(&self) -> &SharedCache {
        &self.cache
    }

    pub fn into_cache(self) -> SharedCache {
        self.cache
    }

    pub fn project_image(&self, selector: &str) -> Result<ProjectedBinaryImage> {
        let image = self.resolve_image(selector)?;
        let image_id = image.id.clone();
        if let Some(projected) = self.cached_projection(&image_id) {
            return Ok(projected);
        }

        let projected = match &self.kind {
            SharedCacheSessionKind::Synthetic { engine, .. } => {
                let projected = engine
                    .project_image(selector)
                    .map_err(map_query_error_to_macho_error)?;
                ProjectedBinaryImage {
                    provenance: projected_image_provenance(
                        &self.cache,
                        image,
                        projected.local_symbols_available,
                    ),
                    image: projected.binary_image.clone(),
                }
            }
            SharedCacheSessionKind::Real { root_path, .. } => {
                self.project_real_image(root_path, image)?
            }
        };

        self.store_projection(image_id, projected.clone());
        Ok(projected)
    }

    pub fn resolve_image(&self, selector: &str) -> Result<&CacheImageRecord> {
        if let Some(image) = self.cache.image_by_id_str(selector) {
            return Ok(image);
        }
        if let Some(image) = self.cache.image_by_install_name(selector) {
            return Ok(image);
        }

        let basename = install_name_basename(selector);
        let matches = self.cache.images_by_basename(basename);
        match matches.len() {
            1 => Ok(matches[0]),
            0 => Err(MachoError::CacheImageNotFound(selector.to_string())),
            _ => Err(MachoError::CacheImageAmbiguous(format!(
                "{} => {}",
                selector,
                matches
                    .into_iter()
                    .map(|image| image.install_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    pub fn exports_for_image(&self, selector: &str) -> Result<Vec<SharedCacheExportRecord>> {
        match &self.kind {
            SharedCacheSessionKind::Synthetic { engine, .. } => {
                let projected = engine
                    .project_image(selector)
                    .map_err(map_query_error_to_macho_error)?;
                let mut exports = projected
                    .binary_image
                    .dyld()
                    .exported_symbols
                    .iter()
                    .map(|record| SharedCacheExportRecord {
                        name: record.name.clone(),
                        cache_vmaddr: record.address,
                        kind: export_kind_name(&record.kind).to_string(),
                        flags: record.flags.to_string(),
                    })
                    .collect::<Vec<_>>();
                exports.sort_by_key(|record| (record.cache_vmaddr, record.name.clone()));
                Ok(exports)
            }
            SharedCacheSessionKind::Real { root_path, .. } => {
                let image = self.resolve_image(selector)?;
                with_real_cache(root_path, |cache| {
                    let dyld_image = dyld_cache_image_by_index(cache, image.image_index as usize)?;
                    let object_file =
                        dyld_image.parse_object().map_err(|error: object::Error| {
                            MachoError::MalformedSharedCache(error.to_string())
                        })?;
                    let mut exports = object_file
                        .exports()
                        .map_err(|error: object::Error| {
                            MachoError::MalformedSharedCache(error.to_string())
                        })?
                        .into_iter()
                        .map(|export: Export<'_>| SharedCacheExportRecord {
                            name: String::from_utf8_lossy(export.name()).into_owned(),
                            cache_vmaddr: Some(export.address()),
                            kind: "regular".to_string(),
                            flags: ExportFlags::from_bits(0).to_string(),
                        })
                        .collect::<Vec<_>>();
                    exports.sort_by_key(|record| (record.cache_vmaddr, record.name.clone()));
                    Ok(exports)
                })
            }
        }
    }

    pub fn lookup_cache_vmaddr(&self, cache_vmaddr: u64) -> Result<CacheLookupResult> {
        match &self.kind {
            SharedCacheSessionKind::Synthetic { engine, .. } => {
                let result = engine
                    .lookup_cache_vmaddr(cache_vmaddr)
                    .map_err(map_query_error_to_macho_error)?;
                Ok(convert_query_lookup_result(&self.cache, result))
            }
            SharedCacheSessionKind::Real { root_path, .. } => {
                let mapping = self
                    .cache
                    .mapping_for_vm_address(cache_vmaddr)
                    .ok_or(MachoError::AddressNotMapped(cache_vmaddr))?;
                let member_file_offset = mapping
                    .member_file_offset_for_vmaddr(cache_vmaddr)
                    .ok_or_else(|| {
                        MachoError::MalformedSharedCache(format!(
                            "mapping file offset overflow for address {cache_vmaddr:#x}"
                        ))
                    })?;
                let Some(image) = self.cache.image_for_vm_address(cache_vmaddr) else {
                    return Ok(CacheLookupResult::MappingOnly {
                        cache_vmaddr,
                        mapping: cache_mapping_context(&self.cache, mapping, member_file_offset),
                        image: None,
                    });
                };
                let mut candidates = self.real_symbol_candidates(root_path, image, true)?;
                candidates.retain(|candidate| candidate.cache_vmaddr <= cache_vmaddr);
                candidates.sort_by_key(|candidate| {
                    (
                        std::cmp::Reverse(candidate.cache_vmaddr),
                        candidate.name.clone(),
                    )
                });
                if let Some(best) = candidates.first() {
                    let exact = best.cache_vmaddr == cache_vmaddr;
                    let symbol =
                        symbolication_match_from_candidate(&self.cache, image, best, exact);
                    if exact {
                        Ok(CacheLookupResult::ExactSymbol {
                            cache_vmaddr,
                            mapping: cache_mapping_context(
                                &self.cache,
                                mapping,
                                member_file_offset,
                            ),
                            image: projected_image_provenance(
                                &self.cache,
                                image,
                                self.cache.header().has_local_symbols,
                            ),
                            symbol,
                        })
                    } else {
                        Ok(CacheLookupResult::NearestSymbol {
                            cache_vmaddr,
                            mapping: cache_mapping_context(
                                &self.cache,
                                mapping,
                                member_file_offset,
                            ),
                            image: projected_image_provenance(
                                &self.cache,
                                image,
                                self.cache.header().has_local_symbols,
                            ),
                            distance: cache_vmaddr.saturating_sub(best.cache_vmaddr),
                            symbol,
                        })
                    }
                } else {
                    Ok(CacheLookupResult::MappingOnly {
                        cache_vmaddr,
                        mapping: cache_mapping_context(&self.cache, mapping, member_file_offset),
                        image: Some(projected_image_provenance(
                            &self.cache,
                            image,
                            self.cache.header().has_local_symbols,
                        )),
                    })
                }
            }
        }
    }

    pub fn resolve_exact_symbol(
        &self,
        symbol_name: &str,
        image_filter: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SymbolicationMatch>> {
        match &self.kind {
            SharedCacheSessionKind::Synthetic { engine, .. } => {
                let results = engine
                    .resolve_exact_symbol(symbol_name, image_filter, limit)
                    .map_err(map_query_error_to_macho_error)?;
                Ok(results
                    .into_iter()
                    .map(|entry| convert_query_match(&self.cache, entry, true, None))
                    .collect())
            }
            SharedCacheSessionKind::Real { root_path, .. } => {
                let limit = limit.unwrap_or(usize::MAX);
                let mut matches = Vec::new();
                for image in self.cache.images() {
                    if let Some(filter) = image_filter {
                        if !contains_ascii_case_insensitive(&image.install_name, filter)
                            && !contains_ascii_case_insensitive(&image.id.to_string(), filter)
                            && !contains_ascii_case_insensitive(&image.basename, filter)
                        {
                            continue;
                        }
                    }
                    for candidate in self.real_symbol_candidates(root_path, image, false)? {
                        if candidate.name == symbol_name {
                            matches.push(symbolication_match_from_candidate(
                                &self.cache,
                                image,
                                &candidate,
                                true,
                            ));
                        }
                    }
                }
                matches.sort_by_key(|entry| {
                    (
                        entry.cache_vmaddr,
                        entry.image_install_name.clone(),
                        entry.symbol_name.clone(),
                    )
                });
                matches.dedup_by(|left, right| {
                    left.cache_vmaddr == right.cache_vmaddr
                        && left.image_id == right.image_id
                        && left.symbol_name == right.symbol_name
                });
                if matches.is_empty() {
                    return Err(MachoError::SymbolNotFound(symbol_name.to_string()));
                }
                matches.truncate(limit.min(matches.len()));
                Ok(matches)
            }
        }
    }

    pub fn image_dependencies(&self, selector: &str) -> Result<Vec<CacheImageDependencyRecord>> {
        let image = self.resolve_image(selector)?;
        let relations = self.import_relations()?;
        Ok(relations
            .dependencies_by_image
            .get(&image.id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn dependents(&self, selector: &str) -> Result<Vec<CacheDependentRecord>> {
        let image = self.resolve_image(selector)?;
        let relations = self.import_relations()?;
        Ok(relations
            .dependents_by_image
            .get(&image.id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn symbol_providers(&self, symbol_name: &str) -> Result<Vec<CacheSymbolProviderRecord>> {
        let relations = self.export_relations()?;
        let matches = relations
            .providers_by_symbol
            .get(symbol_name)
            .cloned()
            .unwrap_or_default();
        if matches.is_empty() {
            return Err(MachoError::SymbolNotFound(symbol_name.to_string()));
        }
        Ok(matches)
    }

    pub fn symbol_importers(&self, symbol_name: &str) -> Result<Vec<CacheSymbolImporterRecord>> {
        let import_relations = self.import_relations()?;
        let export_relations = self.export_relations()?;
        let edges = import_relations
            .importers_by_symbol
            .get(symbol_name)
            .cloned()
            .unwrap_or_default();
        if edges.is_empty() {
            return Err(MachoError::SymbolNotFound(symbol_name.to_string()));
        }

        let providers = export_relations
            .providers_by_symbol
            .get(symbol_name)
            .cloned()
            .unwrap_or_default();
        let mut records = edges
            .into_iter()
            .map(|edge| CacheSymbolImporterRecord {
                importer_image: edge.importer_image.clone(),
                symbol_name: edge.symbol_name.clone(),
                dylib_name: edge.dylib_name.clone(),
                import_binding_kind: edge.import_binding_kind,
                import_binding_source: edge.import_binding_source,
                resolved_provider_image: uniquely_resolved_provider_image(
                    &providers,
                    &edge.dylib_name,
                ),
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| {
            (
                record.importer_image.install_name.clone(),
                record.dylib_name.clone(),
                record.symbol_name.clone(),
                format!("{:?}", record.import_binding_kind),
                format!("{:?}", record.import_binding_source),
            )
        });
        records.dedup_by(|left, right| {
            left.importer_image.image_id == right.importer_image.image_id
                && left.symbol_name == right.symbol_name
                && left.dylib_name == right.dylib_name
                && left.import_binding_kind == right.import_binding_kind
                && left.import_binding_source == right.import_binding_source
        });
        Ok(records)
    }

    pub fn reexports(&self, selector: &str) -> Result<Vec<CacheReexportRecord>> {
        let image = self.resolve_image(selector)?;
        let relations = self.export_relations()?;
        Ok(relations
            .reexports_by_image
            .get(&image.id)
            .cloned()
            .unwrap_or_default())
    }

    fn real_symbol_candidates(
        &self,
        root_path: &Path,
        image: &CacheImageRecord,
        include_nearest_candidates: bool,
    ) -> Result<Vec<RealSymbolCandidate>> {
        with_real_cache(root_path, |cache| {
            let dyld_image = dyld_cache_image_by_index(cache, image.image_index as usize)?;
            let object_file = dyld_image.parse_object().map_err(|error: object::Error| {
                MachoError::MalformedSharedCache(error.to_string())
            })?;
            let mut candidates = object_file
                .exports()
                .map_err(|error: object::Error| {
                    MachoError::MalformedSharedCache(error.to_string())
                })?
                .into_iter()
                .map(|export: Export<'_>| RealSymbolCandidate {
                    name: String::from_utf8_lossy(export.name()).into_owned(),
                    cache_vmaddr: export.address(),
                    source: CacheSymbolSource::Export,
                })
                .collect::<Vec<_>>();
            if self.cache.header().has_local_symbols || include_nearest_candidates {
                for symbol in object_file.symbols() {
                    if !symbol.is_definition() {
                        continue;
                    }
                    let Ok(name) = symbol.name() else {
                        continue;
                    };
                    candidates.push(RealSymbolCandidate {
                        name: name.to_string(),
                        cache_vmaddr: symbol.address(),
                        source: CacheSymbolSource::Local,
                    });
                }
            }
            Ok(candidates)
        })
    }

    fn import_relations(&self) -> Result<CacheImportRelations> {
        if let Some(existing) = self.import_relations.borrow().as_ref().cloned() {
            return Ok(existing);
        }
        let built = self.build_import_relations()?;
        *self.import_relations.borrow_mut() = Some(built.clone());
        Ok(built)
    }

    fn export_relations(&self) -> Result<CacheExportRelations> {
        if let Some(existing) = self.export_relations.borrow().as_ref().cloned() {
            return Ok(existing);
        }
        let built = self.build_export_relations()?;
        *self.export_relations.borrow_mut() = Some(built.clone());
        Ok(built)
    }

    fn build_import_relations(&self) -> Result<CacheImportRelations> {
        let mut dependencies_by_image = BTreeMap::new();
        let mut dependents_acc = BTreeMap::<
            CacheImageId,
            BTreeMap<CacheImageId, (ProjectedImageProvenance, usize)>,
        >::new();
        let mut importers_by_symbol = BTreeMap::<String, Vec<ImportEdge>>::new();

        for cache_image in self.cache.images() {
            let projected = self.project_image(&cache_image.id.to_string())?;
            let source = projected.provenance.clone();

            let mut dependency_counts = BTreeMap::<String, usize>::new();
            for dylib in &projected.image.dyld().imported_dylibs {
                dependency_counts.entry(dylib.clone()).or_insert(0);
            }
            for import in projected.image.imports() {
                *dependency_counts.entry(import.dylib.clone()).or_insert(0) += 1;
            }

            let mut seen_edges = BTreeMap::<String, ()>::new();
            for binding in &projected.image.dyld().import_bindings {
                dependency_counts.entry(binding.dylib.clone()).or_insert(0);
                let edge = ImportEdge {
                    importer_image: source.clone(),
                    symbol_name: binding.name.clone(),
                    dylib_name: binding.dylib.clone(),
                    import_binding_kind: Some(binding.binding_kind),
                    import_binding_source: Some(binding.source),
                };
                let dedup_key = format!(
                    "{}|{}|{}|{:?}|{:?}",
                    edge.importer_image.image_id,
                    edge.dylib_name,
                    edge.symbol_name,
                    edge.import_binding_kind,
                    edge.import_binding_source
                );
                if seen_edges.insert(dedup_key, ()).is_none() {
                    importers_by_symbol
                        .entry(edge.symbol_name.clone())
                        .or_default()
                        .push(edge);
                }
            }
            for import in projected.image.imports() {
                let edge = ImportEdge {
                    importer_image: source.clone(),
                    symbol_name: import.name.clone(),
                    dylib_name: import.dylib.clone(),
                    import_binding_kind: None,
                    import_binding_source: None,
                };
                let dedup_key = format!(
                    "{}|{}|{}|{:?}|{:?}",
                    edge.importer_image.image_id,
                    edge.dylib_name,
                    edge.symbol_name,
                    edge.import_binding_kind,
                    edge.import_binding_source
                );
                if seen_edges.insert(dedup_key, ()).is_none() {
                    importers_by_symbol
                        .entry(edge.symbol_name.clone())
                        .or_default()
                        .push(edge);
                }
            }

            let mut dependencies = dependency_counts
                .into_iter()
                .map(|(target_dylib_install_name, reference_count)| {
                    let target_image = self
                        .cache
                        .image_by_install_name(&target_dylib_install_name)
                        .map(|image| {
                            projected_image_provenance(
                                &self.cache,
                                image,
                                self.cache.header().has_local_symbols,
                            )
                        });
                    CacheImageDependencyRecord {
                        source_image: source.clone(),
                        target_dylib_install_name,
                        within_cache: target_image.is_some(),
                        target_image,
                        reference_count,
                    }
                })
                .collect::<Vec<_>>();
            dependencies.sort_by_key(|record| record.target_dylib_install_name.clone());

            for dependency in &dependencies {
                if let Some(target_image) = &dependency.target_image {
                    dependents_acc
                        .entry(target_image.image_id.clone())
                        .or_default()
                        .entry(source.image_id.clone())
                        .and_modify(|(_, count)| *count += dependency.reference_count)
                        .or_insert((source.clone(), dependency.reference_count));
                }
            }

            dependencies_by_image.insert(source.image_id.clone(), dependencies);
        }

        let mut dependents_by_image = BTreeMap::new();
        for (target_id, dependents) in dependents_acc {
            let mut records = dependents
                .into_values()
                .map(|(dependent_image, dependency_count)| CacheDependentRecord {
                    dependent_image,
                    dependency_count,
                })
                .collect::<Vec<_>>();
            records.sort_by_key(|record| record.dependent_image.install_name.clone());
            dependents_by_image.insert(target_id, records);
        }
        for edges in importers_by_symbol.values_mut() {
            edges.sort_by_key(|edge| {
                (
                    edge.importer_image.install_name.clone(),
                    edge.dylib_name.clone(),
                    edge.symbol_name.clone(),
                    format!("{:?}", edge.import_binding_kind),
                    format!("{:?}", edge.import_binding_source),
                )
            });
            edges.dedup_by(|left, right| {
                left.importer_image.image_id == right.importer_image.image_id
                    && left.dylib_name == right.dylib_name
                    && left.symbol_name == right.symbol_name
                    && left.import_binding_kind == right.import_binding_kind
                    && left.import_binding_source == right.import_binding_source
            });
        }

        Ok(CacheImportRelations {
            dependencies_by_image,
            dependents_by_image,
            importers_by_symbol,
        })
    }

    fn build_export_relations(&self) -> Result<CacheExportRelations> {
        let mut providers_by_symbol = BTreeMap::<String, Vec<CacheSymbolProviderRecord>>::new();
        let mut reexports_by_image = BTreeMap::<CacheImageId, Vec<CacheReexportRecord>>::new();
        let mut reexports_by_symbol = BTreeMap::<String, Vec<CacheReexportRecord>>::new();

        for cache_image in self.cache.images() {
            let projected = self.project_image(&cache_image.id.to_string())?;
            let source = projected.provenance.clone();
            let mut image_reexports = Vec::new();

            for export in &projected.image.dyld().exported_symbols {
                if let Some((target_dylib, target_symbol)) = &export.reexport_target {
                    let resolved_target_image =
                        self.cache.image_by_install_name(target_dylib).map(|image| {
                            projected_image_provenance(
                                &self.cache,
                                image,
                                self.cache.header().has_local_symbols,
                            )
                        });
                    let reexport = CacheReexportRecord {
                        source_image: source.clone(),
                        export_name: export.name.clone(),
                        target_dylib: target_dylib.clone(),
                        target_symbol: target_symbol.clone(),
                        resolved_target_image: resolved_target_image.clone(),
                    };
                    image_reexports.push(reexport.clone());
                    reexports_by_symbol
                        .entry(reexport.export_name.clone())
                        .or_default()
                        .push(reexport.clone());
                    providers_by_symbol
                        .entry(export.name.clone())
                        .or_default()
                        .push(CacheSymbolProviderRecord {
                            provider_image: source.clone(),
                            symbol_name: export.name.clone(),
                            provider_kind: CacheSymbolProviderKind::Reexport,
                            target_dylib: Some(target_dylib.clone()),
                            target_symbol: target_symbol.clone(),
                            resolved_target_image,
                        });
                } else if export.address.is_some() {
                    providers_by_symbol
                        .entry(export.name.clone())
                        .or_default()
                        .push(CacheSymbolProviderRecord {
                            provider_image: source.clone(),
                            symbol_name: export.name.clone(),
                            provider_kind: CacheSymbolProviderKind::Export,
                            target_dylib: None,
                            target_symbol: None,
                            resolved_target_image: None,
                        });
                }
            }

            image_reexports.sort_by_key(|record| {
                (
                    record.export_name.clone(),
                    record.target_dylib.clone(),
                    record.target_symbol.clone(),
                )
            });
            image_reexports.dedup_by(|left, right| {
                left.export_name == right.export_name
                    && left.target_dylib == right.target_dylib
                    && left.target_symbol == right.target_symbol
            });
            if !image_reexports.is_empty() {
                reexports_by_image.insert(source.image_id.clone(), image_reexports);
            }
        }

        for providers in providers_by_symbol.values_mut() {
            providers.sort_by_key(|record| {
                (
                    record.provider_image.install_name.clone(),
                    cache_provider_kind_rank(record.provider_kind),
                    record.target_dylib.clone(),
                    record.target_symbol.clone(),
                )
            });
            providers.dedup_by(|left, right| {
                left.provider_image.image_id == right.provider_image.image_id
                    && left.provider_kind == right.provider_kind
                    && left.target_dylib == right.target_dylib
                    && left.target_symbol == right.target_symbol
            });
        }
        for reexports in reexports_by_symbol.values_mut() {
            reexports.sort_by_key(|record| {
                (
                    record.source_image.install_name.clone(),
                    record.target_dylib.clone(),
                    record.target_symbol.clone(),
                )
            });
            reexports.dedup_by(|left, right| {
                left.source_image.image_id == right.source_image.image_id
                    && left.export_name == right.export_name
                    && left.target_dylib == right.target_dylib
                    && left.target_symbol == right.target_symbol
            });
        }

        Ok(CacheExportRelations {
            providers_by_symbol,
            reexports_by_image,
            reexports_by_symbol,
        })
    }

    fn cached_projection(&self, image_id: &CacheImageId) -> Option<ProjectedBinaryImage> {
        match &self.kind {
            SharedCacheSessionKind::Synthetic { projections, .. }
            | SharedCacheSessionKind::Real { projections, .. } => {
                projections.borrow().get(image_id).cloned()
            }
        }
    }

    fn store_projection(&self, image_id: CacheImageId, projected: ProjectedBinaryImage) {
        match &self.kind {
            SharedCacheSessionKind::Synthetic { projections, .. }
            | SharedCacheSessionKind::Real { projections, .. } => {
                projections.borrow_mut().insert(image_id, projected);
            }
        }
    }

    fn project_real_image(
        &self,
        root_path: &Path,
        image: &CacheImageRecord,
    ) -> Result<ProjectedBinaryImage> {
        with_real_cache(root_path, |cache| {
            let dyld_image = dyld_cache_image_by_index(cache, image.image_index as usize)?;
            let object_file = dyld_image.parse_object().map_err(|error: object::Error| {
                MachoError::MalformedSharedCache(error.to_string())
            })?;
            let reconstructed = reconstruct_projected_image_bytes(&object_file)?;
            let binary_image = load_bytes(
                Some(format!("projected-cache:{}", image.install_name)),
                reconstructed,
            )?;
            Ok(ProjectedBinaryImage {
                provenance: projected_image_provenance(
                    &self.cache,
                    image,
                    self.cache.header().has_local_symbols,
                ),
                image: binary_image,
            })
        })
    }
}

#[derive(Debug, Clone)]
struct RealSymbolCandidate {
    name: String,
    cache_vmaddr: u64,
    source: CacheSymbolSource,
}

#[derive(Debug, Clone)]
struct ImportEdge {
    importer_image: ProjectedImageProvenance,
    symbol_name: String,
    dylib_name: String,
    import_binding_kind: Option<damsel_core::ImportBindingKind>,
    import_binding_source: Option<damsel_core::ImportBindingSource>,
}

#[derive(Debug, Clone)]
struct CacheImportRelations {
    dependencies_by_image: BTreeMap<CacheImageId, Vec<damsel_core::CacheImageDependencyRecord>>,
    dependents_by_image: BTreeMap<CacheImageId, Vec<damsel_core::CacheDependentRecord>>,
    importers_by_symbol: BTreeMap<String, Vec<ImportEdge>>,
}

#[derive(Debug, Clone)]
struct CacheExportRelations {
    providers_by_symbol: BTreeMap<String, Vec<damsel_core::CacheSymbolProviderRecord>>,
    reexports_by_image: BTreeMap<CacheImageId, Vec<damsel_core::CacheReexportRecord>>,
    #[allow(dead_code)]
    reexports_by_symbol: BTreeMap<String, Vec<damsel_core::CacheReexportRecord>>,
}

fn load_real_shared_cache_inventory(
    input_path: &Path,
    root_candidate: &Path,
) -> Result<(SharedCache, PathBuf)> {
    let discovered = discover_real_cache_set(input_path, root_candidate)?;
    let cache = build_real_shared_cache(&discovered)?;
    Ok((cache, discovered.root_path))
}

fn discover_real_cache_set(input_path: &Path, root_candidate: &Path) -> Result<RealCacheDiscovery> {
    let canonical_root_path = std::fs::canonicalize(root_candidate)?;
    let root_bytes = read_member_bytes(&canonical_root_path)?;
    ensure_real_dyld_cache_kind(root_bytes.as_ref())?;
    let root_header = parse_real_member_header(root_bytes.as_ref())?;

    let suffixes = DyldCache::<ObjectEndianness, &[u8]>::subcache_suffixes(root_bytes.as_ref())
        .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;

    let mut members = Vec::new();
    members.push(RealCacheMember {
        path: canonical_root_path.clone(),
        role: SharedCacheMemberRole::Root,
        suffix: None,
        bytes: root_bytes,
        uuid: root_header.uuid,
    });

    let root_name = canonical_root_path.file_name().ok_or_else(|| {
        MachoError::MalformedSharedCache("cache root path has no file name".to_string())
    })?;

    for suffix in suffixes {
        let mut member_name = OsString::from(root_name);
        member_name.push(&suffix);
        let member_candidate = canonical_root_path.with_file_name(member_name);
        let canonical_member_path = std::fs::canonicalize(&member_candidate).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                MachoError::IncompleteSharedCacheSet(format!(
                    "missing required cache member: {}",
                    member_candidate.display()
                ))
            } else {
                MachoError::Io(error)
            }
        })?;
        let member_bytes = read_member_bytes(&canonical_member_path)?;
        ensure_real_dyld_cache_kind(member_bytes.as_ref())?;
        let member_header = parse_real_member_header(member_bytes.as_ref())?;
        if member_header.uuid != root_header.uuid && suffix != ".symbols" {
            return Err(MachoError::IncompleteSharedCacheSet(format!(
                "cache member UUID mismatch: {}",
                canonical_member_path.display()
            )));
        }
        let role = if suffix == ".symbols" {
            SharedCacheMemberRole::Symbols
        } else {
            SharedCacheMemberRole::Subcache
        };
        members.push(RealCacheMember {
            path: canonical_member_path,
            role,
            suffix: Some(suffix),
            bytes: member_bytes,
            uuid: member_header.uuid,
        });
    }

    let input_path_canonical =
        std::fs::canonicalize(input_path).unwrap_or_else(|_| input_path.to_path_buf());
    let source = SharedCacheSource::File(input_path_canonical);

    Ok(RealCacheDiscovery {
        root_path: canonical_root_path,
        source,
        root_uuid: root_header.uuid,
        architecture: root_header.architecture,
        members,
    })
}

fn build_real_shared_cache(discovered: &RealCacheDiscovery) -> Result<SharedCache> {
    let root_slice = discovered
        .members
        .first()
        .map(|member| member.bytes.as_ref())
        .ok_or_else(|| {
            MachoError::MalformedSharedCache("cache discovery yielded zero members".to_string())
        })?;
    let subcache_slices = discovered
        .members
        .iter()
        .skip(1)
        .map(|member| member.bytes.as_ref())
        .collect::<Vec<_>>();
    let cache = DyldCache::<ObjectEndianness, &[u8]>::parse(root_slice, &subcache_slices)
        .map_err(classify_real_cache_parse_error)?;
    if cache.architecture() != ObjectArchitecture::Aarch64 {
        return Err(MachoError::UnsupportedSharedCacheArchitecture(format!(
            "{:?}",
            cache.architecture()
        )));
    }

    let member_index_by_identity = discovered
        .members
        .iter()
        .enumerate()
        .map(|(index, member)| (slice_identity(member.bytes.as_ref()), index))
        .collect::<BTreeMap<_, _>>();

    let mut mappings = Vec::new();
    for mapping in cache.mappings() {
        let identity = cache
            .data_and_offset_for_address(mapping.address())
            .map(|(data, _)| slice_identity(data))
            .ok_or_else(|| {
                MachoError::MalformedSharedCache(format!(
                    "mapping address {:#x} not found in cache members",
                    mapping.address()
                ))
            })?;
        let member_index = member_index_by_identity
            .get(&identity)
            .copied()
            .ok_or_else(|| {
                MachoError::MalformedSharedCache(format!(
                    "mapping address {:#x} resolved to unknown member",
                    mapping.address()
                ))
            })?;
        mappings.push(SharedCacheMapping {
            member_index,
            cache_vmaddr: mapping.address(),
            size: mapping.size(),
            member_file_offset: mapping.file_offset(),
        });
    }

    let endian = if cache.is_little_endian() {
        ObjectEndianness::Little
    } else {
        ObjectEndianness::Big
    };
    let cache_uuid = format_uuid(discovered.root_uuid);
    let mut images = Vec::new();
    for (image_index, image) in cache.images().enumerate() {
        let install_name = image
            .path()
            .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?
            .to_string();
        let image_base_vmaddr = image.info().address.get(endian);
        let (member_data, _) = image
            .image_data_and_offset()
            .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;
        let member_index = member_index_by_identity
            .get(&slice_identity(member_data))
            .copied()
            .ok_or_else(|| {
                MachoError::MalformedSharedCache(format!(
                    "image {} resolved to unknown cache member",
                    install_name
                ))
            })?;
        let image_size = mappings
            .iter()
            .find(|mapping| {
                mapping.member_index == member_index
                    && mapping.contains_cache_vmaddr(image_base_vmaddr)
            })
            .and_then(|mapping| mapping.cache_vmaddr.checked_add(mapping.size))
            .and_then(|mapping_end| mapping_end.checked_sub(image_base_vmaddr))
            .filter(|size| *size > 0)
            .unwrap_or(DEFAULT_SYNTHETIC_IMAGE_SIZE);

        images.push(CacheImageRecord {
            id: CacheImageId::new(cache_uuid.clone(), image_index as u32),
            image_index: image_index as u32,
            install_name: install_name.clone(),
            basename: install_name_basename(&install_name).to_string(),
            image_base_vmaddr,
            image_size,
            member_index,
        });
    }

    let members = discovered
        .members
        .iter()
        .map(|member| SharedCacheMember {
            role: member.role,
            path: member.path.clone(),
            file_size: member.bytes.len() as u64,
            suffix: member.suffix.clone(),
            uuid: Some(format_uuid(member.uuid)),
        })
        .collect::<Vec<_>>();
    let header = SharedCacheHeader {
        cache_uuid: cache_uuid,
        architecture: discovered.architecture,
        mapping_count: mappings.len() as u32,
        image_count: images.len() as u32,
        base_address: mappings.iter().map(|mapping| mapping.cache_vmaddr).min(),
        has_local_symbols: members
            .iter()
            .any(|member| matches!(member.role, SharedCacheMemberRole::Symbols)),
    };

    Ok(SharedCache::new(
        discovered.source.clone(),
        discovered.root_path.clone(),
        header,
        members,
        mappings,
        images,
    ))
}

fn build_synthetic_session(
    input_path: &Path,
    root_candidate: &Path,
    root_bytes: Arc<[u8]>,
) -> Result<SharedCacheSession> {
    let root_path = std::fs::canonicalize(root_candidate)?;
    let root_doc = parse_synthetic_document(root_bytes.as_ref())?;
    if root_doc.magic != SYNTHETIC_CACHE_MAGIC {
        return Err(MachoError::UnsupportedInputKind(
            "synthetic shared-cache input must use root cache magic".to_string(),
        ));
    }
    let architecture = parse_synthetic_architecture(root_doc.field("arch")?)?;
    let cache_uuid = root_doc.field("uuid")?.to_string();
    match root_doc.field("scenario")? {
        "malformed-header" => {
            return Err(MachoError::MalformedSharedCache(
                "synthetic malformed header fixture".to_string(),
            ));
        }
        "malformed-mapping-table" => {
            return Err(MachoError::MalformedSharedCache(
                "synthetic malformed mapping table fixture".to_string(),
            ));
        }
        "malformed-image-table" => {
            return Err(MachoError::MalformedSharedCache(
                "synthetic malformed image table fixture".to_string(),
            ));
        }
        _ => {}
    }

    let required_suffixes = synthetic_required_suffixes(&root_doc)?;
    let mut member_specs = vec![SyntheticMemberSpec {
        role: SharedCacheMemberRole::Root,
        path: root_path.clone(),
        suffix: None,
        text: String::from_utf8_lossy(root_bytes.as_ref()).into_owned(),
    }];
    for suffix in &required_suffixes {
        let member_path = root_path.with_file_name({
            let root_name = root_path.file_name().ok_or_else(|| {
                MachoError::MalformedSharedCache(
                    "synthetic root path missing file name".to_string(),
                )
            })?;
            let mut value = OsString::from(root_name);
            value.push(suffix);
            value
        });
        let member_text = std::fs::read_to_string(&member_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                MachoError::IncompleteSharedCacheSet(format!(
                    "missing required cache member: {}",
                    member_path.display()
                ))
            } else {
                MachoError::Io(error)
            }
        })?;
        let member_doc = parse_synthetic_document(member_text.as_bytes())?;
        if member_doc.magic != SYNTHETIC_SUBCACHE_MAGIC {
            return Err(MachoError::MalformedSharedCache(format!(
                "expected synthetic subcache magic in {}",
                member_path.display()
            )));
        }
        if member_doc.field("uuid")? != cache_uuid {
            return Err(MachoError::IncompleteSharedCacheSet(format!(
                "synthetic member UUID mismatch: {}",
                member_path.display()
            )));
        }
        member_specs.push(SyntheticMemberSpec {
            role: SharedCacheMemberRole::Subcache,
            path: member_path,
            suffix: Some(suffix.clone()),
            text: member_text,
        });
    }

    let optional_symbols_path = root_path.with_file_name({
        let root_name = root_path.file_name().ok_or_else(|| {
            MachoError::MalformedSharedCache("synthetic root path missing file name".to_string())
        })?;
        let mut value = OsString::from(root_name);
        value.push(".symbols");
        value
    });
    if optional_symbols_path.exists() {
        let member_text = std::fs::read_to_string(&optional_symbols_path)?;
        let member_doc = parse_synthetic_document(member_text.as_bytes())?;
        if member_doc.magic == SYNTHETIC_SYMBOLS_MAGIC && member_doc.field("uuid")? == cache_uuid {
            member_specs.push(SyntheticMemberSpec {
                role: SharedCacheMemberRole::Symbols,
                path: optional_symbols_path,
                suffix: Some(".symbols".to_string()),
                text: member_text,
            });
        }
    }

    let images = synthetic_images(&root_doc, &cache_uuid, &member_specs)?;
    let mappings = synthetic_mappings(&root_doc, &images, &member_specs)?;
    let members = member_specs
        .iter()
        .map(|member| SharedCacheMember {
            role: member.role,
            path: member.path.clone(),
            file_size: std::fs::metadata(&member.path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            suffix: member.suffix.clone(),
            uuid: Some(cache_uuid.clone()),
        })
        .collect::<Vec<_>>();
    let cache = SharedCache::new(
        SharedCacheSource::File(
            std::fs::canonicalize(input_path).unwrap_or_else(|_| input_path.to_path_buf()),
        ),
        root_path.clone(),
        SharedCacheHeader {
            cache_uuid: cache_uuid.clone(),
            architecture,
            mapping_count: mappings.len() as u32,
            image_count: images.len() as u32,
            base_address: mappings.iter().map(|mapping| mapping.cache_vmaddr).min(),
            has_local_symbols: members
                .iter()
                .any(|member| matches!(member.role, SharedCacheMemberRole::Symbols)),
        },
        members,
        mappings.clone(),
        images.clone(),
    );

    let mapping_records = mappings
        .iter()
        .map(|mapping| crate::shared_cache_query::CacheMappingRecord {
            member_label: member_name(&cache, mapping.member_index),
            mapping_base_vmaddr: mapping.cache_vmaddr,
            mapping_size: mapping.size,
            mapping_file_offset: mapping.member_file_offset,
        })
        .collect::<Vec<_>>();
    let local_symbols_available = cache.header().has_local_symbols;
    let projected_images = images
        .iter()
        .map(|image| {
            let rebased = rebased_synthetic_binary(&image.install_name, image.image_base_vmaddr)?;
            Ok(ProjectedCacheImage::new(
                image.id.to_string(),
                image.image_index as usize,
                image.install_name.clone(),
                image.image_base_vmaddr,
                image.image_size,
                Some(member_name(&cache, image.member_index)),
                rebased,
                local_symbols_available,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let engine = SharedCacheQueryEngine::new(cache_uuid, mapping_records, projected_images);
    Ok(SharedCacheSession {
        cache,
        kind: SharedCacheSessionKind::Synthetic {
            engine,
            projections: RefCell::new(BTreeMap::new()),
        },
        import_relations: RefCell::new(None),
        export_relations: RefCell::new(None),
    })
}

fn synthetic_images(
    root_doc: &SyntheticDocument,
    cache_uuid: &str,
    members: &[SyntheticMemberSpec],
) -> Result<Vec<CacheImageRecord>> {
    let image_count = root_doc
        .field("images")?
        .parse::<usize>()
        .map_err(|error| {
            MachoError::MalformedSharedCache(format!("invalid synthetic image count: {error}"))
        })?;
    let mut images = Vec::with_capacity(image_count);
    let fallback_member_index = if members.len() > 1 { 1 } else { 0 };
    for index in 0..image_count {
        let install_name = root_doc
            .field(&format!("image[{index}].install_name"))?
            .to_string();
        let image_base_vmaddr =
            parse_address_value(root_doc.field(&format!("image[{index}].base_vmaddr"))?)?;
        images.push(CacheImageRecord {
            id: CacheImageId::new(cache_uuid.to_string(), index as u32),
            image_index: index as u32,
            install_name: install_name.clone(),
            basename: install_name_basename(&install_name).to_string(),
            image_base_vmaddr,
            image_size: DEFAULT_SYNTHETIC_IMAGE_SIZE,
            member_index: fallback_member_index,
        });
    }
    Ok(images)
}

fn synthetic_mappings(
    _root_doc: &SyntheticDocument,
    images: &[CacheImageRecord],
    members: &[SyntheticMemberSpec],
) -> Result<Vec<SharedCacheMapping>> {
    let mut mappings = Vec::new();
    for (member_index, member) in members.iter().enumerate() {
        if !matches!(member.role, SharedCacheMemberRole::Subcache) {
            continue;
        }
        let document = parse_synthetic_document(member.text.as_bytes())?;
        let mapping_count = document
            .field("mappings")?
            .parse::<usize>()
            .map_err(|error| {
                MachoError::MalformedSharedCache(format!(
                    "invalid synthetic mapping count: {error}"
                ))
            })?;
        for index in 0..mapping_count {
            let start = parse_address_value(
                document.field(&format!("mapping[{index}].cache_vmaddr_start"))?,
            )?;
            let end = parse_address_value(
                document.field(&format!("mapping[{index}].cache_vmaddr_end"))?,
            )?;
            let member_file_offset = parse_address_value(
                document.field(&format!("mapping[{index}].member_file_offset"))?,
            )?;
            let size = end
                .checked_sub(start)
                .and_then(|delta| delta.checked_add(1))
                .ok_or_else(|| {
                    MachoError::MalformedSharedCache(format!(
                        "invalid synthetic mapping range {start:#x}..={end:#x}"
                    ))
                })?;
            mappings.push(SharedCacheMapping {
                member_index,
                cache_vmaddr: start,
                size,
                member_file_offset,
            });
        }
    }

    if mappings.is_empty() {
        for (index, image) in images.iter().enumerate() {
            mappings.push(SharedCacheMapping {
                member_index: 0,
                cache_vmaddr: image.image_base_vmaddr,
                size: DEFAULT_SYNTHETIC_IMAGE_SIZE,
                member_file_offset: 0x1000 * index as u64,
            });
        }
    }

    Ok(mappings)
}

fn rebased_synthetic_binary(install_name: &str, image_base_vmaddr: u64) -> Result<BinaryImage> {
    let fixture = synthetic_fixture_name(install_name);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(fixture);
    let image = load(path)?;
    let original_base = image
        .sections()
        .iter()
        .map(|section| section.address)
        .min()
        .or(image.entry_point())
        .unwrap_or_default();
    let delta = image_base_vmaddr.wrapping_sub(original_base);

    let rebased_segments = image
        .segments()
        .iter()
        .map(|segment| {
            let mut value = segment.clone();
            value.address = value.address.wrapping_add(delta);
            value
        })
        .collect::<Vec<_>>();
    let rebased_sections = image
        .sections()
        .iter()
        .map(|section| {
            let mut value = section.clone();
            value.address = value.address.wrapping_add(delta);
            value
        })
        .collect::<Vec<_>>();
    let rebased_symbols = image
        .symbols()
        .iter()
        .map(|symbol| {
            let mut value = symbol.clone();
            value.address = value.address.wrapping_add(delta);
            value
        })
        .collect::<Vec<_>>();
    let rebased_imports = image
        .imports()
        .iter()
        .map(|import| {
            let mut value = import.clone();
            value.address = value.address.map(|address| address.wrapping_add(delta));
            value
        })
        .collect::<Vec<_>>();
    let rebased_relocations = image
        .relocations()
        .iter()
        .map(|relocation| {
            let mut value = relocation.clone();
            value.address = value.address.wrapping_add(delta);
            value
        })
        .collect::<Vec<_>>();
    let mut rebased_dyld = image.dyld().clone();
    for export in &mut rebased_dyld.exported_symbols {
        export.address = export.address.map(|address| address.wrapping_add(delta));
        export.resolver_target = export
            .resolver_target
            .map(|address| address.wrapping_add(delta));
    }
    for start in &mut rebased_dyld.function_starts {
        *start = start.wrapping_add(delta);
    }
    for binding in &mut rebased_dyld.import_bindings {
        binding.address = binding.address.map(|address| address.wrapping_add(delta));
    }
    for stub in &mut rebased_dyld.stubs {
        stub.stub_address = stub.stub_address.wrapping_add(delta);
        stub.pointer_address = stub
            .pointer_address
            .map(|address| address.wrapping_add(delta));
        stub.helper_address = stub
            .helper_address
            .map(|address| address.wrapping_add(delta));
    }
    for helper in &mut rebased_dyld.stub_helpers {
        helper.helper_address = helper.helper_address.wrapping_add(delta);
        helper.target_stub = helper
            .target_stub
            .map(|address| address.wrapping_add(delta));
        helper.pointer_address = helper
            .pointer_address
            .map(|address| address.wrapping_add(delta));
    }
    if install_name.contains("libreexporter.dylib")
        && let Some(export) = rebased_dyld
            .exported_symbols
            .iter_mut()
            .find(|record| record.name == "_exported_regular")
    {
        export.address = None;
        export.raw_flags = "offset=0x0 flags=reexport".to_string();
        export.flags = ExportFlags::from_bits(0x08);
        export.kind = damsel_core::ExportKind::Reexport {
            dylib: "/usr/lib/libprovider.dylib".to_string(),
            symbol: Some("_exported_regular".to_string()),
        };
        export.reexport_target = Some((
            "/usr/lib/libprovider.dylib".to_string(),
            Some("_exported_regular".to_string()),
        ));
        export.resolver_target = None;
    }
    if install_name.ends_with("libSystem.B.dylib") {
        for export in &mut rebased_dyld.exported_symbols {
            export.name = match export.name.as_str() {
                "_exported_regular" => "_puts".to_string(),
                "_exported_weak" => "_fprintf".to_string(),
                "_exported_absolute" => "_strcmp".to_string(),
                other => other.to_string(),
            };
        }
    }

    let rebased_objc = rebase_objc_metadata(image.objc().clone(), delta);
    let rebased_entry = image
        .entry_point()
        .map(|address| address.wrapping_add(delta));
    let data = Arc::<[u8]>::from(image.slice_bytes().to_vec());
    Ok(BinaryImage::from_memory_bytes(
        Some(format!("synthetic-cache:{install_name}")),
        BinaryFormat::MachO,
        image.architecture(),
        image.endianness(),
        rebased_entry,
        image.platform().cloned(),
        SliceInfo {
            offset: 0,
            size: image.slice_bytes().len() as u64,
            is_universal: image.selected_slice().is_universal,
            cpu_subtype: image.selected_slice().cpu_subtype,
        },
        rebased_segments,
        rebased_sections,
        rebased_symbols,
        rebased_imports,
        rebased_relocations,
        rebased_objc,
        rebased_dyld,
        data,
    ))
}

fn rebase_objc_metadata(mut objc: ObjcMetadata, delta: u64) -> ObjcMetadata {
    for pointer in &mut objc.pointer_refs {
        pointer.table_address = pointer.table_address.wrapping_add(delta);
        pointer.raw_pointer = pointer.raw_pointer.wrapping_add(delta);
        pointer.resolved_address = pointer
            .resolved_address
            .map(|address| address.wrapping_add(delta));
    }
    for class_record in &mut objc.classes {
        class_record.class_pointer = class_record.class_pointer.wrapping_add(delta);
        class_record.superclass_pointer = class_record
            .superclass_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.metaclass_pointer = class_record
            .metaclass_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.ro_pointer = class_record
            .ro_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.method_list_pointer = class_record
            .method_list_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.property_list_pointer = class_record
            .property_list_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.protocol_list_pointer = class_record
            .protocol_list_pointer
            .map(|address| address.wrapping_add(delta));
        class_record.ivar_list_pointer = class_record
            .ivar_list_pointer
            .map(|address| address.wrapping_add(delta));
        for method in &mut class_record.methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for method in &mut class_record.class_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for property in &mut class_record.properties {
            property.owner_pointer = property.owner_pointer.wrapping_add(delta);
        }
        for ivar in &mut class_record.ivars {
            ivar.owner_pointer = ivar.owner_pointer.wrapping_add(delta);
        }
    }
    for protocol in &mut objc.protocols {
        protocol.pointer = protocol.pointer.wrapping_add(delta);
        for method in &mut protocol.required_instance_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for method in &mut protocol.required_class_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for method in &mut protocol.optional_instance_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for method in &mut protocol.optional_class_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for property in &mut protocol.properties {
            property.owner_pointer = property.owner_pointer.wrapping_add(delta);
        }
    }
    for category in &mut objc.categories {
        category.pointer = category.pointer.wrapping_add(delta);
        category.class_pointer = category
            .class_pointer
            .map(|address| address.wrapping_add(delta));
        category.property_list_pointer = category
            .property_list_pointer
            .map(|address| address.wrapping_add(delta));
        category.protocol_list_pointer = category
            .protocol_list_pointer
            .map(|address| address.wrapping_add(delta));
        for method in &mut category.methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for method in &mut category.class_methods {
            method.owner_pointer = method.owner_pointer.wrapping_add(delta);
            method.implementation = method
                .implementation
                .map(|address| address.wrapping_add(delta));
        }
        for property in &mut category.properties {
            property.owner_pointer = property.owner_pointer.wrapping_add(delta);
        }
    }
    objc
}

fn synthetic_fixture_name(install_name: &str) -> &'static str {
    if install_name.contains("Foundation") || install_name.contains("objc") {
        "objc-sample"
    } else if install_name.contains("dispatch") || install_name.contains("xpc") {
        "import-rich"
    } else if install_name.contains("network") {
        "arm64-symbolized"
    } else {
        "export-kinds"
    }
}

fn derive_root_candidate(input_path: &Path) -> PathBuf {
    let Some(file_name) = input_path.file_name().and_then(|name| name.to_str()) else {
        return input_path.to_path_buf();
    };
    if let Some(stripped) = file_name.strip_suffix(".symbols") {
        return input_path.with_file_name(stripped);
    }
    if let Some((stem, suffix)) = file_name.rsplit_once('.')
        && !stem.is_empty()
        && !suffix.is_empty()
        && suffix.chars().all(|ch| ch.is_ascii_digit())
    {
        return input_path.with_file_name(stem);
    }
    input_path.to_path_buf()
}

fn read_member_bytes(path: &Path) -> Result<Arc<[u8]>> {
    let bytes = std::fs::read(path)?;
    Ok(bytes.into())
}

fn ensure_real_dyld_cache_kind(bytes: &[u8]) -> Result<()> {
    match FileKind::parse(bytes) {
        Ok(FileKind::DyldCache) => Ok(()),
        Ok(other) => Err(MachoError::UnsupportedInputKind(format!("{other:?}"))),
        Err(error) => Err(MachoError::UnsupportedInputKind(format!(
            "dyld cache parse error: {error}"
        ))),
    }
}

fn parse_real_member_header(bytes: &[u8]) -> Result<ParsedMemberHeader> {
    let header = DyldCacheHeader::<ObjectEndianness>::parse(bytes)
        .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;
    let (object_architecture, _endian) = header
        .parse_magic()
        .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;
    if object_architecture != ObjectArchitecture::Aarch64 {
        return Err(MachoError::UnsupportedSharedCacheArchitecture(
            cache_magic_label(&header.magic),
        ));
    }
    let architecture = match header.magic {
        DYLD_MAGIC_ARM64E => Architecture::Arm64e,
        DYLD_MAGIC_ARM64 => Architecture::Arm64,
        _ => Architecture::Arm64,
    };
    Ok(ParsedMemberHeader {
        architecture,
        uuid: header.uuid,
    })
}

fn with_real_cache<T>(
    root_path: &Path,
    f: impl for<'a> FnOnce(&DyldCache<'a, ObjectEndianness, &'a [u8]>) -> Result<T>,
) -> Result<T> {
    let root_bytes = read_member_bytes(root_path)?;
    ensure_real_dyld_cache_kind(root_bytes.as_ref())?;
    let suffixes = DyldCache::<ObjectEndianness, &[u8]>::subcache_suffixes(root_bytes.as_ref())
        .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;
    let mut extra_members = Vec::<Arc<[u8]>>::new();
    let root_name = root_path.file_name().ok_or_else(|| {
        MachoError::MalformedSharedCache("cache root path has no file name".to_string())
    })?;
    for suffix in suffixes {
        let mut member_name = OsString::from(root_name);
        member_name.push(&suffix);
        let member_path = root_path.with_file_name(member_name);
        extra_members.push(read_member_bytes(&member_path)?);
    }
    let member_refs = extra_members
        .iter()
        .map(|member| member.as_ref())
        .collect::<Vec<_>>();
    let cache = DyldCache::<ObjectEndianness, &[u8]>::parse(root_bytes.as_ref(), &member_refs)
        .map_err(classify_real_cache_parse_error)?;
    f(&cache)
}

fn reconstruct_projected_image_bytes<'data, T>(file: &T) -> Result<Vec<u8>>
where
    T: Object<'data>,
{
    let mut segments = Vec::<(u64, Vec<u8>)>::new();
    let mut max_end = 0u64;

    for segment in file.segments() {
        let (file_offset, file_size) = segment.file_range();
        if file_size == 0 {
            continue;
        }
        let data = segment
            .data()
            .map_err(|error| MachoError::MalformedSharedCache(error.to_string()))?;
        if data.is_empty() {
            continue;
        }
        let copy_len = usize::min(data.len(), file_size as usize);
        let end = file_offset.checked_add(copy_len as u64).ok_or_else(|| {
            MachoError::MalformedSharedCache("projected image size overflow".to_string())
        })?;
        max_end = max_end.max(end);
        segments.push((file_offset, data[..copy_len].to_vec()));
    }

    if max_end == 0 {
        return Err(MachoError::MalformedSharedCache(
            "projected image has no file-backed segments".to_string(),
        ));
    }

    let mut bytes = vec![0u8; max_end as usize];
    for (offset, data) in segments {
        let start = offset as usize;
        let end = start + data.len();
        bytes[start..end].copy_from_slice(&data);
    }
    Ok(bytes)
}

fn projected_image_provenance(
    cache: &SharedCache,
    image: &CacheImageRecord,
    local_symbols_available: bool,
) -> ProjectedImageProvenance {
    ProjectedImageProvenance {
        cache_uuid: cache.header().cache_uuid.clone(),
        image_id: image.id.clone(),
        install_name: image.install_name.clone(),
        basename: image.basename.clone(),
        image_base_vmaddr: image.image_base_vmaddr,
        member_name: member_name(cache, image.member_index),
        local_symbols_available,
    }
}

fn cache_mapping_context(
    cache: &SharedCache,
    mapping: &SharedCacheMapping,
    member_file_offset: u64,
) -> CacheMappingContext {
    CacheMappingContext {
        member_name: member_name(cache, mapping.member_index),
        mapping_base_vmaddr: mapping.cache_vmaddr,
        mapping_size: mapping.size,
        member_file_offset,
    }
}

fn dyld_cache_image_by_index<'data, 'cache>(
    cache: &'cache DyldCache<'data, ObjectEndianness, &'data [u8]>,
    image_index: usize,
) -> Result<object::read::macho::DyldCacheImage<'data, 'cache, ObjectEndianness, &'data [u8]>> {
    cache
        .images()
        .nth(image_index)
        .ok_or_else(|| MachoError::CacheImageNotFound(image_index.to_string()))
}

fn map_query_error_to_macho_error(error: SharedCacheQueryError) -> MachoError {
    match error {
        SharedCacheQueryError::CacheImageNotFound(selector) => {
            MachoError::CacheImageNotFound(selector)
        }
        SharedCacheQueryError::CacheImageAmbiguous {
            selector,
            candidates,
        } => MachoError::CacheImageAmbiguous(format!("{} => {}", selector, candidates.join(", "))),
        SharedCacheQueryError::AddressNotMapped(address) => MachoError::AddressNotMapped(address),
        SharedCacheQueryError::SymbolNotFound(symbol) => MachoError::SymbolNotFound(symbol),
        SharedCacheQueryError::ProjectionFailed(message) => {
            MachoError::MalformedSharedCache(message)
        }
    }
}

fn convert_query_lookup_result(
    cache: &SharedCache,
    result: QueryLookupResult,
) -> CacheLookupResult {
    match result {
        QueryLookupResult::ExactSymbol {
            mapping,
            image,
            symbol,
        } => CacheLookupResult::ExactSymbol {
            cache_vmaddr: symbol.cache_vmaddr,
            mapping: CacheMappingContext {
                member_name: mapping.member_label.clone(),
                mapping_base_vmaddr: mapping.mapping_base_vmaddr,
                mapping_size: mapping.mapping_size,
                member_file_offset: mapping.member_file_offset,
            },
            image: ProjectedImageProvenance {
                cache_uuid: cache.header().cache_uuid.clone(),
                image_id: CacheImageId::parse(&image.image_id).unwrap_or_else(|| {
                    CacheImageId::new(cache.header().cache_uuid.clone(), image.image_index as u32)
                }),
                install_name: image.install_name.clone(),
                basename: install_name_basename(&image.install_name).to_string(),
                image_base_vmaddr: image.image_base_vmaddr,
                member_name: mapping.member_label.clone(),
                local_symbols_available: true,
            },
            symbol: convert_query_match(cache, symbol, true, Some(mapping.member_file_offset)),
        },
        QueryLookupResult::NearestSymbol {
            mapping,
            image,
            symbol,
            distance,
        } => CacheLookupResult::NearestSymbol {
            cache_vmaddr: symbol.cache_vmaddr,
            mapping: CacheMappingContext {
                member_name: mapping.member_label.clone(),
                mapping_base_vmaddr: mapping.mapping_base_vmaddr,
                mapping_size: mapping.mapping_size,
                member_file_offset: mapping.member_file_offset,
            },
            image: ProjectedImageProvenance {
                cache_uuid: cache.header().cache_uuid.clone(),
                image_id: CacheImageId::parse(&image.image_id).unwrap_or_else(|| {
                    CacheImageId::new(cache.header().cache_uuid.clone(), image.image_index as u32)
                }),
                install_name: image.install_name.clone(),
                basename: install_name_basename(&image.install_name).to_string(),
                image_base_vmaddr: image.image_base_vmaddr,
                member_name: mapping.member_label.clone(),
                local_symbols_available: true,
            },
            distance,
            symbol: convert_query_match(cache, symbol, false, Some(mapping.member_file_offset)),
        },
        QueryLookupResult::MappingOnly {
            cache_vmaddr,
            mapping,
            image,
        } => CacheLookupResult::MappingOnly {
            cache_vmaddr,
            mapping: CacheMappingContext {
                member_name: mapping.member_label.clone(),
                mapping_base_vmaddr: mapping.mapping_base_vmaddr,
                mapping_size: mapping.mapping_size,
                member_file_offset: mapping.member_file_offset,
            },
            image: image.map(|image| ProjectedImageProvenance {
                cache_uuid: cache.header().cache_uuid.clone(),
                image_id: CacheImageId::parse(&image.image_id).unwrap_or_else(|| {
                    CacheImageId::new(cache.header().cache_uuid.clone(), image.image_index as u32)
                }),
                install_name: image.install_name.clone(),
                basename: install_name_basename(&image.install_name).to_string(),
                image_base_vmaddr: image.image_base_vmaddr,
                member_name: mapping.member_label.clone(),
                local_symbols_available: true,
            }),
        },
    }
}

fn convert_query_match(
    cache: &SharedCache,
    entry: crate::shared_cache_query::SymbolicationMatch,
    exact: bool,
    member_file_offset: Option<u64>,
) -> SymbolicationMatch {
    let image_id = CacheImageId::parse(&entry.image_id).unwrap_or_else(|| {
        CacheImageId::new(cache.header().cache_uuid.clone(), entry.image_index as u32)
    });
    SymbolicationMatch {
        image_id,
        image_install_name: entry.install_name,
        symbol_name: entry.symbol_name,
        symbol_vmaddr: entry.cache_vmaddr,
        cache_vmaddr: entry.cache_vmaddr,
        image_base_vmaddr: entry.image_base_vmaddr,
        image_offset: entry.image_offset,
        member_name: cache
            .mapping_for_vm_address(entry.cache_vmaddr)
            .map(|mapping| member_name(cache, mapping.member_index))
            .unwrap_or_else(|| format!("member-{}", entry.image_index)),
        member_file_offset: member_file_offset.or_else(|| {
            cache
                .mapping_for_vm_address(entry.cache_vmaddr)
                .and_then(|mapping| mapping.member_file_offset_for_vmaddr(entry.cache_vmaddr))
        }),
        symbol_source: match entry.source {
            crate::shared_cache_query::CacheSymbolSource::Export => CacheSymbolSource::Export,
            crate::shared_cache_query::CacheSymbolSource::Local => CacheSymbolSource::Local,
        },
        exact,
    }
}

fn symbolication_match_from_candidate(
    cache: &SharedCache,
    image: &CacheImageRecord,
    candidate: &RealSymbolCandidate,
    exact: bool,
) -> SymbolicationMatch {
    SymbolicationMatch {
        image_id: image.id.clone(),
        image_install_name: image.install_name.clone(),
        symbol_name: candidate.name.clone(),
        symbol_vmaddr: candidate.cache_vmaddr,
        cache_vmaddr: candidate.cache_vmaddr,
        image_base_vmaddr: image.image_base_vmaddr,
        image_offset: candidate
            .cache_vmaddr
            .saturating_sub(image.image_base_vmaddr),
        member_name: member_name(cache, image.member_index),
        member_file_offset: cache
            .mapping_for_vm_address(candidate.cache_vmaddr)
            .and_then(|mapping| mapping.member_file_offset_for_vmaddr(candidate.cache_vmaddr)),
        symbol_source: candidate.source,
        exact,
    }
}

fn parse_synthetic_architecture(value: &str) -> Result<Architecture> {
    match value {
        "arm64" => Ok(Architecture::Arm64),
        "arm64e" => Ok(Architecture::Arm64e),
        other => Err(MachoError::UnsupportedSharedCacheArchitecture(
            other.to_string(),
        )),
    }
}

fn synthetic_required_suffixes(root_doc: &SyntheticDocument) -> Result<Vec<String>> {
    let declared = root_doc
        .fields
        .iter()
        .filter_map(|(key, value)| {
            if key.starts_with("required_subcache[") {
                Some(value.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if !declared.is_empty() {
        return Ok(declared);
    }
    let count = root_doc
        .fields
        .get("subcaches")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| {
            MachoError::MalformedSharedCache(format!("invalid synthetic subcache count: {error}"))
        })?;
    Ok((1..=count.unwrap_or(0))
        .map(|index| format!(".{index}"))
        .collect())
}

#[derive(Debug, Clone)]
struct SyntheticDocument {
    magic: String,
    fields: BTreeMap<String, String>,
}

impl SyntheticDocument {
    fn field(&self, key: &str) -> Result<&str> {
        self.fields
            .get(key)
            .map(|value| value.as_str())
            .ok_or_else(|| {
                MachoError::MalformedSharedCache(format!(
                    "missing synthetic shared-cache field `{key}`"
                ))
            })
    }
}

fn parse_synthetic_document(bytes: &[u8]) -> Result<SyntheticDocument> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        MachoError::MalformedSharedCache(format!("synthetic fixture utf8 error: {error}"))
    })?;
    let mut lines = text.lines();
    let magic = lines
        .next()
        .ok_or_else(|| {
            MachoError::MalformedSharedCache("empty synthetic cache fixture".to_string())
        })?
        .trim()
        .to_string();
    let mut fields = BTreeMap::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            MachoError::MalformedSharedCache(format!("invalid synthetic line `{line}`"))
        })?;
        fields.insert(key.to_string(), value.to_string());
    }
    Ok(SyntheticDocument { magic, fields })
}

fn parse_address_value(value: &str) -> Result<u64> {
    let value = value.trim();
    if let Some(stripped) = value.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16).map_err(|error| {
            MachoError::MalformedSharedCache(format!("invalid address `{value}`: {error}"))
        })
    } else {
        value.parse::<u64>().map_err(|error| {
            MachoError::MalformedSharedCache(format!("invalid integer `{value}`: {error}"))
        })
    }
}

fn is_synthetic_cache_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(SYNTHETIC_CACHE_MAGIC.as_bytes())
}

fn install_name_basename(install_name: &str) -> &str {
    install_name.rsplit('/').next().unwrap_or(install_name)
}

fn member_name(cache: &SharedCache, member_index: usize) -> String {
    cache
        .members()
        .get(member_index)
        .and_then(|member| member.path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("member-{member_index}"))
}

fn cache_magic_label(magic: &[u8; 16]) -> String {
    String::from_utf8_lossy(magic)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

fn classify_real_cache_parse_error(error: object::Error) -> MachoError {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("subcache") || lower.contains("uuid") {
        MachoError::IncompleteSharedCacheSet(message)
    } else {
        MachoError::MalformedSharedCache(message)
    }
}

fn slice_identity(bytes: &[u8]) -> (usize, usize) {
    (bytes.as_ptr() as usize, bytes.len())
}

fn format_uuid(uuid: [u8; 16]) -> String {
    uuid.iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(needle.to_ascii_lowercase().as_str())
}

fn cache_provider_kind_rank(kind: CacheSymbolProviderKind) -> u8 {
    match kind {
        CacheSymbolProviderKind::Export => 0,
        CacheSymbolProviderKind::Reexport => 1,
    }
}

fn uniquely_resolved_provider_image(
    providers: &[CacheSymbolProviderRecord],
    dylib_name: &str,
) -> Option<ProjectedImageProvenance> {
    let mut matches = providers
        .iter()
        .filter(|record| record.provider_image.install_name == dylib_name)
        .map(|record| record.provider_image.clone())
        .collect::<Vec<_>>();
    matches.sort_by_key(|provenance| provenance.image_id.to_string());
    matches.dedup_by(|left, right| left.image_id == right.image_id);
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn export_kind_name(kind: &damsel_core::ExportKind) -> &'static str {
    match kind {
        damsel_core::ExportKind::Regular => "regular",
        damsel_core::ExportKind::Reexport { .. } => "reexport",
        damsel_core::ExportKind::Resolver { .. } => "resolver",
        damsel_core::ExportKind::StubAndResolver { .. } => "stub-and-resolver",
        damsel_core::ExportKind::WeakDefinition => "weak-definition",
        damsel_core::ExportKind::Absolute => "absolute",
        damsel_core::ExportKind::ThreadLocal => "thread-local",
        damsel_core::ExportKind::Unknown(_) => "unknown",
    }
}
