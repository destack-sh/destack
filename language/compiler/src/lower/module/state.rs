use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported, DirMaterialized,
    DirParsed, DirResolved, DirView, MirDeclared,
};
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{Compiler, CompilerError, CompilerResult, LowerError};

/// One module's DIR, read during lowering.
pub(crate) struct DirModule {
    /// The stages this module lowers from, through materialization.
    stages: DirView,
    /// The module roots.
    pub(in crate::lower) roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The type slot of every node and symbol.
    pub(in crate::lower) types: dir::TypeTable<'static>,
    /// The inference decisions.
    pub(in crate::lower) decisions: dir::DecisionTable<'static>,
    /// The lexical resolutions.
    pub(in crate::lower) resolutions: dir::ResolutionTable<'static>,
    /// The lexical scopes and symbols.
    pub(in crate::lower) bindings: dir::BindingTable<'static>,
    /// The coercions.
    pub(in crate::lower) coercions: dir::CoercionTable<'static>,
    /// The declaration definitions.
    pub(in crate::lower) definitions: dir::DefinitionTable<'static>,
    /// The static values.
    pub(in crate::lower) statics: dir::StaticTable<'static>,
    /// The generic templates and parameters.
    pub(in crate::lower) generics: dir::GenericTable<'static>,
    /// The member functions the compiler derived for witnesses.
    pub(in crate::lower) derived_functions: FxIndexSet<dir::GlobalSymbolId>,
    /// The decorator applications.
    pub(in crate::lower) decorators: dir::DecoratorTable<'static>,
    /// The closure captures.
    pub(in crate::lower) captures: dir::CaptureTable<'static>,
    /// The layout policies of the nominal declarations.
    pub(in crate::lower) representations: dir::RepresentationTable<'static>,
    /// The member selections.
    pub(in crate::lower) members: dir::MemberTable<'static>,
    /// The symbol each declaration node declares.
    declared_symbols: FxIndexMap<dir::GlobalNodeIdAny, dir::LocalSymbolId>,
    /// The definition declaring each method symbol.
    declared_methods: FxIndexMap<dir::GlobalSymbolId, DeclaredMethod>,
    /// The canonical symbol path of the module.
    pub(in crate::lower) path: String,
}

/// One method as its definition declares it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeclaredMethod {
    /// The definition declaring the method.
    pub(crate) owner: dir::GlobalSymbolId,
    /// The role the method plays.
    pub(crate) role: Option<dir::FunctionRole>,
    /// Whether the method lives in the static space.
    pub(crate) is_static: bool,
}

impl DirModule {
    /// Read one module's materialized stage into its lowering state.
    pub(crate) fn load(
        compiler: &Compiler,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Self> {
        let view = DirView::materialized(
            artifacts.read::<DirParsed>(module)?,
            artifacts.read::<DirBound>((module, profile))?,
            artifacts.read::<DirImported>((module, profile))?,
            artifacts.read::<DirExpanded>((module, profile))?,
            artifacts.read::<DirResolved>((module, profile))?,
            artifacts.read::<DirDeclared>((module, profile))?,
            artifacts.read::<DirElaborated>((module, profile))?,
            artifacts.read::<DirChecked>((module, profile))?,
            artifacts.read::<DirMaterialized>((module, profile))?,
        );
        let path = compiler.module_symbol_path(context, module)?;

        Ok(Self::new(view, path))
    }

    /// Compose the state of one module from its stages.
    fn new(view: DirView, path: String) -> Self {
        let materialized = Arc::clone(
            view.materialized
                .as_ref()
                .unwrap_or_else(|| unreachable!("a lowered view without its materialized stage")),
        );
        let bindings = view.bindings().clone();
        let definitions = view.definitions().clone();

        // index the symbol each declaration node declares
        let mut declared_symbols = FxIndexMap::default();
        for id in bindings.symbol_ids() {
            if let Some(node) = bindings.get_symbol(id).declaration {
                declared_symbols.insert(node, id);
            }
        }

        // index the definition declaring each method
        let mut declared_methods = FxIndexMap::default();
        for (owner, definition) in definitions.iter_definitions() {
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member {
                    declared_methods.insert(
                        method.symbol,
                        DeclaredMethod {
                            owner,
                            role: method.role,
                            is_static: method.space == dir::MemberSpace::Static,
                        },
                    );
                }
            }
        }

        Self {
            roots: materialized.roots.to_vec(),
            types: view.types().clone(),
            resolutions: view.resolutions().clone(),
            decisions: view.decisions().clone(),
            bindings,
            coercions: view.coercions().clone(),
            definitions,
            statics: view.statics().clone(),
            generics: view.generics().clone(),
            derived_functions: view
                .generics()
                .iter_witnesses()
                .flat_map(|(_, _, witness)| witness.functions.iter())
                .filter(|function| function.source == dir::WitnessSource::Derived)
                .map(|function| function.function.symbol)
                .collect(),
            decorators: view.decorators().clone(),
            captures: view.captures().clone(),
            representations: view.representations().clone(),
            members: view.members().clone(),
            path,
            declared_symbols,
            declared_methods,
            stages: view,
        }
    }

    /// Return the symbol one declaration node declares.
    pub(in crate::lower) fn declared_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::LocalSymbolId> {
        self.declared_symbols.get(&node).copied()
    }

    /// Return the definition declaring one method symbol.
    pub(in crate::lower) fn declared_method(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<DeclaredMethod> {
        self.declared_methods.get(&symbol).copied()
    }

    /// Return the expression tree with every patch over it.
    pub(in crate::lower) fn tree(&self) -> dir::View<'_> {
        self.stages.tree()
    }
}

impl ModuleLowerer<'_> {
    /// Read one module's artifact stack into the loaded modules.
    fn load_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let state = DirModule::load(
            self.compiler,
            self.context,
            &self.artifacts,
            self.profile,
            module,
        )?;
        self.modules.insert(module, state);

        Ok(())
    }

    /// Return the state of one loaded module.
    pub(in crate::lower) fn state(&mut self, module: ModuleId) -> CompilerResult<&DirModule> {
        if !self.modules.contains_key(&module) {
            self.load_module(module)?;
        }

        Ok(&self.modules[&module])
    }

    /// Return the state of the module being lowered.
    pub(in crate::lower) fn local(&self) -> &DirModule {
        match self.modules.get(&self.module) {
            Some(state) => state,
            None => unreachable!("the lowered module is always loaded"),
        }
    }
}

/// Which of the two lowering artifacts is being built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LowerPhase {
    /// The module's declarations, foreign ones reserved under their symbols.
    Declare,
    /// The module's bodies, foreign declarations filled from their declaring modules.
    Lower,
}

/// One module's declared MIR, read while lowering the modules naming it.
pub(in crate::lower) struct DeclaredModule {
    /// The declared stage.
    pub(in crate::lower) stage: Arc<MirDeclared>,
    /// The declared function of each symbol.
    functions: FxIndexMap<mir::Symbol, mir::FunctionId>,
    /// The declared global of each symbol.
    globals: FxIndexMap<mir::Symbol, mir::GlobalId>,
}

impl DeclaredModule {
    /// Index one declared stage by symbol.
    fn new(stage: Arc<MirDeclared>) -> Self {
        let functions = stage
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, function)| (function.symbol, id))
            .collect();
        let globals = stage
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, global)| (global.symbol, id))
            .collect();

        Self {
            stage,
            functions,
            globals,
        }
    }
}

impl ModuleLowerer<'_> {
    /// Read one module's declared MIR on first use, a blocked read yielding to the engine.
    fn load_declared(&mut self, module: ModuleId) -> CompilerResult<()> {
        if !self.declared.contains_key(&module) {
            let stage = self
                .artifacts
                .read::<MirDeclared>((module, self.profile, self.target))?;
            self.declared.insert(module, DeclaredModule::new(stage));
        }

        Ok(())
    }

    /// Fill one foreign declaration's definition from its module's declared tree.
    pub(in crate::lower) fn import_type(
        &mut self,
        tree: &mut mir::Tree,
        module: ModuleId,
        symbol: mir::Symbol,
        name: &str,
    ) -> CompilerResult<()> {
        if self.phase == LowerPhase::Declare {
            return Ok(());
        }
        let declared = self.declared_stage(module)?;
        let Some(ty) = declared.tree.identified_type(symbol) else {
            return Err(self.undeclared_import(module, name)?);
        };
        self.import_declared(tree, |importer| {
            importer.import_type(&declared.tree, ty);
        })
    }

    /// Return the diagnostic for one import its declaring module failed to declare.
    fn undeclared_import(&mut self, module: ModuleId, name: &str) -> CompilerResult<CompilerError> {
        let path = self.state(module)?.path.clone();

        Ok(LowerError::Unsupported {
            anchor: self.module.into(),
            construct: format!("{name} the module '{path}' failed to declare"),
        }
        .into())
    }

    /// Define one base class's storage from its module's declared tree.
    pub(in crate::lower) fn fill_heritage(
        &mut self,
        tree: &mut mir::Tree,
        storage: mir::TypeId,
    ) -> CompilerResult<()> {
        // read the declaration behind an application of a generic base
        let storage = match *tree.get(storage) {
            mir::Type::Application { base, .. } => base,
            _ => storage,
        };
        if tree.is_defined_type(storage) {
            return Ok(());
        }
        let Some(symbol) = tree.type_symbol(storage) else {
            return Ok(());
        };
        let module = symbol.declaring_module();
        if module == self.module {
            return Ok(());
        }
        let declared = self.declared_stage(module)?;
        let Some(ty) = declared.tree.identified_type(symbol) else {
            return Err(self.undeclared_import(module, "a base class")?);
        };

        // hand over the base's module alone, its own imports staying reserved
        let base = Arc::clone(&declared.tree);
        let mut declared_of = |other: ModuleId| (other == module).then(|| Arc::clone(&base));
        mir::Importer::new(tree, &mut declared_of).import_type(&declared.tree, ty);

        Ok(())
    }

    /// Run one import reading declared trees on first reach, a blocked read yielding after it.
    fn import_declared<T>(
        &mut self,
        tree: &mut mir::Tree,
        import: impl FnOnce(&mut mir::Importer<'_, '_>) -> T,
    ) -> CompilerResult<T> {
        let mut blocked = None;
        let mut declared_of = |module: ModuleId| match self.declared_stage(module) {
            Ok(stage) => Some(Arc::clone(&stage.tree)),
            Err(error) => {
                blocked.get_or_insert(error);

                None
            }
        };
        let imported = import(&mut mir::Importer::new(tree, &mut declared_of));

        match blocked {
            Some(error) => Err(error),
            None => Ok(imported),
        }
    }

    /// Read one module's declared MIR on first use.
    fn declared_stage(&mut self, module: ModuleId) -> CompilerResult<Arc<MirDeclared>> {
        self.load_declared(module)?;

        Ok(self.declared[&module].stage.clone())
    }

    /// Import one foreign function's header from its module's declared tree.
    pub(in crate::lower) fn import_function(
        &mut self,
        tree: &mut mir::Tree,
        module: ModuleId,
        symbol: mir::Symbol,
        name: &str,
    ) -> CompilerResult<mir::Function> {
        if self.phase == LowerPhase::Declare {
            return Err(CompilerError::Internal {
                message: format!("the foreign callable '{name}' reached while declaring"),
            });
        }
        self.load_declared(module)?;
        let Some(function) = self.declared[&module].functions.get(&symbol).copied() else {
            return Err(self.undeclared_import(module, name)?);
        };
        let declared = self.declared[&module].stage.clone();

        self.import_declared(tree, |importer| {
            importer.import_function_header(&declared.tree, function)
        })
    }

    /// Import one foreign global from its module's declared tree.
    pub(in crate::lower) fn import_global(
        &mut self,
        tree: &mut mir::Tree,
        module: ModuleId,
        symbol: mir::Symbol,
    ) -> CompilerResult<Option<mir::Global>> {
        if self.phase == LowerPhase::Declare {
            return Err(CompilerError::Internal {
                message: format!("the foreign global {symbol:?} reached while declaring"),
            });
        }
        self.load_declared(module)?;
        let Some(global) = self.declared[&module].globals.get(&symbol).copied() else {
            return Ok(None);
        };
        let declared = self.declared[&module].stage.clone();
        let global = self.import_declared(tree, |importer| {
            importer.import_global(&declared.tree, global)
        })?;

        Ok(Some(global))
    }
}
