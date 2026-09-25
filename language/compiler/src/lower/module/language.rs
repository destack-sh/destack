use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return the language item declared by one symbol.
    pub(in crate::lower) fn language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LanguageItem> {
        self.environment.language.item(symbol)
    }

    /// Mark one MIR declaration with the language item its source symbol declares.
    pub(in crate::lower) fn index_language_declaration<T>(
        &mut self,
        tree: &mut mir::Tree,
        node: mir::LocalNodeId<T>,
        symbol: dir::GlobalSymbolId,
    ) where
        T: mir::Node,
    {
        if let Some(item) = self.language_item(symbol) {
            let key = self.strings.intern(&item.key());
            tree.push_attribute(node, mir::LanguageItem::attribute(key));
        }
    }

    /// Return the symbol declaring one language item.
    pub(in crate::lower) fn language_item_symbol(
        &self,
        item: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.environment.language.symbol(item).ok_or_else(|| {
            LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a type at an unloaded '{}' representation item", item.key()),
            }
            .into()
        })
    }
}
