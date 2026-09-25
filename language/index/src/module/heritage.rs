use tspp_dir as dir;
use tspp_repository::{ProviderError, ProviderResult};

use super::context::ModuleIndexContext;

/// Builder for one heritage index from checked DIR.
pub(crate) struct HeritageIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::HeritageEntry>,
}

impl<'context, 'index> HeritageIndexer<'context, 'index> {
    /// Build the heritage index.
    pub(crate) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> ProviderResult<dir::HeritageIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked nominal heritage edges
        indexer.collect_heritage()?;

        Ok(dir::HeritageIndex::new(indexer.entries))
    }

    /// Collect heritage index entries.
    fn collect_heritage(&mut self) -> ProviderResult<()> {
        // collect checked definition relations
        for (symbol, definition) in self.module.definitions().iter_definitions() {
            self.collect_definition(symbol, definition)?;
        }

        Ok(())
    }

    /// Collect heritage entries from one checked definition.
    fn collect_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> ProviderResult<()> {
        // collect relation fields by declaration kind
        match definition {
            dir::Definition::Struct(definition) => {
                self.collect_implements(symbol, symbol, &definition.implements, 0)?;
            }
            dir::Definition::Class(definition) => {
                let mut ordinal = 0;
                if let Some(extends) = &definition.extends {
                    self.collect_extends(symbol, symbol, extends, ordinal)?;
                    ordinal += 1;
                }

                self.collect_implements(symbol, symbol, &definition.implements, ordinal)?;
            }
            dir::Definition::Interface(definition) => {
                for (ordinal, extends) in definition.extends.iter().enumerate() {
                    self.collect_extends(symbol, symbol, extends, ordinal as u32)?;
                }
            }
            dir::Definition::Enum(definition) => {
                self.collect_implements(symbol, symbol, &definition.implements, 0)?;
            }
            dir::Definition::Extension(extension) => {
                // index extensions rooted in a declaration
                let Some(root) = extension.target.declaration() else {
                    return Ok(());
                };

                // collect implemented interfaces for the extended nominal
                self.collect_implements(root, symbol, &extension.implements, 0)?;
            }
            dir::Definition::TypeAlias(_) | dir::Definition::Newtype(_) => {}
        }

        Ok(())
    }

    /// Collect one extends relation.
    fn collect_extends(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        declaration_symbol: dir::GlobalSymbolId,
        heritage: &dir::NominalHeritage,
        ordinal: u32,
    ) -> ProviderResult<()> {
        self.push_heritage(
            derived_symbol,
            declaration_symbol,
            heritage.ty,
            ordinal,
            dir::HeritageKind::Extends,
        )
    }

    /// Collect implements relations.
    fn collect_implements(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        declaration_symbol: dir::GlobalSymbolId,
        implementations: &[dir::NominalConformance],
        first_ordinal: u32,
    ) -> ProviderResult<()> {
        // emit each implemented interface edge
        for (index, implementation) in implementations.iter().enumerate() {
            let ordinal = first_ordinal + index as u32;
            self.push_heritage(
                derived_symbol,
                declaration_symbol,
                implementation.interface,
                ordinal,
                dir::HeritageKind::Implements,
            )?;
        }

        Ok(())
    }

    /// Add one heritage edge.
    fn push_heritage(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        declaration_symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        ordinal: u32,
        kind: dir::HeritageKind,
    ) -> ProviderResult<()> {
        let base = self.heritage_base(ty)?;

        // emit heritage edge entry
        self.entries.push(dir::HeritageEntry {
            derived: derived_symbol,
            declaration: declaration_symbol,
            base,
            ordinal,
            kind,
        });

        Ok(())
    }

    /// Return the nominal declaration at the head of one heritage type.
    fn heritage_base(&self, mut ty: dir::GlobalTypeId) -> ProviderResult<dir::GlobalSymbolId> {
        loop {
            // heritage types intern beside the tables that record them
            if ty.module_id != self.module.module_id() {
                return Err(ProviderError::internal(format!(
                    "heritage type escapes its module: {ty:?}"
                ))
                .into());
            }

            match self.module.types().get_type(ty.local_id) {
                dir::Type::Refined(refined) => ty = self.module.types().refined(refined).base,
                dir::Type::Application(application) => return Ok(application.symbol),
                head => {
                    return Err(ProviderError::internal(format!(
                        "heritage type has no nominal application: {ty:?}, head={head:?}"
                    ))
                    .into());
                }
            }
        }
    }
}
