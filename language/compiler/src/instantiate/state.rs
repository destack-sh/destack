use std::sync::Arc;

use tspp_artifact::{MirDeclared, MirInstantiated, MirLowered, MirVerified};
use tspp_core::{FxIndexMap, StringPool};
use tspp_mir as mir;
use tspp_repository::{ArtifactReader, ProfileId};
use tspp_source::{ModuleId, TargetId};

use crate::instantiate::function::Specialization;
use crate::verify::VerifyState;
use crate::{CompilerError, CompilerResult};

/// One module's MIR gaining the bodies its shared specializations declare.
pub(crate) struct InstantiateState<'a> {
    /// The module being instantiated.
    pub(super) module: ModuleId,
    /// The strings the trees name their symbols through.
    pub(super) strings: &'a StringPool,
    /// Target ABI layout.
    pub(super) layout: mir::TargetLayout,
    /// The MIR tree the instance bodies land in.
    pub(super) tree: mir::Tree,
    /// Type layouts covering the instances.
    pub(super) layouts: mir::LayoutTable,
    /// Dispatch shapes and tables.
    pub(super) dispatch: mir::DispatchTable,
    /// Drop hooks by type.
    pub(super) drops: mir::DropTable,
    /// Function and call effects.
    pub(super) effects: mir::EffectTable,
    /// The witness each closed type answers an interface with.
    pub(super) witnesses: mir::WitnessTable,
    /// The MIR of each reachable module, by module.
    pub(super) sources: FxIndexMap<ModuleId, Arc<MirLowered>>,
    /// This tree's functions by symbol.
    pub(super) functions: FxIndexMap<mir::Symbol, mir::FunctionId>,
    /// This tree's globals by symbol.
    pub(super) globals: FxIndexMap<mir::Symbol, mir::GlobalId>,
    /// The module and function defining each template, by symbol.
    pub(super) templates: FxIndexMap<mir::Symbol, (ModuleId, mir::FunctionId)>,
    /// The declared MIR of every declaring module read for a definition.
    pub(super) declared: FxIndexMap<ModuleId, Arc<MirDeclared>>,
    /// The first declared MIR read that blocked, yielding the instantiation to the engine.
    pub(super) blocked: Option<CompilerError>,
    /// The artifact reader the declaring modules' MIR is read through.
    pub(super) artifacts: &'a ArtifactReader<'a>,
    /// The profile the instantiated module builds under.
    pub(super) profile: ProfileId,
    /// The target the instantiated module builds for.
    pub(super) target: TargetId,
    /// The specializations whose bodies still wait for their template's.
    pub(super) pending: Vec<mir::FunctionId>,
    /// The specializations given bodies so far.
    pub(super) specializations: Vec<mir::FunctionId>,
}

impl<'a> InstantiateState<'a> {
    /// Create instantiation state from lowered MIR.
    pub(crate) fn new(
        module: ModuleId,
        lowered: Arc<MirLowered>,
        strings: &'a StringPool,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        target: TargetId,
    ) -> Self {
        Self {
            module,
            strings,
            layout: lowered.target,
            tree: mir::Tree::clone(&lowered.tree),
            layouts: lowered.layouts.clone(),
            dispatch: lowered.dispatch.clone(),
            drops: lowered.drops.clone(),
            effects: lowered.effects.clone(),
            witnesses: lowered.witnesses.clone(),
            sources: FxIndexMap::from_iter([(module, lowered)]),
            functions: FxIndexMap::default(),
            globals: FxIndexMap::default(),
            templates: FxIndexMap::default(),
            declared: FxIndexMap::default(),
            blocked: None,
            artifacts,
            profile,
            target,
            pending: Vec::new(),
            specializations: Vec::new(),
        }
    }

    /// Add the MIR of one reachable module, a source of templates to instantiate.
    pub(crate) fn add_source(&mut self, module: ModuleId, lowered: Arc<MirLowered>) {
        self.sources.insert(module, lowered);
    }

    /// Give every shared specialization its template's body at its arguments.
    pub(crate) fn instantiate(&mut self) -> CompilerResult<()> {
        // index the module's functions, globals, and templates
        self.index();

        // queue every shared specialization still without a body, keeping the concrete bodies
        let mut bodies = Vec::new();
        for (id, function) in self.tree.iter_nodes::<mir::Function>() {
            if function.linkage == mir::Linkage::Shared && function.body.is_none() {
                self.pending.push(id);
            }
            if function.body.is_some() && !function.is_polymorphic() {
                bodies.push(id);
            }
        }

        // close calls and allocations in the module's existing concrete bodies
        for function in bodies {
            self.declare_dynamic_tables(function)?;
        }

        // process each new body, including specializations its tables and drop hooks require
        while let Some(instance) = self.pending.pop() {
            self.specialize(instance)?;
            self.declare_dynamic_tables(instance)?;
            self.specializations.push(instance);
        }

        Ok(())
    }

    /// Return the instantiated tables with layouts covering every specialization.
    pub(crate) fn finish(mut self, verified: &MirVerified) -> CompilerResult<MirInstantiated> {
        // return the blocked dependency before completing instantiation
        if let Some(blocked) = self.blocked.take() {
            return Err(blocked);
        }

        // clear the generic template bodies
        let templates: Vec<_> = self
            .tree
            .iter_nodes::<mir::Function>()
            .filter(|(_, function)| function.body.is_some() && function.is_polymorphic())
            .map(|(id, _)| id)
            .collect();
        for template in templates {
            self.tree.get_mut(template).clear_body();
        }

        // compute layouts for the instantiated types
        mir::LayoutBuilder::new(&self.tree, &mut self.layouts, self.layout)
            .witnesses(&self.witnesses)
            .layout_reachable_types()
            .map_err(|error| CompilerError::Internal {
                message: format!("instance layouts failed: {error:?}"),
            })?;

        // verify the instantiated functions
        let source = &self.sources[&self.module];
        let mut analyses = VerifyState::over(
            &self.tree,
            self.strings,
            &self.drops,
            &self.dispatch,
            &self.effects,
            None,
            self.layout,
        );
        analyses.verify_functions(&self.specializations);

        // fail on invalid MIR in an instance
        if let Some(error) = analyses.take_invalid_mir() {
            return Err(error);
        }

        // reject an instance that fails verification, every failure named
        let errors = analyses.take_errors();
        if !errors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("a verified template failed at an instance: {errors:?}"),
            });
        }

        // merge retention requirements from generic and instantiated bodies
        let mut retention = verified.retention.clone();
        retention.extend(analyses.take_retention());
        retention.sort();

        Ok(MirInstantiated {
            target: self.layout,
            tree: Arc::new(self.tree),
            initializer: source.initializer,
            layouts: self.layouts,
            dispatch: self.dispatch,
            drops: self.drops,
            effects: self.effects,
            profile: source.profile.clone(),
            retention,
        })
    }

    /// Index this tree's functions and globals by symbol, and every defined template by symbol.
    fn index(&mut self) {
        // index this tree's own functions and the templates among them
        for (id, function) in self.tree.iter_nodes::<mir::Function>() {
            self.functions.entry(function.symbol).or_insert(id);
            if function.is_polymorphic() && function.body.is_some() {
                self.templates
                    .entry(function.symbol)
                    .or_insert((self.module, id));
            }
        }

        // index this tree's own globals
        for (id, global) in self.tree.iter_nodes::<mir::Global>() {
            self.globals.entry(global.symbol).or_insert(id);
        }

        // index the templates every reachable module defines
        for (module, source) in &self.sources {
            for (id, function) in source.tree.iter_nodes::<mir::Function>() {
                if function.is_polymorphic() && function.body.is_some() {
                    self.templates
                        .entry(function.symbol)
                        .or_insert((*module, id));
                }
            }
        }
    }

    /// Return the module and function defining the template one specialization applies.
    pub(super) fn template_definition(
        &self,
        instance: mir::FunctionId,
    ) -> CompilerResult<(ModuleId, mir::FunctionId)> {
        let Some(template) = self.tree.get(instance).template else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a specialization of '{}' without its template",
                    self.strings.get(self.tree.get(instance).name)
                ),
            });
        };
        let symbol = self.tree.get(template).symbol;

        self.templates
            .get(&symbol)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a specialization of '{}' without a template body in any reachable module",
                    self.strings.get(self.tree.get(template).name)
                ),
            })
    }

    /// Give one specialization its template's body at the specialization's arguments.
    fn specialize(&mut self, instance: mir::FunctionId) -> CompilerResult<()> {
        // read the instance arguments and generic template
        let arguments = self.tree.get(instance).arguments.clone();
        let (module, template) = self.template_definition(instance)?;
        let source = self.sources[&module].clone();

        // require one argument per template parameter
        let slots = source.tree.get(template).generics.len();
        if arguments.len() != slots {
            return Err(CompilerError::Internal {
                message: format!(
                    "a specialization of '{}' with {} arguments for {slots} slots",
                    self.strings.get(self.tree.get(instance).name),
                    arguments.len()
                ),
            });
        }

        // copy the template's body at the arguments
        let mut specialization = Specialization {
            state: self,
            module,
            source: &source.tree,
            template,
            arguments,
            locals: FxIndexMap::default(),
            blocks: FxIndexMap::default(),
        };
        let body = specialization.body()?;

        // copy the template's effects to the instance
        let effect = source.effects.function(template).cloned();
        if let Some(effect) = effect {
            *self.effects.upsert_function(instance) = effect;
        }
        self.tree.get_mut(instance).set_body(body);

        Ok(())
    }
}
