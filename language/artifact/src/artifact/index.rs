use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

/// One module query index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ModuleIndex {
    /// Declared symbols.
    Symbols(dir::SymbolIndex),
    /// Resolved exports.
    Exports(dir::ExportIndex),
    /// Member declarations and implementations.
    Members(dir::MemberIndex),
    /// Resolved reference occurrences.
    References(dir::ReferenceIndex),
    /// Resolved call edges.
    Calls(dir::CallIndex),
    /// Nominal heritage edges.
    Heritage(dir::HeritageIndex),
    /// Decorator applications.
    Decorators(dir::DecoratorIndex),
    /// Code fingerprints and adjacent pairs.
    Code(dir::CodeIndex),
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
            Self::Decorators(_) => IndexKind::Decorators,
            Self::Code(_) => IndexKind::Code,
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
            Self::Decorators(index) => index.finish(),
            Self::Code(index) => index.finish(),
        }
    }
}

/// One program query index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ProgramIndex {
    /// Symbol name postings.
    Symbols(dir::SymbolPostings),
    /// Export name postings.
    Exports(dir::ExportPostings),
    /// Member implementation postings.
    Members(dir::MemberPostings),
    /// Reference target postings.
    References(dir::ReferencePostings),
    /// Call graph postings.
    Calls(dir::CallPostings),
    /// Heritage lookup postings.
    Heritage(dir::HeritagePostings),
    /// Decorator name postings.
    Decorators(dir::DecoratorPostings),
    /// Code fingerprint postings.
    Code(dir::CodePostings),
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
            Self::Decorators(_) => IndexKind::Decorators,
            Self::Code(_) => IndexKind::Code,
        }
    }
}

/// One index kind shared by module and program artifacts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum IndexKind {
    /// Declared symbols.
    Symbols,
    /// Resolved exports.
    Exports,
    /// Member declarations and implementations.
    Members,
    /// Resolved reference occurrences.
    References,
    /// Resolved call edges.
    Calls,
    /// Nominal heritage edges.
    Heritage,
    /// Decorator applications.
    Decorators,
    /// Code fingerprints.
    Code,
}

impl IndexKind {
    /// All index kinds in stable order.
    pub const ALL: [Self; 8] = [
        Self::Symbols,
        Self::Exports,
        Self::Members,
        Self::References,
        Self::Calls,
        Self::Heritage,
        Self::Decorators,
        Self::Code,
    ];
    /// Return this kind's stable ordinal.
    pub const fn ordinal(self) -> usize {
        match self {
            Self::Symbols => 0,
            Self::Exports => 1,
            Self::Members => 2,
            Self::References => 3,
            Self::Calls => 4,
            Self::Heritage => 5,
            Self::Decorators => 6,
            Self::Code => 7,
        }
    }
}
