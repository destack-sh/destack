use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, SegmentView};

/// Cumulative nominal declarations for one DIR module.
#[derive(Debug, Clone)]
pub struct NominalTable<'a> {
    /// The module id of the nominal table.
    pub module_id: ModuleId,
    /// The ordered nominal table segments.
    segments: SegmentView<'a, NominalSegment>,
}

impl NominalTable<'static> {
    /// Create a nominal table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<NominalSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a nominal table from one segment.
    pub fn from_segment(segment: Arc<NominalSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> NominalTable<'a> {
    /// Create a nominal table from a segment view.
    pub fn from_view(segments: SegmentView<'a, NominalSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("nominal table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "nominal table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Return one nominal definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&NominalDefinition> {
        for segment in self.segments.iter().rev() {
            if let Some(definition) = segment.definition(symbol) {
                return Some(definition);
            }
        }

        None
    }

    /// Return one newtype definition by symbol.
    pub fn newtype_definition(&self, symbol: GlobalSymbolId) -> Option<&NewtypeDefinition> {
        match self.definition(symbol) {
            Some(NominalDefinition::Newtype(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Return one enum definition by symbol.
    pub fn enum_definition(&self, symbol: GlobalSymbolId) -> Option<&EnumDefinition> {
        match self.definition(symbol) {
            Some(NominalDefinition::Enum(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Iterate nominal definitions in phase order.
    pub fn iter_definitions(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &NominalDefinition)> + '_ {
        let mut definitions = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol, definition) in segment.iter_definitions() {
                definitions.insert(symbol, definition);
            }
        }

        definitions.into_iter()
    }

    /// Return true when this table has no definitions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Nominal declarations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominalSegment {
    /// The module id of the nominal segment.
    pub module_id: ModuleId,
    /// Nominal definitions keyed by declaring symbol.
    pub(crate) definitions: IndexMap<GlobalSymbolId, NominalDefinition>,
}

impl NominalSegment {
    /// Create a new nominal segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            definitions: IndexMap::new(),
        }
    }

    /// Insert one nominal definition.
    pub fn insert_definition(&mut self, definition: NominalDefinition) {
        let symbol = definition.symbol();

        self.definitions.insert(symbol, definition);
    }

    /// Return one nominal definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&NominalDefinition> {
        self.definitions.get(&symbol)
    }

    /// Iterate nominal definitions in insertion order.
    pub fn iter_definitions(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &NominalDefinition)> + '_ {
        self.definitions
            .iter()
            .map(|(symbol, definition)| (*symbol, definition))
    }

    /// Return true when this segment has no definitions.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

/// Checked declaration facts for one nominal symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NominalDefinition {
    /// Struct declaration facts.
    ///
    /// Example:
    /// ```ds
    /// struct User { name: string }
    /// ```
    Struct(StructDefinition),
    /// Class declaration facts.
    ///
    /// Example:
    /// ```ds
    /// class User { name: string }
    /// ```
    Class(ClassDefinition),
    /// Enum declaration facts.
    ///
    /// Example:
    /// ```ds
    /// enum Status { Ready, Done }
    /// ```
    Enum(EnumDefinition),
    /// Newtype declaration facts.
    ///
    /// Example:
    /// ```ds
    /// newtype UserId = int64
    /// ```
    Newtype(NewtypeDefinition),
}

impl NominalDefinition {
    /// Return the declaring symbol.
    pub fn symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Struct(definition) => definition.symbol,
            Self::Class(definition) => definition.symbol,
            Self::Enum(definition) => definition.symbol,
            Self::Newtype(definition) => definition.symbol,
        }
    }
}

/// Checked facts for one struct declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDefinition {
    /// The struct symbol.
    pub symbol: GlobalSymbolId,
}

/// Checked facts for one class declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// The class symbol.
    pub symbol: GlobalSymbolId,
}

/// Checked facts for one enum declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDefinition {
    /// The enum symbol.
    pub symbol: GlobalSymbolId,
}

/// Checked facts for one newtype declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewtypeDefinition {
    /// The newtype symbol.
    pub symbol: GlobalSymbolId,
}
