use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, Extension, GlobalNodeIdAny, GlobalSymbolId, LocalExtensionId, SegmentView};

/// Cumulative checked extensions for one DIR module.
#[derive(Debug, Clone)]
pub struct ExtensionTable<'a> {
    /// The module id of the extension table.
    pub module_id: ModuleId,
    /// The ordered extension table segments.
    segments: SegmentView<'a, ExtensionSegment>,
}

impl ExtensionTable<'static> {
    /// Create an extension table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<ExtensionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create an extension table from one segment.
    pub fn from_segment(segment: Arc<ExtensionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> ExtensionTable<'a> {
    /// Create an extension table from a segment view.
    pub fn from_view(segments: SegmentView<'a, ExtensionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("extension table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "extension table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        for segment in self.segments.iter() {
            if let Some(extension) = segment.get_local_extension(extension_id) {
                return extension;
            }
        }

        panic!("DIR extension {extension_id:?} is not visible")
    }

    /// Return the source declaration node for one extension.
    pub fn extension_source(&self, extension_id: LocalExtensionId) -> Option<GlobalNodeIdAny> {
        for segment in self.segments.iter() {
            if segment.get_local_extension(extension_id).is_some() {
                return Some(segment.extension_source(extension_id));
            }
        }

        None
    }

    /// Get an extension id by its symbol.
    pub fn symbol_extension_id(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        for segment in self.segments.iter().rev() {
            if let Some(extension_id) = segment.symbol_extension_id(extension_symbol) {
                return Some(extension_id);
            }
        }

        None
    }

    /// Iterate extensions targeting a specific nominal symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> impl Iterator<Item = LocalExtensionId> + '_ {
        self.segments.iter().flat_map(move |segment| {
            segment
                .target_extensions(target_symbol)
                .into_iter()
                .flatten()
                .copied()
        })
    }

    /// Iterate blanket extensions.
    pub fn blanket_extensions(&self) -> impl Iterator<Item = LocalExtensionId> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.blanket_extensions().iter().copied())
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_extensions())
    }

    /// Get the number of extensions in the table.
    pub fn extension_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.extension_count())
            .unwrap_or(0)
    }
}

/// Extensions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSegment {
    /// The module id of the extension segment.
    pub module_id: ModuleId,
    /// The first extension id owned by this table segment.
    pub(crate) first_extension_id: u32,
    /// Extension records.
    pub(crate) extensions: Arena<Extension>,
    /// Source declaration nodes keyed by extension id.
    pub(crate) sources: IndexMap<LocalExtensionId, GlobalNodeIdAny>,
    /// Extension ids by declaring symbol.
    pub(crate) extensions_by_symbol: IndexMap<GlobalSymbolId, LocalExtensionId>,
    /// Extension ids by target symbol.
    pub(crate) extensions_by_target_symbol: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
    /// Blanket extension ids.
    pub(crate) blanket_extensions: Vec<LocalExtensionId>,
}

impl ExtensionSegment {
    /// Create a new extension segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_extension_id: 0,
            extensions: Arena::new(),
            sources: IndexMap::new(),
            extensions_by_symbol: IndexMap::new(),
            extensions_by_target_symbol: IndexMap::new(),
            blanket_extensions: Vec::new(),
        }
    }

    /// Insert a new extension.
    pub fn insert_extension(
        &mut self,
        source: GlobalNodeIdAny,
        extension: Extension,
    ) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.extension_count());
        let extension_symbol = extension.symbol;

        self.sources.insert(extension_id, source);
        self.extensions_by_symbol
            .insert(extension_symbol, extension_id);
        match extension.target.nominal_root() {
            Some(target_symbol) => {
                self.extensions_by_target_symbol
                    .entry(target_symbol)
                    .or_default()
                    .push(extension_id);
            }
            None => {
                self.blanket_extensions.push(extension_id);
            }
        }
        self.extensions.allocate(extension);

        extension_id
    }

    /// Return the source declaration node for one extension.
    pub fn extension_source(&self, extension_id: LocalExtensionId) -> GlobalNodeIdAny {
        *self
            .sources
            .get(&extension_id)
            .unwrap_or_else(|| panic!("DIR extension {extension_id:?} has no source"))
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.get_local_extension(extension_id).unwrap_or_else(|| {
            panic!("DIR extension {extension_id:?} is not allocated in this segment")
        })
    }

    /// Get an extension id by its symbol.
    pub fn symbol_extension_id(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.extensions_by_symbol.get(&extension_symbol).copied()
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extensions_by_target_symbol.get(&target_symbol)
    }

    /// Get all blanket extensions.
    pub fn blanket_extensions(&self) -> &Vec<LocalExtensionId> {
        &self.blanket_extensions
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> + '_ {
        (self.first_extension_id..self.extension_count()).map(|index| {
            let extension_id = LocalExtensionId::new(index);
            (extension_id, self.get_extension(extension_id))
        })
    }

    /// Get the number of extensions in the segment.
    pub fn extension_count(&self) -> u32 {
        self.first_extension_id + self.extensions.len() as u32
    }

    /// Get an extension owned by this table segment.
    pub(crate) fn get_local_extension(&self, extension_id: LocalExtensionId) -> Option<&Extension> {
        self.contains_extension_id(extension_id).then(|| {
            self.extensions
                .get(extension_id.0 - self.first_extension_id)
        })
    }

    /// Return whether this segment contains the given extension id.
    fn contains_extension_id(&self, extension_id: LocalExtensionId) -> bool {
        extension_id.0 >= self.first_extension_id && extension_id.0 < self.extension_count()
    }
}
