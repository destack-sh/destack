use destack_artifact::LanguageEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build language item bindings from builtin modules.
    pub(in crate::import) fn build_language_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<LanguageEnvironment> {
        let mut environment = LanguageEnvironment::default();
        let artifacts = self.artifact_reader(context);

        let modules = self.repository.builtin_package().module_ids();

        // scan builtin modules for language item declarations
        for module_id in modules {
            let parsed = artifacts
                .dir_parsed(module_id)
                .map_err(CompilerError::from)?;
            let bound = artifacts
                .dir_bound(module_id, profile)
                .map_err(CompilerError::from)?;
            let bindings = bound.binding_table();

            self.collect_language_items(module_id, &parsed.tree, &bindings, &mut environment)?;
        }

        Ok(environment)
    }

    /// Collect language item declarations from one bound module.
    fn collect_language_items(
        &self,
        module: ModuleId,
        tree: &dir::Tree,
        bindings: &dir::BindingTable<'_>,
        environment: &mut LanguageEnvironment,
    ) -> CompilerResult<()> {
        for (target_id, decorators) in tree.get_all_decorators() {
            // recover declaration target
            let node_type = tree.get_node_type(*target_id);
            let target = dir::LocalNodeIdAny::new(*target_id, node_type);

            // collect language item markers attached to this declaration
            for decorator_id in decorators {
                let decorator = tree.get(*decorator_id);
                self.collect_language_item(module, tree, bindings, target, decorator, environment)?;
            }
        }

        Ok(())
    }

    /// Collect one marked language item declaration.
    fn collect_language_item(
        &self,
        module: ModuleId,
        tree: &dir::Tree,
        bindings: &dir::BindingTable<'_>,
        target: dir::LocalNodeIdAny,
        decorator: &dir::Decorator,
        environment: &mut LanguageEnvironment,
    ) -> CompilerResult<()> {
        // skip unrelated decorators
        let Some(key) = self.read_language_item_key(tree, decorator) else {
            return Ok(());
        };

        // resolve item identity
        let Some(item) = dir::LanguageItem::from_key(key) else {
            return Err(CompilerError::Internal {
                message: format!("unknown language item: {key}"),
            });
        };

        // find decorated symbol
        let global_target = target.into_global(module);
        let Some(symbol_id) = bindings.symbol_for_declaration(global_target) else {
            return Err(CompilerError::Internal {
                message: format!("language item has no symbol: {key}"),
            });
        };

        // verify symbol form
        let symbol = bindings.get_symbol(symbol_id);
        let expected_form = dir::SymbolForm::from(item.form());
        if symbol.form != expected_form {
            return Err(CompilerError::Internal {
                message: format!(
                    "language item {key} expects {:?}, got {:?}",
                    expected_form, symbol.form
                ),
            });
        }

        // bind language item
        let global_symbol = symbol_id.into_global(module);
        self.define_language_item(environment, item, global_symbol)?;

        // bind symbol name
        if let Some(name) = symbol.name() {
            environment.symbols.entry(name).or_insert(global_symbol);
        }

        Ok(())
    }

    /// Read the stable language item key from one decorator.
    fn read_language_item_key<'a>(
        &'a self,
        tree: &dir::Tree,
        decorator: &dir::Decorator,
    ) -> Option<&'a str> {
        // match call shaped decorators
        let dir::Expression::Call {
            left, arguments, ..
        } = tree.get(decorator.expression)
        else {
            return None;
        };

        // match @languageItem
        let dir::Expression::Identifier { name } = tree.get(*left) else {
            return None;
        };
        if self.strings().get(*name) != "languageItem" {
            return None;
        }

        // read key argument
        let [argument] = arguments.as_slice() else {
            return None;
        };
        let argument = tree.get(*argument);
        let expression = argument.value()?;
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(key)) = tree.get(expression)
        else {
            return None;
        };

        Some(self.strings().get(*key))
    }

    /// Define one language item and reject ambiguous definitions.
    fn define_language_item(
        &self,
        environment: &mut LanguageEnvironment,
        item: dir::LanguageItem,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // insert first definition
        let Some(previous) = environment.items.get(&item).copied() else {
            environment.items.insert(item, symbol);

            return Ok(());
        };

        // accept idempotent definitions
        if previous == symbol {
            Ok(())
        } else {
            Err(CompilerError::Internal {
                message: format!("duplicate language item: {item}"),
            })
        }
    }
}
