use std::collections::BTreeMap;

use damsel_core::{BinaryImage, ExportRecord, Symbol};
use object::endian::Endian;
use object::read::{ReadRef, macho::DyldCacheImage};
use object::{Object, ObjectSegment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SharedCacheQueryError {
    CacheImageNotFound(String),
    CacheImageAmbiguous {
        selector: String,
        candidates: Vec<String>,
    },
    AddressNotMapped(u64),
    SymbolNotFound(String),
    ProjectionFailed(String),
}

pub(crate) type Result<T> = std::result::Result<T, SharedCacheQueryError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheSymbolSource {
    Export,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheMappingRecord {
    pub member_label: String,
    pub mapping_base_vmaddr: u64,
    pub mapping_size: u64,
    pub mapping_file_offset: u64,
}

impl CacheMappingRecord {
    fn contains_address(&self, vmaddr: u64) -> bool {
        self.mapping_base_vmaddr
            .checked_add(self.mapping_size)
            .map(|end| (self.mapping_base_vmaddr..end).contains(&vmaddr))
            .unwrap_or(false)
    }

    fn member_file_offset_for(&self, vmaddr: u64) -> Option<u64> {
        if !self.contains_address(vmaddr) {
            return None;
        }
        let mapping_delta = vmaddr.checked_sub(self.mapping_base_vmaddr)?;
        self.mapping_file_offset.checked_add(mapping_delta)
    }
}

#[derive(Debug)]
pub(crate) struct ProjectedCacheImage {
    pub image_id: String,
    pub image_index: usize,
    pub install_name: String,
    pub image_base_vmaddr: u64,
    pub image_size: u64,
    #[allow(dead_code)]
    pub member_label: Option<String>,
    pub binary_image: BinaryImage,
    pub local_symbols_available: bool,
}

impl ProjectedCacheImage {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        image_id: String,
        image_index: usize,
        install_name: String,
        image_base_vmaddr: u64,
        image_size: u64,
        member_label: Option<String>,
        binary_image: BinaryImage,
        local_symbols_available: bool,
    ) -> Self {
        let inferred_size = if image_size > 0 {
            image_size
        } else {
            infer_image_size(&binary_image, image_base_vmaddr).unwrap_or(1)
        };
        Self {
            image_id,
            image_index,
            install_name,
            image_base_vmaddr,
            image_size: inferred_size,
            member_label,
            binary_image,
            local_symbols_available,
        }
    }

    fn contains_address(&self, vmaddr: u64) -> bool {
        self.image_base_vmaddr
            .checked_add(self.image_size)
            .map(|end| (self.image_base_vmaddr..end).contains(&vmaddr))
            .unwrap_or(false)
    }

    fn image_offset_for(&self, vmaddr: u64) -> Option<u64> {
        vmaddr.checked_sub(self.image_base_vmaddr)
    }

    fn basename(&self) -> &str {
        install_name_basename(&self.install_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheMappingContext {
    pub member_label: String,
    pub mapping_base_vmaddr: u64,
    pub mapping_size: u64,
    pub member_file_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheImageContext {
    pub image_id: String,
    pub image_index: usize,
    pub install_name: String,
    pub image_base_vmaddr: u64,
    pub image_size: u64,
    pub image_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolicationMatch {
    pub source: CacheSymbolSource,
    pub symbol_name: String,
    pub cache_vmaddr: u64,
    pub image_id: String,
    pub image_index: usize,
    pub install_name: String,
    pub image_base_vmaddr: u64,
    pub image_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CacheLookupResult {
    ExactSymbol {
        mapping: CacheMappingContext,
        image: CacheImageContext,
        symbol: SymbolicationMatch,
    },
    NearestSymbol {
        mapping: CacheMappingContext,
        image: CacheImageContext,
        symbol: SymbolicationMatch,
        distance: u64,
    },
    MappingOnly {
        mapping: CacheMappingContext,
        image: Option<CacheImageContext>,
    },
}

impl CacheLookupResult {
    #[allow(dead_code)]
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::ExactSymbol { .. } => "exact_symbol",
            Self::NearestSymbol { .. } => "nearest_symbol",
            Self::MappingOnly { .. } => "mapping_only",
        }
    }
}

#[derive(Debug)]
pub(crate) struct SharedCacheQueryEngine {
    #[allow(dead_code)]
    pub cache_uuid: String,
    mappings: Vec<CacheMappingRecord>,
    images: Vec<ProjectedCacheImage>,
    image_id_index: BTreeMap<String, usize>,
    install_name_index: BTreeMap<String, usize>,
    basename_index: BTreeMap<String, Vec<usize>>,
}

impl SharedCacheQueryEngine {
    pub(crate) fn new(
        cache_uuid: impl Into<String>,
        mappings: Vec<CacheMappingRecord>,
        images: Vec<ProjectedCacheImage>,
    ) -> Self {
        let mut image_id_index = BTreeMap::new();
        let mut install_name_index = BTreeMap::new();
        let mut basename_index = BTreeMap::<String, Vec<usize>>::new();

        for (position, image) in images.iter().enumerate() {
            image_id_index.insert(image.image_id.clone(), position);
            install_name_index.insert(image.install_name.clone(), position);
            basename_index
                .entry(image.basename().to_string())
                .or_default()
                .push(position);
        }

        Self {
            cache_uuid: cache_uuid.into(),
            mappings,
            images,
            image_id_index,
            install_name_index,
            basename_index,
        }
    }

    pub(crate) fn project_image(&self, selector: &str) -> Result<&ProjectedCacheImage> {
        let position = self.resolve_image_position(selector)?;
        Ok(&self.images[position])
    }

    #[allow(dead_code)]
    pub(crate) fn project_binary_image(&self, selector: &str) -> Result<&BinaryImage> {
        Ok(&self.project_image(selector)?.binary_image)
    }

    #[allow(dead_code)]
    pub(crate) fn exports_for_image(
        &self,
        selector: &str,
        name_filter: Option<&str>,
    ) -> Result<Vec<SymbolicationMatch>> {
        let image = self.project_image(selector)?;
        let mut matches = image
            .binary_image
            .dyld()
            .exported_symbols
            .iter()
            .filter(|export_record| {
                name_filter
                    .map(|needle| {
                        contains_ascii_case_insensitive(export_record.name.as_str(), needle)
                    })
                    .unwrap_or(true)
            })
            .filter_map(|export_record| self.export_match_for(image, export_record))
            .collect::<Vec<_>>();

        matches.sort_by_key(|symbol_match| {
            (
                symbol_match.cache_vmaddr,
                symbol_match.symbol_name.clone(),
                symbol_match.image_index,
            )
        });
        Ok(matches)
    }

    pub(crate) fn lookup_cache_vmaddr(&self, cache_vmaddr: u64) -> Result<CacheLookupResult> {
        let mapping = self
            .mappings
            .iter()
            .find(|mapping_record| mapping_record.contains_address(cache_vmaddr))
            .ok_or(SharedCacheQueryError::AddressNotMapped(cache_vmaddr))?;
        let member_file_offset = mapping
            .member_file_offset_for(cache_vmaddr)
            .ok_or_else(|| {
                SharedCacheQueryError::ProjectionFailed("mapping overflow".to_string())
            })?;
        let mapping_context = CacheMappingContext {
            member_label: mapping.member_label.clone(),
            mapping_base_vmaddr: mapping.mapping_base_vmaddr,
            mapping_size: mapping.mapping_size,
            member_file_offset,
        };

        let image = self.find_image_for_address(cache_vmaddr);
        let Some(image) = image else {
            return Ok(CacheLookupResult::MappingOnly {
                mapping: mapping_context,
                image: None,
            });
        };

        let image_context = self.image_context(image, cache_vmaddr)?;
        if let Some(symbol) = self.exact_symbol_for_image(image, cache_vmaddr)? {
            return Ok(CacheLookupResult::ExactSymbol {
                mapping: mapping_context,
                image: image_context,
                symbol,
            });
        }

        if let Some((symbol, distance)) = self.nearest_symbol_for_image(image, cache_vmaddr)? {
            return Ok(CacheLookupResult::NearestSymbol {
                mapping: mapping_context,
                image: image_context,
                symbol,
                distance,
            });
        }

        Ok(CacheLookupResult::MappingOnly {
            mapping: mapping_context,
            image: Some(image_context),
        })
    }

    pub(crate) fn resolve_exact_symbol(
        &self,
        symbol_name: &str,
        image_filter: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SymbolicationMatch>> {
        let mut matches = Vec::new();
        let limit = limit.unwrap_or(usize::MAX);
        for image in self.filtered_images(image_filter) {
            for export_record in image.binary_image.dyld().exports_named(symbol_name) {
                if let Some(symbol_match) = self.export_match_for(image, export_record) {
                    matches.push(symbol_match);
                }
            }
            if image.local_symbols_available {
                for symbol in image.binary_image.symbols() {
                    if symbol.name == symbol_name && symbol.defined {
                        matches.push(self.local_symbol_match(image, symbol)?);
                    }
                }
            }
        }

        matches.sort_by_key(|symbol_match| {
            (
                symbol_match.cache_vmaddr,
                symbol_match.image_index,
                symbol_source_rank(symbol_match.source),
                symbol_match.symbol_name.clone(),
            )
        });
        matches.dedup_by(|left, right| {
            left.source == right.source
                && left.symbol_name == right.symbol_name
                && left.cache_vmaddr == right.cache_vmaddr
                && left.image_id == right.image_id
        });
        if matches.is_empty() {
            return Err(SharedCacheQueryError::SymbolNotFound(
                symbol_name.to_string(),
            ));
        }
        if matches.len() > limit {
            matches.truncate(limit);
        }
        Ok(matches)
    }

    fn filtered_images<'a>(
        &'a self,
        image_filter: Option<&'a str>,
    ) -> impl Iterator<Item = &'a ProjectedCacheImage> {
        self.images.iter().filter(move |image| {
            image_filter
                .map(|needle| {
                    contains_ascii_case_insensitive(&image.install_name, needle)
                        || contains_ascii_case_insensitive(&image.image_id, needle)
                        || contains_ascii_case_insensitive(image.basename(), needle)
                })
                .unwrap_or(true)
        })
    }

    fn resolve_image_position(&self, selector: &str) -> Result<usize> {
        if let Some(position) = self.image_id_index.get(selector) {
            return Ok(*position);
        }
        if let Some(position) = self.install_name_index.get(selector) {
            return Ok(*position);
        }
        if let Some(matches) = self.basename_index.get(selector) {
            if matches.len() == 1 {
                return Ok(matches[0]);
            }
            let mut candidates = matches
                .iter()
                .map(|position| self.images[*position].install_name.clone())
                .collect::<Vec<_>>();
            candidates.sort();
            return Err(SharedCacheQueryError::CacheImageAmbiguous {
                selector: selector.to_string(),
                candidates,
            });
        }
        Err(SharedCacheQueryError::CacheImageNotFound(
            selector.to_string(),
        ))
    }

    fn find_image_for_address(&self, cache_vmaddr: u64) -> Option<&ProjectedCacheImage> {
        self.images
            .iter()
            .filter(|image| image.contains_address(cache_vmaddr))
            .min_by_key(|image| (image.image_size, image.image_index))
    }

    fn image_context(
        &self,
        image: &ProjectedCacheImage,
        cache_vmaddr: u64,
    ) -> Result<CacheImageContext> {
        let image_offset = image.image_offset_for(cache_vmaddr).ok_or_else(|| {
            SharedCacheQueryError::ProjectionFailed("image offset underflow".to_string())
        })?;
        Ok(CacheImageContext {
            image_id: image.image_id.clone(),
            image_index: image.image_index,
            install_name: image.install_name.clone(),
            image_base_vmaddr: image.image_base_vmaddr,
            image_size: image.image_size,
            image_offset,
        })
    }

    fn exact_symbol_for_image(
        &self,
        image: &ProjectedCacheImage,
        cache_vmaddr: u64,
    ) -> Result<Option<SymbolicationMatch>> {
        for export_record in &image.binary_image.dyld().exported_symbols {
            if export_record.address == Some(cache_vmaddr) {
                if let Some(symbol_match) = self.export_match_for(image, export_record) {
                    return Ok(Some(symbol_match));
                }
            }
        }
        if image.local_symbols_available {
            for symbol in image.binary_image.symbols() {
                if symbol.address == cache_vmaddr && symbol.defined {
                    return Ok(Some(self.local_symbol_match(image, symbol)?));
                }
            }
        }
        Ok(None)
    }

    fn nearest_symbol_for_image(
        &self,
        image: &ProjectedCacheImage,
        cache_vmaddr: u64,
    ) -> Result<Option<(SymbolicationMatch, u64)>> {
        let mut candidates = Vec::<SymbolicationMatch>::new();
        for export_record in &image.binary_image.dyld().exported_symbols {
            if let Some(address) = export_record.address {
                if address <= cache_vmaddr {
                    if let Some(symbol_match) = self.export_match_for(image, export_record) {
                        candidates.push(symbol_match);
                    }
                }
            }
        }
        if image.local_symbols_available {
            for symbol in image.binary_image.symbols() {
                if symbol.defined && symbol.address <= cache_vmaddr {
                    candidates.push(self.local_symbol_match(image, symbol)?);
                }
            }
        }

        candidates.sort_by_key(|symbol_match| {
            (
                std::cmp::Reverse(symbol_match.cache_vmaddr),
                symbol_source_rank(symbol_match.source),
                symbol_match.symbol_name.clone(),
            )
        });

        let Some(best) = candidates.into_iter().next() else {
            return Ok(None);
        };
        let distance = cache_vmaddr.checked_sub(best.cache_vmaddr).ok_or_else(|| {
            SharedCacheQueryError::ProjectionFailed("distance underflow".to_string())
        })?;
        Ok(Some((best, distance)))
    }

    fn export_match_for(
        &self,
        image: &ProjectedCacheImage,
        export_record: &ExportRecord,
    ) -> Option<SymbolicationMatch> {
        let cache_vmaddr = export_record.address?;
        let image_offset = cache_vmaddr.checked_sub(image.image_base_vmaddr)?;
        Some(SymbolicationMatch {
            source: CacheSymbolSource::Export,
            symbol_name: export_record.name.clone(),
            cache_vmaddr,
            image_id: image.image_id.clone(),
            image_index: image.image_index,
            install_name: image.install_name.clone(),
            image_base_vmaddr: image.image_base_vmaddr,
            image_offset,
        })
    }

    fn local_symbol_match(
        &self,
        image: &ProjectedCacheImage,
        symbol: &Symbol,
    ) -> Result<SymbolicationMatch> {
        let image_offset = symbol
            .address
            .checked_sub(image.image_base_vmaddr)
            .ok_or_else(|| {
                SharedCacheQueryError::ProjectionFailed(format!(
                    "local symbol address {:#x} falls below image base {:#x}",
                    symbol.address, image.image_base_vmaddr
                ))
            })?;
        Ok(SymbolicationMatch {
            source: CacheSymbolSource::Local,
            symbol_name: symbol.name.clone(),
            cache_vmaddr: symbol.address,
            image_id: image.image_id.clone(),
            image_index: image.image_index,
            install_name: image.install_name.clone(),
            image_base_vmaddr: image.image_base_vmaddr,
            image_offset,
        })
    }
}

#[allow(dead_code)]
pub(crate) fn project_cache_image_from_dyld_image<'data, 'cache, E, R>(
    cache_uuid: &str,
    image_index: usize,
    image: &DyldCacheImage<'data, 'cache, E, R>,
    binary_image: BinaryImage,
    member_label: Option<String>,
    local_symbols_available: bool,
) -> Result<ProjectedCacheImage>
where
    E: Endian,
    R: ReadRef<'data>,
{
    let install_name = image.path().map_err(|error| {
        SharedCacheQueryError::ProjectionFailed(format!(
            "failed to read projected image path: {error}"
        ))
    })?;
    let object_file = image.parse_object().map_err(|error| {
        SharedCacheQueryError::ProjectionFailed(format!(
            "failed to parse projected cache image object: {error}"
        ))
    })?;
    let mut min_address = u64::MAX;
    let mut max_address = 0u64;
    for segment in object_file.segments() {
        min_address = min_address.min(segment.address());
        max_address = max_address.max(segment.address().saturating_add(segment.size()));
    }
    if min_address == u64::MAX {
        return Err(SharedCacheQueryError::ProjectionFailed(
            "projected image has no segments".to_string(),
        ));
    }
    let image_size = max_address.saturating_sub(min_address).max(1);
    Ok(ProjectedCacheImage::new(
        format!("{cache_uuid}:{image_index}"),
        image_index,
        install_name.to_string(),
        min_address,
        image_size,
        member_label,
        binary_image,
        local_symbols_available,
    ))
}

fn symbol_source_rank(source: CacheSymbolSource) -> u8 {
    match source {
        CacheSymbolSource::Export => 0,
        CacheSymbolSource::Local => 1,
    }
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(needle.to_ascii_lowercase().as_str())
}

fn install_name_basename(install_name: &str) -> &str {
    install_name.rsplit('/').next().unwrap_or(install_name)
}

fn infer_image_size(binary_image: &BinaryImage, image_base_vmaddr: u64) -> Option<u64> {
    let max_end = binary_image
        .segments()
        .iter()
        .map(|segment| segment.address.saturating_add(segment.size))
        .max()?;
    max_end
        .checked_sub(image_base_vmaddr)
        .filter(|size| *size > 0)
}
