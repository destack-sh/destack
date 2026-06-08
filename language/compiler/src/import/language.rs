use destack_artifact::LanguageEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_repository::{ProfileId, ProviderContext};

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

        // scan builtin modules for language item declarations
        let modules = self.repository.builtin_package().module_ids();
        for module_id in modules {
            let parsed = artifacts
                .dir_parsed(module_id)
                .map_err(CompilerError::from)?;
            let bound = artifacts
                .dir_bound(module_id, profile)
                .map_err(CompilerError::from)?;
            let bindings = bound.binding_table();

            self.collect_language_items_into(module_id, &parsed.tree, &bindings, &mut environment)?;
        }

        Ok(environment)
    }

    /// Collect language item declarations from one bound module.
    fn collect_language_items_into(
        &self,
        module: ModuleId,
        tree: &dir::Tree,
        bindings: &dir::BindingTable<'_>,
        environment: &mut LanguageEnvironment,
    ) -> CompilerResult<()> {
        for (target_id, decorators) in tree.get_all_decorators() {
            // recover decorated target
            let node_type = tree.get_node_type(*target_id);
            let target = dir::LocalNodeIdAny::new(*target_id, node_type);

            // collect language item markers attached to this target
            for decorator_id in decorators {
                let decorator = tree.get(*decorator_id);
                if let Some(key) = self.extract_language_item_key(tree, decorator) {
                    self.collect_language_item(module, tree, bindings, target, key, environment)?;
                }
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
        key: &str,
        environment: &mut LanguageEnvironment,
    ) -> CompilerResult<()> {
        // resolve item identity
        let Some(item) = dir::LanguageItem::from_key(key) else {
            return Err(CompilerError::Internal {
                message: format!("unknown language item: {key}"),
            });
        };

        // find decorated symbol
        let Some(symbol_id) = self.language_item_symbol(module, tree, bindings, target) else {
            return Err(CompilerError::Internal {
                message: format!("language item has no symbol: {key}"),
            });
        };

        // verify symbol kind
        let symbol = bindings.get_symbol(symbol_id);
        let expected_kind = dir::SymbolKind::from(item.kind());
        if symbol.kind != expected_kind {
            return Err(CompilerError::Internal {
                message: format!(
                    "language item {key} expects {:?}, got {:?}",
                    expected_kind, symbol.kind
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

    /// Return the symbol targeted by one `@languageItem` decorator.
    fn language_item_symbol(
        &self,
        module: ModuleId,
        tree: &dir::Tree,
        bindings: &dir::BindingTable<'_>,
        target: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalSymbolId> {
        let global_target = target.into_global(module);
        if let Some(symbol_id) = bindings.symbol_for_declaration(global_target) {
            return Some(symbol_id);
        }

        // route variable expressions through their declarator pattern
        if target.ty == dir::NodeType::Expression {
            let expression_id = target.into_typed::<dir::Expression>();
            let expression = tree.get(expression_id);
            let declarators = match expression {
                dir::Expression::Let { declarators, .. } => declarators,
                dir::Expression::Using { declarators, .. } => declarators,
                _ => return None,
            };
            let [declarator_id] = declarators.as_slice() else {
                return None;
            };
            let declarator = tree.get(*declarator_id);
            let pattern = declarator.pattern.into_global_any(module);

            return bindings.symbol_for_declaration(pattern);
        }

        // route direct declarator decorators through their pattern
        if target.ty == dir::NodeType::Declarator {
            let declarator_id = target.into_typed::<dir::Declarator>();
            let declarator = tree.get(declarator_id);
            let pattern = declarator.pattern.into_global_any(module);

            return bindings.symbol_for_declaration(pattern);
        }

        None
    }

    /// Read the stable language item key from one decorator.
    fn extract_language_item_key<'a>(
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
        let Some(previous) = environment.symbol_by_item.get(&item).copied() else {
            environment.symbol_by_item.insert(item, symbol);
            environment.items_by_symbol.insert(symbol, item);

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
