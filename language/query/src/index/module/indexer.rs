use destack_artifact::ModuleIndex;
use destack_repository::ProviderResult;

use crate::ModuleQueryContext;

use super::call::CallIndexer;
use super::decorator::DecoratorIndexer;
use super::export::ExportIndexer;
use super::extension::ExtensionIndexer;
use super::heritage::HeritageIndexer;
use super::member::MemberIndexer;
use super::reference::ReferenceIndexer;
use super::symbol::SymbolIndexer;

/// Builder for one module index from checked DIR.
pub(in crate::index) struct ModuleIndexer<'context, 'query> {
    /// The indexed module query context.
    pub(in crate::index) module: &'context ModuleQueryContext<'query>,
}

impl ModuleIndexer<'_, '_> {
    /// Build one module index from checked DIR.
    pub(in crate::index) fn build(self) -> ProviderResult<ModuleIndex> {
        let module = self.module;
        let references = ReferenceIndexer::build(module)?;

        // build each checked module index family
        let index = ModuleIndex {
            symbols: SymbolIndexer::build(module)?,
            exports: ExportIndexer::build(module)?,
            members: MemberIndexer::build(module),
            references,
            calls: CallIndexer::build(module)?,
            heritage: HeritageIndexer::build(module),
            extensions: ExtensionIndexer::build(module)?,
            decorators: DecoratorIndexer::build(module),
        };

        Ok(index)
    }
}
