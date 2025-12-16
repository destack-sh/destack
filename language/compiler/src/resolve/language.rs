use destack_builtin::LanguageItem;
use destack_dir::{GlobalSymbolId, StaticKey};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    /// Resolve all builtin modules and required language items.
    pub fn resolve_builtins(&self) -> ResolveResult<()> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };

        // resolve prelude/core modules
        self.require_resolve_module(builtins.prelude_module_id)?;
        for &module_id in builtins.core_module_by_path.values() {
            self.require_resolve_module(module_id)?;
        }

        // resolve all required language items
        for item in LanguageItem::required() {
            self.require_language_item(item)?;
        }

        Ok(())
    }

    /// Get a required language item, returning an error if not found.
    pub fn require_language_item(&self, item: LanguageItem) -> ResolveResult<GlobalSymbolId> {
        // check if builtins are available
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        // check cache
        if let Some(cached) = builtins.items.get(&item) {
            return Ok(*cached);
        }

        // resolve the module for this language item
        let module_id = builtins.module_for_item(item);
        self.require_resolve_module(module_id)?;

        // resolve the symbol in the module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();

        // find the symbol in the module's namespace scope
        let name_id = self.program.strings.intern(item.export_name());
        let namespace_scope = symbols.get_scope_by_id(module.dir.namespace_scope);
        let key = StaticKey::Name(name_id);
        let Some(symbol_id) = namespace_scope.find(key) else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        // result
        let global_id = symbol_id.into_global(module_id);
        builtins.items.insert(item, global_id);
        Ok(global_id)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn expect_language_item(&self, item: LanguageItem) -> GlobalSymbolId {
        let builtins = self
            .program
            .builtins
            .as_ref()
            .expect("builtins not available");
        *builtins
            .items
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not resolved"))
    }
}

#[cfg(test)]
mod tests {
    use destack_builtin::LanguageItem;

    use crate::{TestProgram, assert_string};

    /// Test that language item modules can be looked up correctly.
    #[test]
    fn test_resolve_language_item_symbol() {
        let test = TestProgram::memory_sequential_with_builtins();
        test.resolve_builtins();
        test.compile();

        let language_item_id = test.compiler.expect_language_item(LanguageItem::Add);
        let language_item_module = test.program.modules.get(language_item_id.module_id);
        let language_item_module = language_item_module.read();
        let language_item_symbols = language_item_module.dir.symbols.read();
        let language_item_symbol = language_item_symbols.get_symbol(language_item_id.into_local());
        assert_string!(test.program, language_item_symbol.name().unwrap(), "Add");
    }
}
