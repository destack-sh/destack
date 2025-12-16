use destack_builtin::LanguageItem;
use destack_dir::{GlobalSymbolId, StaticKey};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    /// Resolve all builtin modules and required language items.
    pub fn resolve_builtins(&self) -> ResolveResult<()> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };

        // resolve prelude/core module
        self.require_resolve_module(builtins.prelude_module_id)?;
        for &module_id in &builtins.core_modules {
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

        // check cache first
        if let Some(cached) = builtins.items.get(&item) {
            return Ok(*cached);
        }

        // get the module for this language item
        let module_id = builtins.module_for_item(item);

        // ensure the module is resolved (may yield if not ready)
        self.require_resolve_module_direct(module_id)?;

        // look up the export in the module's symbol table
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();

        let export_name = item.export_name();
        let name_id = self.program.strings.intern(export_name);

        // find the symbol in the module's namespace scope
        let namespace_scope = symbols.get_scope_by_id(module.dir.namespace_scope);
        let key = StaticKey::Name(name_id);
        let Some(symbol_id) = namespace_scope.find(key) else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        let global_id = symbol_id.into_global(module_id);

        // cache the result
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

    #[test]
    fn test_expect_language_item_resolved() {
        let test = TestProgram::memory_sequential_with_builtins();
        test.resolve_builtins();
        test.compile_dump_clean();

        // check a specific item
        let add_id = test.compiler.expect_language_item(LanguageItem::Add);

        // verify the symbol exists and has the expected name
        let module = test.program.modules.get(add_id.module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();
        let symbol = symbols.get_symbol(add_id.into_local());

        let name_id = symbol.name().expect("symbol should have a name");
        assert_string!(test.program, name_id, "Add");
    }
}
