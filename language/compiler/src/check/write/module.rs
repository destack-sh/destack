use std::mem::replace;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckError, CheckState, Origin, VarianceState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write one solved module into its checked DIR segments.
    pub(in crate::check) fn write_module(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let mut sealed = FxIndexMap::default();
        let node_types = self.resolved_node_types(module, failed_applications, &mut sealed)?;
        let symbol_types = self.resolved_symbol_types(module, failed_applications, &mut sealed)?;
        let reduced_types = self.resolved_reduced_types(module, &node_types, &symbol_types)?;
        let symbol_literals = self.static_symbol_literals(module)?;

        // record inferred types and checked reduced types
        let state = self.module_mut(module);
        for (node, ty) in node_types {
            state.types_tail.set_node_type(node, ty);
        }
        for (symbol, ty) in symbol_types {
            state.types_tail.set_symbol_type(symbol, ty);
        }
        for (source, target) in reduced_types {
            state.types_tail.set_type_reduction(source, target);
        }


        // write symbol values as final statics
        for (symbol, literal) in symbol_literals {
            let id = state
                .statics
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            state
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // write closure capture frames and bindings
        self.write_captures(module)?;

        // record derived parameter variances beside their declarations
        self.write_derived_variances(module)?;

        // record marker conformances, then seal every embedded type id
        self.write_auto_conformances(module)?;
        self.seal_output_segments(module, failed_applications, &mut sealed)?;

        Ok(())
    }

    /// Record derived parameter variances on the checked generic segment.
    ///
    /// Declared modifiers stay as written; unannotated nominal type
    /// parameters record the default-context derivation the reifier and
    /// dumps show.
    fn write_derived_variances(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect the module's cached derivations first
        let derivations = self
            .variances
            .iter()
            .filter_map(|((parameter, context), state)| match state {
                VarianceState::Derived(derived) if parameter.module_id == module => {
                    Some((*parameter, *context, *derived))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        // keep only unannotated nominal type parameters at their own context
        let mut filled = Vec::new();
        for (parameter, context, derived) in derivations {
            if context != self.parameter_variance_form(parameter)? {
                continue;
            }
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };
            if binding.variance.is_some()
                || binding.origin != dir::GenericParameterOrigin::Explicit
                || binding.is_comptime()
                || binding.is_const
            {
                continue;
            }
            if !self.parameter_owner_is_nominal(parameter)? {
                continue;
            }
            let Some(modifier) = derived.modifier() else {
                continue;
            };
            filled.push((parameter.local_id, modifier));
        }

        // record the derivations on the checked tail
        for (parameter, modifier) in filled {
            self.module_mut(module)
                .generics
                .set_derived_variance(parameter, modifier);
        }

        Ok(())
    }

    /// Record representation marker conformance for each concrete nominal.
    fn write_auto_conformances(&mut self, module: ModuleId) -> CompilerResult<()> {
        // generic nominals conform per materialized instance
        let mut nominals = Vec::new();
        for (symbol, definition) in self.module(module).definitions.iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
            );
            if is_nominal && definition.template().is_none() {
                nominals.push(symbol);
            }
        }

        // seal the satisfied markers on each nominal's own instance
        for symbol in nominals {
            let instance = self.declaration_instance(module, symbol)?;
            let target = self.intern_type(module, dir::Type::Application(instance))?;
            let origin = Origin::Symbol(symbol);
            for interface in dir::AutoInterface::REPRESENTATION {
                let holds = match self.satisfies_auto_interface(origin, target, interface)? {
                    Answer::Ready(holds) => holds,
                    // leave conformance over unsolved body inferred members
                    //  to the owning inference component
                    Answer::Pending(_) if self.is_declaration() => continue,
                    Answer::Pending(_) => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "marker conformance for {symbol:?} suspended at settle"
                            ),
                        });
                    }
                };

                if holds {
                    self.module_mut(module)
                        .auto
                        .push_conformance(dir::AutoConformance { interface, target });
                }
            }
        }

        Ok(())
    }

    /// Seal every type id written into this module's output segments.
    fn seal_output_segments(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<()> {
        let mut result = Ok(());

        // seal declaration definitions
        let empty = dir::DefinitionSegment::new(module);
        let mut definitions = replace(&mut self.module_mut(module).definitions, empty);
        definitions.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).definitions = definitions;

        // seal auto implementations
        let empty = dir::AutoSegment::new(module);
        let mut auto = replace(&mut self.module_mut(module).auto, empty);
        auto.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).auto = auto;

        // seal checked decorator selections
        let empty = dir::DecoratorSegment::new(module);
        let mut decorators = replace(&mut self.module_mut(module).decorators, empty);
        decorators.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).decorators = decorators;

        // seal decided node resolutions
        let empty = dir::ResolutionSegment::new(module);
        let mut resolutions = replace(&mut self.module_mut(module).resolutions, empty);
        resolutions.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).resolutions = resolutions;

        // seal closure capture frames
        let empty = dir::CaptureSegment::new(module);
        let mut captures = replace(&mut self.module_mut(module).capture_segment, empty);
        captures.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).capture_segment = captures;

        // seal static terms
        let empty = dir::StaticSegment::new(module);
        let mut statics = replace(&mut self.module_mut(module).statics, empty);
        statics.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).statics = statics;

        // seal implicit coercions
        let empty = dir::CoercionSegment::new(module);
        let mut coercions = replace(&mut self.module_mut(module).coercions, empty);
        coercions.map_type_ids(&mut |id| {
            self.seal_or_record(id, failed_applications, sealed, &mut result)
        });
        self.module_mut(module).coercions = coercions;

        result
    }

    /// Seal one embedded type id, recording the first failure aside.
    pub(super) fn seal_or_record(
        &mut self,
        id: dir::GlobalTypeId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
        result: &mut CompilerResult<()>,
    ) -> dir::GlobalTypeId {
        match self.seal_type(id, failed_applications, sealed) {
            Ok(sealed) => sealed,
            Err(error) => {
                if result.is_ok() {
                    *result = Err(error);
                }

                id
            }
        }
    }

    /// Resolve one module's recorded node types.
    fn resolved_node_types(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self
            .node_types
            .iter()
            .map(|(node, ty)| (*node, *ty))
            .filter_map(|(node, ty)| (node.module_id == module).then_some((node, ty)))
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            let ty = self.settled_root(ty)?;

            // omit rows declarations cannot settle without bodies
            if self.is_declaration() && self.type_flags(ty)?.has_variable() {
                continue;
            }
            let ty = self.seal_type(ty, failed_applications, sealed)?;
            resolved.push((node, ty));
        }

        Ok(resolved)
    }

    /// Resolve one written reduced type.
    fn resolved_reduced_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::GlobalTypeId)>> {
        let ty = self.settled_root(ty)?;
        if self.type_flags(ty)?.has_variable() {
            return Ok(None);
        }

        match self.reduce_type(origin, ty)? {
            Answer::Ready(reduced) if reduced == ty => Ok(None),
            Answer::Ready(reduced) => Ok(Some((ty, reduced))),
            Answer::Pending(blockers) => {
                let (_, anchor) = self.origin_diagnostic_anchor(origin)?;

                Err(CompilerError::Internal {
                    message: format!(
                        "written type for {origin:?} at {anchor:?} is still pending: {blockers:?}"
                    ),
                })
            }
        }
    }

    /// Resolve one module's recorded symbol types.
    fn resolved_symbol_types(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let mut symbol_types = Vec::new();
        symbol_types.extend(
            self.declaration_types
                .iter()
                .filter_map(|(symbol, ty)| (symbol.module_id == module).then_some((*symbol, *ty))),
        );
        symbol_types.extend(
            self.binding_types
                .iter()
                .filter_map(|(symbol, ty)| (symbol.module_id == module).then_some((*symbol, *ty))),
        );

        let mut resolved = Vec::with_capacity(symbol_types.len());
        for (symbol, ty) in symbol_types {
            let ty = self.settled_root(ty)?;

            // omit rows declarations cannot settle without bodies
            if self.is_declaration() && self.type_flags(ty)?.has_variable() {
                continue;
            }
            let ty = self.seal_type(ty, failed_applications, sealed)?;

            // require re-derivations to land on the declaration pass's sealed type
            let previous = self.module(module).types_tail.get_symbol_type_id(symbol);
            if let Some(previous) = previous
                && previous != ty
            {
                return Err(CompilerError::Internal {
                    message: format!(
                        "declared symbol {symbol:?} resealed to a different type: {} was {}",
                        self.format_type(ty),
                        self.format_type(previous),
                    ),
                });
            }
            resolved.push((symbol, ty));
        }

        Ok(resolved)
    }

    /// Resolve one module's checked reduced types.
    fn resolved_reduced_types(
        &mut self,
        module: ModuleId,
        node_types: &[(dir::GlobalNodeIdAny, dir::GlobalTypeId)],
        symbol_types: &[(dir::GlobalSymbolId, dir::GlobalTypeId)],
    ) -> CompilerResult<Vec<(dir::GlobalTypeId, dir::GlobalTypeId)>> {
        let mut sources = Vec::new();
        for (node, ty) in node_types {
            sources.push((self.node_origin(*node)?, *ty));
        }
        sources.extend(
            symbol_types
                .iter()
                .map(|(symbol, ty)| (Origin::Symbol(*symbol), *ty)),
        );
        sources.extend(
            self.module(module)
                .definitions
                .iter_definitions()
                .filter_map(|(symbol, definition)| match definition {
                    dir::Definition::TypeAlias(definition) => {
                        Some((Origin::Symbol(symbol), definition.value))
                    }
                    _ => None,
                }),
        );

        let mut seen = FxIndexSet::default();
        let mut resolved = Vec::new();
        for (origin, ty) in sources {
            if !seen.insert(ty) {
                continue;
            }

            if let Some(reduction) = self.resolved_reduced_type(origin, ty)? {
                resolved.push(reduction);
            }
        }

        Ok(resolved)
    }

    /// Resolve one module's literal symbol values.
    fn static_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>> {
        let static_values = self
            .module(module)
            .static_values
            .iter()
            .map(|(symbol, value)| (*symbol, *value))
            .collect::<Vec<_>>();
        let mut literals = Vec::new();
        for (symbol, value) in static_values {
            let value = self.settled_root(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, literal));
            }
        }

        Ok(literals)
    }

    /// Seal one written type by replacing every variable with its solution.
    fn seal_type(
        &mut self,
        id: dir::GlobalTypeId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep foreign types, whose tables closed with their own component
        if !self.is_component_module(id.module_id) {
            if failed_applications.contains(&id) || self.type_flags(id)?.has_variable() {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check found unsettled state in the sealed type {}",
                        self.format_type(id)
                    ),
                });
            }

            return Ok(id);
        }

        // poison a generic application whose declared argument bound failed
        if failed_applications.contains(&id) {
            return self.intern_type(id.module_id, dir::Type::Error);
        }

        // keep settled types when no failed application can occur below them
        if failed_applications.is_empty() && !self.type_flags(id)?.has_variable() {
            return Ok(id);
        }

        // reuse an already sealed type
        if let Some(existing) = sealed.get(&id) {
            let Some(existing) = existing else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check sealed a cyclic solution graph at {}",
                        self.format_type(id)
                    ),
                });
            };

            return Ok(*existing);
        }
        sealed.insert(id, None);

        // seal a variable through its solved root, or report and poison
        let ty = self.ty(id)?;
        let result = if let dir::Type::Variable(variable) = ty {
            match self.solver.solution(variable)? {
                Some(solution) => {
                    let solution = self.settled_root(solution)?;

                    self.seal_type(solution, failed_applications, sealed)?
                }
                None => {
                    // report written unsolved variables in clean modules as
                    //  missed checks; declarations leave them to inference
                    let origin = self.solver.variable(variable)?.origin;
                    let origin = self.solver.origin(origin);
                    let module = origin.module();
                    if !self.is_declaration()
                        && self.is_component_module(module)
                        && self.module(module).diagnostics.is_empty()
                    {
                        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                        self.report(module, CheckError::CannotInferType { anchor, module });
                    }

                    self.intern_type(id.module_id, dir::Type::Error)?
                }
            }
        }
        // rebuild a composite around its sealed children, in place in its module
        else {
            let rebuilt =
                self.map_type_children(id.module_id, id.module_id, ty, &mut |state, child| {
                    state.seal_type(child, failed_applications, sealed)
                })?;

            self.intern_type(id.module_id, rebuilt)?
        };

        sealed.insert(id, Some(result));

        Ok(result)
    }
    /// Resolve one complete set of embedded type ids for source rendering.
    pub(in crate::check) fn resolve_type_ids(
        &mut self,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>> {
        let failed = FxIndexSet::default();
        let mut replacements = FxIndexMap::default();
        for id in ids {
            let replacement = self.seal_type(id, &failed, sealed)?;
            replacements.insert(id, replacement);
        }

        Ok(replacements)
    }

}
