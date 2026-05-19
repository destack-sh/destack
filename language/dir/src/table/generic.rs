use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalTypeId, SegmentView, StaticTerm, VarianceModifier};

/// Cumulative checked generic parameters for one DIR module.
#[derive(Debug, Clone)]
pub struct GenericTable<'a> {
    /// The module id of the generic table.
    pub module_id: ModuleId,
    /// The ordered generic table segments.
    segments: SegmentView<'a, GenericSegment>,
}

impl GenericTable<'static> {
    /// Create a generic table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<GenericSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a generic table from one segment.
    pub fn from_segment(segment: Arc<GenericSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> GenericTable<'a> {
    /// Create a generic table from a segment view.
    pub fn from_view(segments: SegmentView<'a, GenericSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("generic table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "generic table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a generic table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b GenericSegment) -> GenericTable<'b> {
        GenericTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate effective generic parameters keyed by parameter symbol.
    pub fn parameters(&self) -> impl Iterator<Item = (GlobalSymbolId, GenericParameterShape)> + '_ {
        let mut entries = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol_id, parameter) in &segment.parameters {
                entries.insert(*symbol_id, parameter.clone());
            }
        }

        entries.into_iter()
    }

    /// Iterate effective generic parameter lists keyed by declaration symbol.
    pub fn parameter_lists(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, GenericParameterList)> + '_ {
        let mut entries = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol_id, parameter_list) in &segment.parameter_lists {
                entries.insert(*symbol_id, parameter_list.clone());
            }
        }

        entries.into_iter()
    }

    /// Get checked generic parameter metadata for a parameter symbol.
    pub fn get_parameter(&self, symbol_id: GlobalSymbolId) -> Option<GenericParameterShape> {
        for segment in self.segments.iter().rev() {
            if let Some(parameter) = segment.get_parameter(symbol_id) {
                return Some(parameter);
            }
        }

        None
    }

    /// Get a generic parameter list by declaration symbol.
    pub fn get_parameter_list(&self, symbol_id: GlobalSymbolId) -> Option<GenericParameterList> {
        for segment in self.segments.iter().rev() {
            if let Some(parameter_list) = segment.get_parameter_list(symbol_id) {
                return Some(parameter_list.clone());
            }
        }

        None
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic binders added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// Generic parameter metadata keyed by parameter symbol.
    pub(crate) parameters: IndexMap<GlobalSymbolId, GenericParameterShape>,
    /// Generic parameter lists keyed by declaration symbol.
    pub(crate) parameter_lists: IndexMap<GlobalSymbolId, GenericParameterList>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            parameters: IndexMap::new(),
            parameter_lists: IndexMap::new(),
        }
    }

    /// Set generic parameter metadata for a parameter symbol.
    pub fn set_parameter(&mut self, symbol_id: GlobalSymbolId, parameter: GenericParameterShape) {
        self.parameters.insert(symbol_id, parameter);
    }

    /// Get checked generic parameter metadata for a parameter symbol.
    pub fn get_parameter(&self, symbol_id: GlobalSymbolId) -> Option<GenericParameterShape> {
        self.parameters.get(&symbol_id).cloned()
    }

    /// Set the generic parameter list for a declaration symbol.
    pub fn set_parameter_list(
        &mut self,
        symbol_id: GlobalSymbolId,
        parameter_list: GenericParameterList,
    ) {
        self.parameter_lists.insert(symbol_id, parameter_list);
    }

    /// Get a generic parameter list by declaration symbol.
    pub fn get_parameter_list(&self, symbol_id: GlobalSymbolId) -> Option<&GenericParameterList> {
        self.parameter_lists.get(&symbol_id)
    }

    /// Iterate generic parameters keyed by parameter symbol.
    pub fn parameters(&self) -> impl Iterator<Item = (GlobalSymbolId, GenericParameterShape)> + '_ {
        self.parameters
            .iter()
            .map(|(symbol_id, parameter)| (*symbol_id, parameter.clone()))
    }

    /// Iterate generic parameter lists keyed by declaration symbol.
    pub fn parameter_lists(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &GenericParameterList)> + '_ {
        self.parameter_lists
            .iter()
            .map(|(symbol_id, parameter_list)| (*symbol_id, parameter_list))
    }

    /// Return true when this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.parameter_lists.is_empty()
    }
}

/// Checked metadata for one generic parameter symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericParameterShape {
    /// Type parameter.
    Type {
        /// The constraint type for this parameter.
        constraint: Option<LocalTypeId>,
        /// The default type argument.
        default: Option<LocalTypeId>,
        /// The variance for this parameter.
        variance: Option<VarianceModifier>,
    },
    /// Static parameter.
    Static {
        /// The static term type accepted by this parameter.
        ty: Option<LocalTypeId>,
        /// The default static argument.
        default: Option<StaticTerm>,
    },
}

/// Checked generic parameters owned by one declaration symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericParameterList {
    /// The ordered generic parameter symbols.
    pub parameters: Vec<GlobalSymbolId>,
}
