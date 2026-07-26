use std::sync::Arc;

use destack_artifact::{ModuleIndex, ModuleIndexProjection, ProgramIndex};
use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::ModuleId;

/// One current module in program index order.
pub(in crate::index) struct ProgramModule<'a> {
    /// The indexed module id.
    pub(in crate::index) module_id: ModuleId,
    /// The current module index payload.
    pub(in crate::index) index: &'a ModuleIndex,
}

/// Changed inputs to one program index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::index) struct ProgramIndexChanges {
    /// One bit per module index projection.
    projection_bits: u8,
}

/// Builder for one program index from module indexes.
pub(in crate::index) struct ProgramIndexer<'a> {
    /// The reusable previous program index.
    pub(in crate::index) previous: Option<&'a ProgramIndex>,
    /// The current modules.
    pub(in crate::index) modules: Vec<ProgramModule<'a>>,
    /// The changed program index inputs.
    pub(in crate::index) changes: ProgramIndexChanges,
}

impl ProgramIndexChanges {
    /// Mark one module index projection as changed.
    pub(in crate::index) fn mark_projection(
        &mut self,
        projection: ModuleIndexProjection,
        is_changed: bool,
    ) {
        if is_changed {
            self.projection_bits |= Self::projection_bit(projection);
        }
    }

    /// Return whether one module index projection changed.
    fn contains_projection(self, projection: ModuleIndexProjection) -> bool {
        self.projection_bits & Self::projection_bit(projection) != 0
    }

    /// Mark every program index input as changed.
    pub(in crate::index) const fn all() -> Self {
        Self {
            projection_bits: u8::MAX,
        }
    }

    /// Return the bit assigned to this projection.
    const fn projection_bit(projection: ModuleIndexProjection) -> u8 {
        match projection {
            ModuleIndexProjection::Symbols => 1 << 0,
            ModuleIndexProjection::Exports => 1 << 1,
            ModuleIndexProjection::Members => 1 << 2,
            ModuleIndexProjection::References => 1 << 3,
            ModuleIndexProjection::Calls => 1 << 4,
            ModuleIndexProjection::Heritage => 1 << 5,
            ModuleIndexProjection::Extensions => 1 << 6,
            ModuleIndexProjection::Decorators => 1 << 7,
        }
    }
}

impl ProgramIndexer<'_> {
    /// Build one program index from module indexes.
    pub(in crate::index) fn build(self) -> ProviderResult<ProgramIndex> {
        let modules = self.modules.iter().map(|module| module.module_id).collect();

        // build or reuse each postings projection independently
        Ok(ProgramIndex {
            modules,
            symbols: self.symbols()?,
            exports: self.exports()?,
            members: self.members()?,
            references: self.references()?,
            calls: self.calls()?,
            heritage: self.heritage()?,
            extensions: self.extensions()?,
            decorators: self.decorators()?,
        })
    }

    /// Build or reuse one program postings projection.
    fn postings<Component, Postings>(
        &self,
        projection: ModuleIndexProjection,
        previous: impl FnOnce(&ProgramIndex) -> &Arc<Postings>,
        component: impl Fn(&ModuleIndex) -> &Component,
        build: impl FnOnce(&[&Component]) -> Postings,
    ) -> ProviderResult<Arc<Postings>> {
        // reuse the required previous projection when no module component changed
        if !self.changes.contains_projection(projection) {
            let index = self.previous.ok_or_else(|| {
                ProviderError::internal(format!(
                    "unchanged program index projection has no previous index: {projection:?}"
                ))
            })?;

            return Ok(previous(index).clone());
        }

        // rebuild from current module components
        let components = self
            .modules
            .iter()
            .map(|module| component(module.index))
            .collect::<Vec<_>>();

        Ok(Arc::new(build(&components)))
    }

    /// Build or reuse symbol postings.
    fn symbols(&self) -> ProviderResult<Arc<dir::SymbolPostings>> {
        self.postings(
            ModuleIndexProjection::Symbols,
            |index| &index.symbols,
            |module| &module.symbols,
            dir::SymbolPostings::build,
        )
    }

    /// Build or reuse export postings.
    fn exports(&self) -> ProviderResult<Arc<dir::ExportPostings>> {
        self.postings(
            ModuleIndexProjection::Exports,
            |index| &index.exports,
            |module| &module.exports,
            dir::ExportPostings::build,
        )
    }

    /// Build or reuse member postings.
    fn members(&self) -> ProviderResult<Arc<dir::MemberPostings>> {
        self.postings(
            ModuleIndexProjection::Members,
            |index| &index.members,
            |module| &module.members,
            dir::MemberPostings::build,
        )
    }

    /// Build or reuse reference postings.
    fn references(&self) -> ProviderResult<Arc<dir::ReferencePostings>> {
        self.postings(
            ModuleIndexProjection::References,
            |index| &index.references,
            |module| &module.references,
            dir::ReferencePostings::build,
        )
    }

    /// Build or reuse call postings.
    fn calls(&self) -> ProviderResult<Arc<dir::CallPostings>> {
        self.postings(
            ModuleIndexProjection::Calls,
            |index| &index.calls,
            |module| &module.calls,
            dir::CallPostings::build,
        )
    }

    /// Build or reuse heritage postings.
    fn heritage(&self) -> ProviderResult<Arc<dir::HeritagePostings>> {
        self.postings(
            ModuleIndexProjection::Heritage,
            |index| &index.heritage,
            |module| &module.heritage,
            dir::HeritagePostings::build,
        )
    }

    /// Build or reuse extension postings.
    fn extensions(&self) -> ProviderResult<Arc<dir::ExtensionPostings>> {
        self.postings(
            ModuleIndexProjection::Extensions,
            |index| &index.extensions,
            |module| &module.extensions,
            dir::ExtensionPostings::build,
        )
    }

    /// Build or reuse decorator postings.
    fn decorators(&self) -> ProviderResult<Arc<dir::DecoratorPostings>> {
        self.postings(
            ModuleIndexProjection::Decorators,
            |index| &index.decorators,
            |module| &module.decorators,
            dir::DecoratorPostings::build,
        )
    }
}
