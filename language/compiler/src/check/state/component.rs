use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed, GlobalEnvironment};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::check::{StaticTerm, TypeTerm};
use crate::{Compiler, CompilerError, CompilerResult};

use super::CheckModuleState;

/// State for checking one resolved component.
pub(in crate::check) struct CheckComponentState<'a> {
    /// The compiler running this check attempt.
    pub(in crate::check) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
    /// The active profile.
    pub(in crate::check) profile: ProfileId,
    /// The component modules in stable order.
    pub(in crate::check) component_modules: Vec<ModuleId>,

    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,
    /// Loaded modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
    /// Loaded out-of-component dependencies keyed by module id.
    pub(in crate::check) dependencies: IndexMap<ModuleId, CheckDependencyState>,
}

/// Checked tables loaded for one out-of-component dependency module.
pub(in crate::check) struct CheckDependencyState {
    /// The parsed dependency module.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded dependency module.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The checked binding table.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// The checked type table.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The checked static table.
    pub(in crate::check) statics: dir::StaticTable<'static>,
    /// The checked extension table.
    pub(in crate::check) extensions: dir::ExtensionTable<'static>,
}

impl CheckDependencyState {
    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }
}

impl<'a> CheckComponentState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        profile: ProfileId,
        component_modules: Vec<ModuleId>,
        environment: Arc<GlobalEnvironment>,
    ) -> Self {
        Self {
            compiler,
            context,
            profile,
            component_modules,
            environment,
            modules: IndexMap::new(),
            dependencies: IndexMap::new(),
        }
    }

    /// Load all modules in one check component.
    pub(in crate::check) fn load(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // load modules in stable component order
        for module in modules {
            self.load_module(module)?;
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // walk modules in stable component order
        for module in modules {
            let check_module = self.module_mut(module)?;
            check_module.walk().map_err(CompilerError::from)?;
        }

        // link visible external symbols after walk has created demanded variables
        self.define_visible_symbol_variables()?;

        Ok(())
    }

    /// Load one module.
    fn load_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        if self.modules.contains_key(&module) {
            return Ok(());
        }

        let artifacts = self.compiler.artifact_reader(self.context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let strings = Arc::clone(self.compiler.repository.string_pool());

        let check_module = CheckModuleState::new(
            module,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            Arc::clone(&self.environment),
        );

        // publish loaded module state
        self.modules.insert(module, check_module);

        Ok(())
    }

    /// Define variables for imported and profile-global symbols visible in component modules.
    fn define_visible_symbol_variables(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // define each module's visible imports and globals
        for module in modules {
            self.define_module_import_variables(module)?;
            self.define_module_global_variables(module)?;
        }

        Ok(())
    }

    /// Define variables for explicit imports in one module.
    fn define_module_import_variables(&mut self, module: ModuleId) -> CompilerResult<()> {
        let imports = self
            .module(module)?
            .input
            .resolved
            .imports
            .symbol_targets()
            .collect::<Vec<_>>();

        // link each explicit import symbol to its selected target
        for (symbol, target) in imports {
            self.define_visible_symbol(module, symbol, target)?;
        }

        Ok(())
    }

    /// Define variables for profile globals visible in one module.
    fn define_module_global_variables(&mut self, module: ModuleId) -> CompilerResult<()> {
        let globals = self
            .module(module)?
            .input
            .resolved
            .imports
            .global_symbol_by_key
            .values()
            .flat_map(|symbols| symbols.iter().copied())
            .collect::<Vec<_>>();

        // link each profile global symbol to itself in this module's check state
        for target in globals {
            self.define_visible_symbol(module, target, target)?;
        }

        Ok(())
    }

    /// Define one visible symbol from a component module or checked dependency.
    fn define_visible_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.modules.contains_key(&target.module_id) {
            self.define_component_symbol_alias(module, symbol, target)
        } else {
            self.import_dependency_symbol(module, symbol, target)
        }
    }

    /// Define one component-local visible symbol alias.
    fn define_component_symbol_alias(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.component_symbol_is_extension(target)? {
            return Ok(());
        }

        let type_variable = self.module_mut(module)?.symbol_type_variable(symbol);
        let target_type_variable = self
            .module_mut(target.module_id)?
            .symbol_type_variable(target);
        let has_static_variable = self
            .module(module)?
            .work
            .variables
            .static_by_symbol
            .contains_key(&symbol);
        let check_module = self.module_mut(module)?;

        check_module.define_type_term(type_variable, TypeTerm::Variable(target_type_variable));
        if has_static_variable {
            let static_variable = self.module_mut(module)?.symbol_static_variable(symbol);
            let target_static_variable = self
                .module_mut(target.module_id)?
                .symbol_static_variable(target);
            let check_module = self.module_mut(module)?;

            check_module.define_static_term(
                static_variable,
                StaticTerm::Variable(target_static_variable),
            );
        }

        Ok(())
    }

    /// Import one checked dependency symbol into one component module.
    fn import_dependency_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let is_extension = self.dependency_symbol_is_extension(target)?;
        let dependency = self.dependency_input(target.module_id)?;
        let types = dependency.types.clone();
        let statics = dependency.statics.clone();
        let imported = self
            .module_mut(module)?
            .import_symbol(symbol, target, &types, &statics);

        if !imported && !is_extension {
            return Err(CompilerError::Internal {
                message: format!("checked dependency symbol {target:?} has no checked value"),
            });
        }

        Ok(())
    }

    /// Return whether one component symbol declares an extension.
    fn component_symbol_is_extension(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let check_module = self.module(symbol.module_id)?;
        let Some(source) = check_module.symbol_source_node(symbol) else {
            return Ok(false);
        };
        if source.ty != dir::NodeType::Declaration {
            return Ok(false);
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(source.id);
        let is_extension = matches!(
            check_module.input.view().get(declaration),
            dir::Declaration::Extension(_)
        );

        Ok(is_extension)
    }

    /// Return whether one dependency symbol declares an extension.
    fn dependency_symbol_is_extension(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let dependency = self.dependency_input(symbol.module_id)?;
        let is_extension = dependency.extensions.symbol_extension_id(symbol).is_some();

        Ok(is_extension)
    }

    /// Return checked inputs for one dependency module.
    pub(in crate::check) fn dependency_input(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckDependencyState> {
        if !self.dependencies.contains_key(&module) {
            let dependency = self.load_dependency(module)?;

            self.dependencies.insert(module, dependency);
        }

        self.dependencies
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check dependency {module:?} was not loaded"),
            })
    }

    /// Load checked inputs for one dependency module.
    fn load_dependency(&self, module: ModuleId) -> CompilerResult<CheckDependencyState> {
        let artifacts = self.compiler.artifact_reader(self.context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module, self.profile)
            .map_err(CompilerError::from)?;
        let bindings = expanded.binding_table(bound.as_ref());
        let types = checked.type_table(bound.as_ref(), expanded.as_ref());
        let statics = checked.static_table(bound.as_ref(), expanded.as_ref());
        let extensions = checked.extension_table();

        Ok(CheckDependencyState {
            parsed,
            expanded,
            bindings,
            types,
            statics,
            extensions,
        })
    }

    /// Define one out-of-component dependency symbol in a component module.
    pub(in crate::check) fn define_dependency_symbol(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.modules.contains_key(&target.module_id) {
            return Ok(());
        }

        self.import_dependency_symbol(module, target, target)
    }

    /// Return one loaded module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> CompilerResult<&CheckModuleState> {
        self.modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} was not loaded"),
            })
    }

    /// Return one loaded module mutably.
    pub(in crate::check) fn module_mut(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&mut CheckModuleState> {
        self.modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} was not loaded"),
            })
    }
}
