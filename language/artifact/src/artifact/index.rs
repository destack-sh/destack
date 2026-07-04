use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::ArtifactProjectionFingerprint;

/// Indexed checked DIR facts for one module profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ModuleIndex {
    /// Indexed declared symbols.
    pub symbols: dir::SymbolIndex,
    /// Indexed exports.
    pub exports: dir::ExportIndex,
    /// Indexed checked members.
    pub members: dir::MemberIndex,
    /// Indexed reference memberships.
    pub references: dir::ReferenceIndex,
    /// Indexed call edges.
    pub calls: dir::CallIndex,
    /// Indexed nominal heritage edges.
    pub heritage: dir::HeritageIndex,
    /// Indexed checked extensions.
    pub extensions: dir::ExtensionIndex,
    /// Indexed module specifiers.
    pub specifiers: dir::SpecifierIndex,
    /// Indexed decorators.
    pub decorators: dir::DecoratorIndex,
}

impl ModuleIndex {
    /// Sort and deduplicate all index sections.
    pub fn finish(&mut self) {
        self.symbols.finish();
        self.exports.finish();
        self.members.finish();
        self.references.finish();
        self.calls.finish();
        self.heritage.finish();
        self.extensions.finish();
        self.specifiers.finish();
        self.decorators.finish();
    }

    /// Return the stable fingerprint of one module index projection.
    pub fn projection_fingerprint(
        &self,
        projection: ModuleIndexProjection,
    ) -> ArtifactProjectionFingerprint {
        match projection {
            ModuleIndexProjection::Symbols => {
                ArtifactProjectionFingerprint::new(&dir::SymbolPostings::new(&[&self.symbols]))
            }
            ModuleIndexProjection::Exports => {
                ArtifactProjectionFingerprint::new(&dir::ExportPostings::new(&[&self.exports]))
            }
            ModuleIndexProjection::Members => {
                ArtifactProjectionFingerprint::new(&dir::MemberPostings::new(&[&self.members]))
            }
            ModuleIndexProjection::References => ArtifactProjectionFingerprint::new(
                &dir::ReferencePostings::new(&[&self.references]),
            ),
            ModuleIndexProjection::Calls => {
                ArtifactProjectionFingerprint::new(&dir::CallPostings::new(&[&self.calls]))
            }
            ModuleIndexProjection::Heritage => {
                ArtifactProjectionFingerprint::new(&dir::HeritagePostings::new(&[&self.heritage]))
            }
            ModuleIndexProjection::Extensions => ArtifactProjectionFingerprint::new(
                &dir::ExtensionPostings::new(&[&self.extensions]),
            ),
            ModuleIndexProjection::Specifiers => ArtifactProjectionFingerprint::new(
                &dir::SpecifierPostings::new(&[&self.specifiers]),
            ),
            ModuleIndexProjection::Decorators => ArtifactProjectionFingerprint::new(
                &dir::DecoratorPostings::new(&[&self.decorators]),
            ),
        }
    }
}

/// Indexed checked DIR module set for one program profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProgramIndex {
    /// The indexed modules in stable ordinal order.
    pub modules: Vec<IndexedModule>,
    /// Symbol postings.
    pub symbols: dir::SymbolPostings,
    /// Export postings.
    pub exports: dir::ExportPostings,
    /// Member postings.
    pub members: dir::MemberPostings,
    /// Reference postings.
    pub references: dir::ReferencePostings,
    /// Call postings.
    pub calls: dir::CallPostings,
    /// Heritage postings.
    pub heritage: dir::HeritagePostings,
    /// Extension postings.
    pub extensions: dir::ExtensionPostings,
    /// Module specifier postings.
    pub specifiers: dir::SpecifierPostings,
    /// Decorator postings.
    pub decorators: dir::DecoratorPostings,
}

/// One module segment in a program index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct IndexedModule {
    /// The module id.
    pub module: ModuleId,
}

/// One observable projection of a module index artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ModuleIndexProjection {
    /// Symbol postings.
    Symbols,
    /// Export postings.
    Exports,
    /// Member postings.
    Members,
    /// Reference postings.
    References,
    /// Call postings.
    Calls,
    /// Heritage postings.
    Heritage,
    /// Extension postings.
    Extensions,
    /// Specifier postings.
    Specifiers,
    /// Decorator postings.
    Decorators,
}

impl ModuleIndexProjection {
    /// All module index projections in stable order.
    pub const ALL: [Self; 9] = [
        Self::Symbols,
        Self::Exports,
        Self::Members,
        Self::References,
        Self::Calls,
        Self::Heritage,
        Self::Extensions,
        Self::Specifiers,
        Self::Decorators,
    ];
}
