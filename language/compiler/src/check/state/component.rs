use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed, GlobalEnvironment};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    Capture, CheckInputState, CheckOutputState, ExportLookupKey, ExportLookupState, FlowState,
    ImportTable, SolutionTable, StaticCondition, Term, TermId, TermTable, VariableId,
    VariableTable,
};
use crate::CheckError;
use crate::{Compiler, CompilerError, CompilerResult};

/// State for checking one resolved component.
pub(in crate::check) struct CheckState<'a> {
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
    /// Module inputs keyed by module id.
    pub(in crate::check) inputs: IndexMap<ModuleId, CheckInputState>,
    /// Module imports keyed by module id.
    pub(in crate::check) imports: IndexMap<ModuleId, ImportTable>,
    /// Module flow states keyed by module id.
    pub(in crate::check) flows: IndexMap<ModuleId, FlowState>,
    /// Module captures keyed by module id.
    pub(in crate::check) captures: IndexMap<ModuleId, Vec<Capture>>,
    /// Module static availability keyed by module id.
    pub(in crate::check) availability:
        IndexMap<ModuleId, IndexMap<dir::GlobalSymbolId, StaticCondition>>,
    /// Module diagnostics keyed by module id.
    pub(in crate::check) diagnostics: IndexMap<ModuleId, Vec<CheckError>>,
    /// Module outputs keyed by module id.
    pub(in crate::check) outputs: IndexMap<ModuleId, CheckOutputState>,
    /// Loaded out-of-component dependencies keyed by module id.
    pub(in crate::check) dependencies: IndexMap<ModuleId, CheckDependencyState>,
    /// Component-wide variable graph.
    pub(in crate::check) variables: VariableTable,
    /// Component-wide term table.
    pub(in crate::check) terms: TermTable,
    /// Component-wide solver solutions.
    pub(in crate::check) solutions: SolutionTable,
    /// Export lookups already computed during this check component.
    pub(in crate::check) exports: HashMap<ExportLookupKey, ExportLookupState>,
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
    /// The checked generic table.
    pub(in crate::check) generics: dir::GenericTable<'static>,
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

impl<'a> CheckState<'a> {
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
            inputs: IndexMap::new(),
            imports: IndexMap::new(),
            flows: IndexMap::new(),
            captures: IndexMap::new(),
            availability: IndexMap::new(),
            diagnostics: IndexMap::new(),
            outputs: IndexMap::new(),
            dependencies: IndexMap::new(),
            variables: VariableTable::new(),
            terms: TermTable::new(),
            solutions: SolutionTable::new(),
            exports: HashMap::new(),
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

        // import checked dependency values before walk classifies references
        self.import_module_dependencies()?;

        // walk modules in stable component order
        for module in modules {
            self.walk_module(module);
        }

        Ok(())
    }

    /// Prepare fixed generic and variable state before solve.
    pub(in crate::check) fn prepare(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // induce concrete owner generics from transparent type leaves
        self.prepare_generics()?;

        // define nominal references after all induced slots are known
        for module in modules {
            self.define_nominal_symbol_types(module);
        }

        Ok(())
    }

    /// Add one term to the component term table.
    pub(in crate::check) fn intern_term<T: Term>(&mut self, term: T) -> TermId<T> {
        self.terms.push(term)
    }

    /// Return one term from the component term table.
    pub(in crate::check) fn term<T: Term>(&self, id: TermId<T>) -> T
    where
        T: Clone,
    {
        self.terms.get(id).clone()
    }

    /// Load one module.
    fn load_module(&mut self, module_id: ModuleId) -> CompilerResult<()> {
        if self.inputs.contains_key(&module_id) {
            return Ok(());
        }

        let artifacts = self.compiler.artifact_reader(self.context);
        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?
            .key;
        let module = self.compiler.module(self.context.revision(), module_id)?;
        let parsed = artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let strings = Arc::clone(self.compiler.repository.string_pool());

        let input = CheckInputState::new(
            module_id,
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
            Arc::clone(&self.environment),
        );
        let output = CheckOutputState::new(module_id, &expanded);

        // publish loaded module state
        self.inputs.insert(module_id, input);
        self.imports.insert(module_id, ImportTable::new());
        self.flows.insert(module_id, FlowState::default());
        self.captures.insert(module_id, Vec::new());
        self.availability.insert(module_id, IndexMap::new());
        self.diagnostics.insert(module_id, Vec::new());
        self.outputs.insert(module_id, output);

        Ok(())
    }

    /// Import checked dependency modules visible to component modules.
    fn import_module_dependencies(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // load module dependencies without broad symbol materialization
        for module in modules {
            let dependencies = self.module_dependency_targets(module)?;

            for dependency in dependencies {
                self.mark_dependency_module(module, dependency)?;
            }

            let namespace_dependencies = self.module_namespace_dependency_targets(module)?;
            for dependency in namespace_dependencies {
                self.import_dependency_module_symbols(module, dependency)?;
            }

            let symbols = self.module_dependency_symbols(module)?;
            for symbol in symbols {
                self.import_dependency_symbol(module, symbol)?;
            }
        }

        Ok(())
    }

    /// Return dependency modules that can be named from one component module.
    fn module_dependency_targets(&self, module: ModuleId) -> CompilerResult<IndexSet<ModuleId>> {
        let mut dependencies = IndexSet::new();
        let imports = &self.input(module).resolved.imports;

        // include direct dependency modules
        for dependency in &imports.dependencies {
            if !self.inputs.contains_key(dependency) {
                dependencies.insert(*dependency);
            }
        }

        // include resolved explicit import targets
        for (_, target) in imports.symbol_targets() {
            let dependency = target.module_id;
            if !self.inputs.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        // include resolved profile global targets
        for target in imports
            .global_symbol_by_key
            .values()
            .flat_map(|symbols| symbols.iter().copied())
        {
            let dependency = target.module_id;
            if !self.inputs.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        // include syntax-required language items
        for target in imports.language_symbols() {
            let dependency = target.module_id;
            if !self.inputs.contains_key(&dependency) {
                dependencies.insert(dependency);
            }
        }

        Ok(dependencies)
    }

    /// Return namespace import modules that require broad member visibility.
    fn module_namespace_dependency_targets(
        &self,
        module: ModuleId,
    ) -> CompilerResult<IndexSet<ModuleId>> {
        let mut dependencies = IndexSet::new();
        let imports = &self.input(module).resolved.imports;

        // include namespace import target modules
        for target in imports.target_by_symbol.values() {
            let dir::ImportTarget::Namespace(dependency) = target else {
                continue;
            };
            if !self.inputs.contains_key(dependency) {
                dependencies.insert(*dependency);
            }
        }

        Ok(dependencies)
    }

    /// Return concrete dependency symbols selected by resolve.
    fn module_dependency_symbols(
        &self,
        module: ModuleId,
    ) -> CompilerResult<IndexSet<dir::GlobalSymbolId>> {
        let mut symbols = IndexSet::new();
        let imports = &self.input(module).resolved.imports;

        // include explicit imported symbols
        for (_, target) in imports.symbol_targets() {
            if !self.inputs.contains_key(&target.module_id) {
                symbols.insert(target);
            }
        }

        // include profile global symbols
        for target in imports
            .global_symbol_by_key
            .values()
            .flat_map(|symbols| symbols.iter().copied())
        {
            if !self.inputs.contains_key(&target.module_id) {
                symbols.insert(target);
            }
        }

        // include syntax-required language item symbols
        for target in imports.language_symbols() {
            if !self.inputs.contains_key(&target.module_id) {
                symbols.insert(target);
            }
        }

        Ok(symbols)
    }

    /// Require one external symbol's module to be imported into this module.
    pub(in crate::check) fn require_symbol_module_imported(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if symbol.module_id == module || self.inputs.contains_key(&symbol.module_id) {
            return Ok(());
        }
        if self.imports(module).modules.contains(&symbol.module_id) {
            return Ok(());
        }

        let label = self
            .environment
            .language
            .item(symbol)
            .map(|item| format!(" language_item={item}"))
            .unwrap_or_default();

        Err(CompilerError::Internal {
            message: format!(
                "dependency symbol {symbol:?}{label} was not imported into module {module:?}"
            ),
        })
    }

    /// Return one syntax-required language symbol resolved for one module.
    pub(in crate::check) fn language_symbol(
        &self,
        module: ModuleId,
        item: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let symbol = self
            .input(module)
            .resolved
            .imports
            .language_symbol(item)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("language item {item} was not resolved for module {module:?}"),
            })?;

        self.require_symbol_module_imported(module, symbol)?;

        Ok(symbol)
    }

    /// Return the type variable for one symbol visible from a component module.
    pub(in crate::check) fn symbol_type_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        if self.inputs.contains_key(&symbol.module_id) {
            return Ok(self.intern_symbol_type_variable(symbol.module_id, symbol));
        }

        self.require_symbol_module_imported(module, symbol)?;

        Ok(self.intern_symbol_type_variable(module, symbol))
    }

    /// Return the static variable for one symbol visible from a component module.
    pub(in crate::check) fn symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        if self.inputs.contains_key(&symbol.module_id) {
            return Ok(self.intern_symbol_static_variable(symbol.module_id, symbol));
        }

        self.require_symbol_module_imported(module, symbol)?;

        Ok(self.intern_symbol_static_variable(module, symbol))
    }

    /// Return the static generic variable for one symbol visible from a component module.
    pub(in crate::check) fn generic_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<VariableId>> {
        let _module = if self.inputs.contains_key(&symbol.module_id) {
            symbol.module_id
        } else {
            self.require_symbol_module_imported(module, symbol)?;

            module
        };

        Ok(self.generic_static_variable_for_symbol(symbol))
    }

    /// Return checked inputs for one dependency module.
    pub(in crate::check) fn load_dependency_input(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckDependencyState> {
        if !self.dependencies.contains_key(&module) {
            let dependency = self.read_dependency_input(module)?;

            self.dependencies.insert(module, dependency);
        }

        self.dependencies
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check dependency {module:?} was not loaded"),
            })
    }

    /// Load checked inputs for one dependency module.
    fn read_dependency_input(&self, module: ModuleId) -> CompilerResult<CheckDependencyState> {
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
        let generics = checked.generic_table();
        let extensions = checked.extension_table();

        Ok(CheckDependencyState {
            parsed,
            expanded,
            bindings,
            types,
            statics,
            generics,
            extensions,
        })
    }

}
