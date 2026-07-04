mod call;
mod decorator;
mod export;
mod extension;
mod heritage;
mod member;
mod reference;
mod specifier;
mod symbol;

use destack_artifact::ModuleIndex;

use crate::ModuleQueryContext;

/// Builder for one module index from checked DIR.
pub(super) struct ModuleIndexer<'context, 'query> {
    /// The indexed module query context.
    pub(super) module: &'context ModuleQueryContext<'query>,
}

impl ModuleIndexer<'_, '_> {
    /// Build one module index from checked DIR.
    pub(super) fn build(self) -> ModuleIndex {
        let module = self.module;

        // build each checked module index family
        ModuleIndex {
            symbols: symbol::SymbolIndexer::build(module),
            exports: export::ExportIndexer::build(module),
            members: member::MemberIndexer::build(module),
            references: reference::ReferenceIndexer::build(module),
            calls: call::CallIndexer::build(module),
            heritage: heritage::HeritageIndexer::build(module),
            extensions: extension::ExtensionIndexer::build(module),
            specifiers: specifier::SpecifierIndexer::build(module),
            decorators: decorator::DecoratorIndexer::build(module),
        }
    }
}
