use std::sync::Arc;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{ComponentId, ModuleId};
use serde::{Deserialize, Serialize};

/// One persisted inference-component query index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct InferenceComponentIndex {
    /// The indexed inference component.
    pub component: ComponentId,
    /// The indexed query family.
    pub kind: IndexKind,
    /// Module indexes in stable component order.
    pub modules: Vec<InferenceComponentModule>,
}

impl InferenceComponentIndex {
    /// Return one indexed module.
    pub fn get(&self, module: ModuleId) -> Option<&Arc<ModuleIndex>> {
        let index = self
            .modules
            .binary_search_by_key(&module, |entry| entry.module)
            .ok()?;

        Some(&self.modules[index].index)
    }
}

/// One module row inside an inference-component query index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct InferenceComponentModule {
    /// The indexed module.
    pub module: ModuleId,
    /// The module's query index.
    pub index: Arc<ModuleIndex>,
}

/// One persisted module query index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ModuleIndex {
    /// Declared symbols.
    Symbols(dir::SymbolIndex),
    /// Resolved exports.
    Exports(dir::ExportIndex),
    /// Checked members.
    Members(dir::MemberIndex),
    /// Resolved reference occurrences.
    References(dir::ReferenceIndex),
    /// Resolved call edges.
    Calls(dir::CallIndex),
    /// Nominal heritage edges.
    Heritage(dir::HeritageIndex),
    /// Checked extensions.
    Extensions(dir::ExtensionIndex),
    /// Decorator applications.
    Decorators(dir::DecoratorIndex),
}

impl ModuleIndex {
    /// Return this module index's kind.
    pub const fn kind(&self) -> IndexKind {
        match self {
            Self::Symbols(_) => IndexKind::Symbols,
            Self::Exports(_) => IndexKind::Exports,
            Self::Members(_) => IndexKind::Members,
            Self::References(_) => IndexKind::References,
            Self::Calls(_) => IndexKind::Calls,
            Self::Heritage(_) => IndexKind::Heritage,
            Self::Extensions(_) => IndexKind::Extensions,
            Self::Decorators(_) => IndexKind::Decorators,
        }
    }

    /// Sort and deduplicate this module index.
    pub fn finish(&mut self) {
        match self {
            Self::Symbols(index) => index.finish(),
            Self::Exports(index) => index.finish(),
            Self::Members(index) => index.finish(),
            Self::References(index) => index.finish(),
            Self::Calls(index) => index.finish(),
            Self::Heritage(index) => index.finish(),
            Self::Extensions(index) => index.finish(),
            Self::Decorators(index) => index.finish(),
        }
    }
}

/// One persisted program query index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ProgramIndex {
    /// Symbol name postings.
    Symbols(dir::SymbolPostings),
    /// Export name postings.
    Exports(dir::ExportPostings),
    /// Member lookup postings.
    Members(dir::MemberPostings),
    /// Reference target postings.
    References(dir::ReferencePostings),
    /// Call graph postings.
    Calls(dir::CallPostings),
    /// Heritage lookup postings.
    Heritage(dir::HeritagePostings),
    /// Extension lookup postings.
    Extensions(dir::ExtensionPostings),
    /// Decorator name postings.
    Decorators(dir::DecoratorPostings),
}

impl ProgramIndex {
    /// Return this program index's kind.
    pub const fn kind(&self) -> IndexKind {
        match self {
            Self::Symbols(_) => IndexKind::Symbols,
            Self::Exports(_) => IndexKind::Exports,
            Self::Members(_) => IndexKind::Members,
            Self::References(_) => IndexKind::References,
            Self::Calls(_) => IndexKind::Calls,
            Self::Heritage(_) => IndexKind::Heritage,
            Self::Extensions(_) => IndexKind::Extensions,
            Self::Decorators(_) => IndexKind::Decorators,
        }
    }
}

/// One query index kind shared by module, inference-component, and program artifacts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum IndexKind {
    /// Declared symbols.
    Symbols,
    /// Resolved exports.
    Exports,
    /// Checked members.
    Members,
    /// Resolved reference occurrences.
    References,
    /// Resolved call edges.
    Calls,
    /// Nominal heritage edges.
    Heritage,
    /// Checked extensions.
    Extensions,
    /// Decorator applications.
    Decorators,
}

impl IndexKind {
    /// All query index kinds in stable order.
    pub const ALL: [Self; 8] = [
        Self::Symbols,
        Self::Exports,
        Self::Members,
        Self::References,
        Self::Calls,
        Self::Heritage,
        Self::Extensions,
        Self::Decorators,
    ];
    /// Number of module-owned index kinds.
    pub const MODULE_COUNT: usize = 2;
    /// Number of inference-component-owned index kinds.
    pub const INFERENCE_COMPONENT_COUNT: usize = 6;

    /// Return whether modules own this index family.
    pub const fn is_module_owned(self) -> bool {
        matches!(self, Self::Symbols | Self::Exports)
    }

    /// Return whether inference components own this index family.
    pub const fn is_inference_component_owned(self) -> bool {
        !self.is_module_owned()
    }

    /// Return this kind's stable program index ordinal.
    pub const fn program_index_ordinal(self) -> usize {
        match self {
            Self::Symbols => 0,
            Self::Exports => 1,
            Self::Members => 2,
            Self::References => 3,
            Self::Calls => 4,
            Self::Heritage => 5,
            Self::Extensions => 6,
            Self::Decorators => 7,
        }
    }

    /// Return this kind's module index ordinal.
    pub const fn module_index_ordinal(self) -> Option<usize> {
        match self {
            Self::Symbols => Some(0),
            Self::Exports => Some(1),
            Self::Members
            | Self::References
            | Self::Calls
            | Self::Heritage
            | Self::Extensions
            | Self::Decorators => None,
        }
    }

    /// Return this kind's inference component index ordinal.
    pub const fn inference_component_index_ordinal(self) -> Option<usize> {
        match self {
            Self::Symbols | Self::Exports => None,
            Self::Members => Some(0),
            Self::References => Some(1),
            Self::Calls => Some(2),
            Self::Heritage => Some(3),
            Self::Extensions => Some(4),
            Self::Decorators => Some(5),
        }
    }
}
