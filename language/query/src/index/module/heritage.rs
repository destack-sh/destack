use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one heritage index from checked DIR.
pub(super) struct HeritageIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::HeritageEntry>,
}

impl<'context, 'query> HeritageIndexer<'context, 'query> {
    /// Build the heritage index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::HeritageIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked nominal heritage edges
        indexer.collect_heritage();

        dir::HeritageIndex::new(indexer.entries)
    }

    /// Collect heritage index entries.
    fn collect_heritage(&mut self) {
        // collect checked definition relations
        for (symbol, definition) in self.module.definitions().iter_definitions() {
            self.collect_definition(symbol, definition);
        }
    }

    /// Collect heritage entries from one checked definition.
    fn collect_definition(&mut self, symbol: dir::GlobalSymbolId, definition: &dir::Definition) {
        // collect relation fields by declaration kind
        match definition {
            dir::Definition::Struct(definition) => {
                self.collect_implements(symbol, &definition.implements);
            }
            dir::Definition::Class(definition) => {
                if let Some(extends) = &definition.extends {
                    self.collect_extends(symbol, extends);
                }

                self.collect_implements(symbol, &definition.implements);
            }
            dir::Definition::Interface(definition) => {
                for extends in &definition.extends {
                    self.collect_extends(symbol, extends);
                }
            }
            dir::Definition::Enum(definition) => {
                self.collect_implements(symbol, &definition.implements);
            }
            dir::Definition::Extension(extension) => {
                // skip unresolved extension targets
                let Some(root) = extension.target.root() else {
                    return;
                };

                // collect implemented interfaces for the extended nominal
                self.collect_implements(root, &extension.implements);
            }
            dir::Definition::TypeAlias(_) | dir::Definition::Newtype(_) => {}
        }
    }

    /// Collect one extends relation.
    fn collect_extends(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        heritage: &dir::NominalHeritage,
    ) {
        self.push_heritage(derived_symbol, heritage, dir::HeritageKind::Extends);
    }

    /// Collect implements relations.
    fn collect_implements(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        implements: &[dir::NominalHeritage],
    ) {
        // emit each implemented interface edge
        for heritage in implements {
            self.push_heritage(derived_symbol, heritage, dir::HeritageKind::Implements);
        }
    }

    /// Add one heritage edge.
    fn push_heritage(
        &mut self,
        derived_symbol: dir::GlobalSymbolId,
        heritage: &dir::NominalHeritage,
        kind: dir::HeritageKind,
    ) {
        // emit heritage edge row
        self.entries.push(dir::HeritageEntry {
            derived: derived_symbol,
            base: heritage.symbol,
            source: heritage.source,
            arguments: heritage.arguments.clone(),
            kind,
        });
    }
}
