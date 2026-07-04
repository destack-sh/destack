mod call;
mod decorator;
mod export;
mod extension;
mod heritage;
mod member;
mod reference;
mod specifier;
mod symbol;

use destack_artifact::{ModuleIndex, ModuleIndexProjection, ProgramIndex};
use destack_source::ModuleId;

/// One current module in program index order.
pub(super) struct ProgramModule<'a> {
    /// The indexed module id.
    pub(super) module_id: ModuleId,
    /// The current module index payload.
    pub(super) index: &'a ModuleIndex,
}

/// Section changes for one program index build.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ProgramChanges {
    /// Symbol postings changed.
    pub(super) symbols: bool,
    /// Export postings changed.
    pub(super) exports: bool,
    /// Member postings changed.
    pub(super) members: bool,
    /// Reference postings changed.
    pub(super) references: bool,
    /// Call postings changed.
    pub(super) calls: bool,
    /// Heritage postings changed.
    pub(super) heritage: bool,
    /// Extension postings changed.
    pub(super) extensions: bool,
    /// Specifier postings changed.
    pub(super) specifiers: bool,
    /// Decorator postings changed.
    pub(super) decorators: bool,
}

/// Builder for one program index from module indexes.
pub(super) struct ProgramIndexer<'a> {
    /// The reusable previous program index.
    pub(super) previous: Option<&'a ProgramIndex>,
    /// The current modules.
    pub(super) modules: Vec<ProgramModule<'a>>,
    /// The changed program index sections.
    pub(super) changes: ProgramChanges,
}

impl ProgramChanges {
    /// Mark one module index projection as changed.
    pub(super) fn mark(&mut self, projection: ModuleIndexProjection, is_changed: bool) {
        match projection {
            ModuleIndexProjection::Symbols => self.symbols |= is_changed,
            ModuleIndexProjection::Exports => self.exports |= is_changed,
            ModuleIndexProjection::Members => self.members |= is_changed,
            ModuleIndexProjection::References => self.references |= is_changed,
            ModuleIndexProjection::Calls => self.calls |= is_changed,
            ModuleIndexProjection::Heritage => self.heritage |= is_changed,
            ModuleIndexProjection::Extensions => self.extensions |= is_changed,
            ModuleIndexProjection::Specifiers => self.specifiers |= is_changed,
            ModuleIndexProjection::Decorators => self.decorators |= is_changed,
        }
    }

    /// Return a change set with every section changed.
    pub(super) const fn all() -> Self {
        Self {
            symbols: true,
            exports: true,
            members: true,
            references: true,
            calls: true,
            heritage: true,
            extensions: true,
            specifiers: true,
            decorators: true,
        }
    }
}

impl ProgramIndexer<'_> {
    /// Build one program index from module indexes.
    pub(super) fn build(self) -> ProgramIndex {
        // keep module ordinals stable for every postings section
        let modules = self.modules.iter().map(|module| module.module_id).collect();

        // build or reuse each postings section independently
        ProgramIndex {
            modules,
            symbols: symbol::SymbolIndexer::build(&self),
            exports: export::ExportIndexer::build(&self),
            members: member::MemberIndexer::build(&self),
            references: reference::ReferenceIndexer::build(&self),
            calls: call::CallIndexer::build(&self),
            heritage: heritage::HeritageIndexer::build(&self),
            extensions: extension::ExtensionIndexer::build(&self),
            specifiers: specifier::SpecifierIndexer::build(&self),
            decorators: decorator::DecoratorIndexer::build(&self),
        }
    }
}
