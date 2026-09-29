use std::sync::Arc;

use tspp_artifact::MirLowered;
use tspp_mir as mir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::instantiate::state::InstantiateState;

impl InstantiateState<'_> {
    /// Return the witness of one closed type for one constraint.
    pub(crate) fn witness(
        &mut self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> CompilerResult<Option<mir::Witness>> {
        if let Some(witness) = self.witnesses.get(concrete, constraint) {
            return Ok(Some(witness.clone()));
        }

        // search the homes, the concrete first
        let homes = [self.home_module(concrete), self.home_module(constraint)];
        for module in homes.into_iter().flatten() {
            if let Some(witness) = self.import_witness(module, concrete, constraint)? {
                self.witnesses.insert(witness.clone());

                return Ok(Some(witness));
            }
        }

        Ok(None)
    }

    /// Import one foreign module's witness for one type pair.
    fn import_witness(
        &mut self,
        module: ModuleId,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> CompilerResult<Option<mir::Witness>> {
        if module == self.module {
            return Ok(None);
        }
        let source = self.home_source(module)?;

        // match the foreign pairs of the same nominal
        let nominal = Self::nominal_symbol(&self.tree, concrete);
        for foreign in source.witnesses.iter() {
            if nominal != Self::nominal_symbol(&source.tree, foreign.concrete) {
                continue;
            }
            let foreign_concrete = self.import_type(module, foreign.concrete, &[]);
            let foreign_concrete = mir::erase_lifetimes(&self.tree, foreign_concrete);
            let foreign_constraint = self.import_type(module, foreign.constraint, &[]);
            let foreign_constraint = mir::erase_lifetimes(&self.tree, foreign_constraint);
            if foreign_concrete != concrete || foreign_constraint != constraint {
                continue;
            }

            // import the witness members
            let functions = foreign
                .functions
                .iter()
                .map(|function| mir::WitnessFunction {
                    requirement: self.import_function(module, &source.tree, function.requirement),
                    implementation: match &function.implementation {
                        mir::WitnessImplementation::Function {
                            function,
                            arguments,
                        } => mir::WitnessImplementation::Function {
                            function: self.import_function(module, &source.tree, *function),
                            arguments: arguments
                                .iter()
                                .map(|argument| {
                                    argument
                                        .as_ref()
                                        .map(|argument| self.import_argument(module, argument))
                                })
                                .collect(),
                        },
                        mir::WitnessImplementation::Virtual { slot } => {
                            mir::WitnessImplementation::Virtual { slot: *slot }
                        }
                        mir::WitnessImplementation::Default => mir::WitnessImplementation::Default,
                        mir::WitnessImplementation::Dynamic { slot } => {
                            mir::WitnessImplementation::Dynamic { slot: *slot }
                        }
                    },
                })
                .collect();
            let types = foreign
                .types
                .iter()
                .map(|witness_type| mir::WitnessType {
                    member: witness_type.member,
                    ty: self.import_type(module, witness_type.ty, &[]),
                })
                .collect();
            let constants = foreign
                .constants
                .iter()
                .map(|constant| mir::WitnessConst {
                    member: constant.member,
                    global: self.import_global(module, &source.tree, constant.global),
                    value: constant
                        .value
                        .map(|value| self.import_static(module, &source.tree, value)),
                })
                .collect();

            return Ok(Some(mir::Witness {
                concrete,
                constraint,
                functions,
                types,
                constants,
            }));
        }

        Ok(None)
    }

    /// Return the function declaring one language item in a type's home.
    pub(crate) fn home_language_function(
        &mut self,
        ty: mir::TypeId,
        item: mir::LanguageItem,
    ) -> CompilerResult<Option<mir::FunctionId>> {
        let Some(module) = self.home_module(ty) else {
            return Ok(None);
        };
        if let Some(function) = self.language_functions.get(&(module, item)) {
            return Ok(*function);
        }
        let source = match module == self.module {
            true => None,
            false => Some(self.home_source(module)?),
        };
        let tree = source.as_ref().map_or(&self.tree, |source| &source.tree);

        // find and import the declaration once
        let found = tree
            .iter_nodes::<mir::Function>()
            .find(|(function, _)| mir::LanguageItem::of(tree, *function) == Some(item))
            .map(|(function, _)| function);
        let function = match (found, source) {
            (Some(function), Some(source)) => {
                Some(self.import_function(module, &source.tree, function))
            }
            (found, _) => found,
        };
        self.language_functions.insert((module, item), function);

        Ok(function)
    }

    /// Import one foreign generic argument into this tree.
    pub(crate) fn import_argument(
        &mut self,
        module: ModuleId,
        argument: &mir::GenericArgument,
    ) -> mir::GenericArgument {
        match argument {
            mir::GenericArgument::Type(ty) => {
                mir::GenericArgument::Type(self.import_type(module, *ty, &[]))
            }
            other => other.clone(),
        }
    }

    /// Return the home module of one type.
    fn home_module(&self, ty: mir::TypeId) -> Option<ModuleId> {
        Self::nominal_symbol(&self.tree, ty)?.module()
    }

    /// Return the nominal symbol one type names.
    fn nominal_symbol(tree: &mir::Tree, ty: mir::TypeId) -> Option<mir::Symbol> {
        match tree.get(ty) {
            mir::Type::Declaration { declaration } => Some(tree.get(*declaration).symbol),
            mir::Type::Application { base, .. } => Self::nominal_symbol(tree, *base),
            mir::Type::Reference { pointee, .. } => Self::nominal_symbol(tree, *pointee),
            _ => None,
        }
    }

    /// Return the lowered MIR of one home module.
    pub(crate) fn home_source(&mut self, module: ModuleId) -> CompilerResult<Arc<MirLowered>> {
        if let Some(source) = self.sources.get(&module) {
            return Ok(source.clone());
        }

        // read the home
        let source = self
            .artifacts
            .read::<MirLowered>((module, self.profile, self.target))?;
        self.add_source(module, source.clone());
        self.index_templates(module, &source);

        Ok(source)
    }
}
