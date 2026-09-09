use std::sync::Arc;

use destack_artifact::{MirDeclared, MirLowered};
use destack_core::{FxIndexMap, StringPool};
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId};
use destack_source::{ModuleId, TargetId};

use crate::instantiate::function::Specialization;
use crate::{CompilerError, CompilerResult};

/// One module's MIR after instantiation: its tree with every demanded instance body filled.
pub(crate) struct Instantiated {
    /// The module instantiated.
    pub(crate) module: ModuleId,
    /// Target ABI layout.
    pub(crate) layout: mir::TargetLayout,
    /// The MIR tree, template bodies cleared and instance bodies filled.
    pub(crate) tree: mir::Tree,
    /// Type layouts covering the instances.
    pub(crate) layouts: mir::LayoutTable,
    /// Drop hooks by type.
    pub(crate) drops: mir::DropTable,
    /// Memory accesses by instruction.
    pub(crate) accesses: mir::AccessTable,
    /// Function and call effects.
    pub(crate) effects: mir::EffectTable,
    /// The specializations the instantiation gave bodies.
    pub(crate) specializations: Vec<mir::FunctionId>,
}

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
    /// Drop hooks by type.
    pub(super) drops: mir::DropTable,
    /// Memory accesses by instruction.
    pub(super) accesses: mir::AccessTable,
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
    /// Open the instantiation of one module over its lowered MIR.
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
            drops: lowered.drops.clone(),
            accesses: lowered.accesses.clone(),
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
        self.index();

        // queue every shared specialization still without a body
        for (id, function) in self.tree.iter_nodes::<mir::Function>() {
            if function.linkage == mir::Linkage::Shared && function.body.is_none() {
                self.pending.push(id);
            }
        }

        // give each queued specialization its body, the calls it makes queueing more
        while let Some(instance) = self.pending.pop() {
            self.specialize(instance)?;
            self.specializations.push(instance);
        }

        Ok(())
    }

    /// Return the instantiated tables with layouts covering every specialization.
    pub(crate) fn finish(mut self) -> CompilerResult<Instantiated> {
        // yield to the engine until every declared tree an import reached is built
        if let Some(blocked) = self.blocked.take() {
            return Err(blocked);
        }

        // clear the template bodies, read by instantiation alone
        let templates: Vec<_> = self
            .tree
            .iter_nodes::<mir::Function>()
            .filter(|(_, function)| function.body.is_some() && function.is_polymorphic())
            .map(|(id, _)| id)
            .collect();
        for template in templates {
            self.tree.get_mut(template).clear_body();
        }

        mir::LayoutBuilder::new(&self.tree, &mut self.layouts, self.layout)
            .layout_reachable_types()
            .map_err(|error| CompilerError::Internal {
                message: format!("instance layouts failed: {error:?}"),
            })?;

        Ok(Instantiated {
            module: self.module,
            layout: self.layout,
            tree: self.tree,
            layouts: self.layouts,
            drops: self.drops,
            accesses: self.accesses,
            effects: self.effects,
            specializations: self.specializations,
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
        let arguments = self.tree.get(instance).arguments.clone();
        let (module, template) = self.template_definition(instance)?;
        let source = self.sources[&module].clone();

        // copy the template's body at the arguments
        let mut specialization = Specialization {
            state: self,
            module,
            source: &source.tree,
            template,
            arguments,
            locals: FxIndexMap::default(),
            blocks: FxIndexMap::default(),
            added_values: Vec::new(),
        };
        let body = specialization.body(&source.accesses)?;

        // carry the template's effects over to the instance
        let effect = source.effects.function(template).cloned();
        if let Some(effect) = effect {
            *self.effects.upsert_function(instance) = effect;
        }
        self.tree.get_mut(instance).set_body(body);

        Ok(())
    }
}
