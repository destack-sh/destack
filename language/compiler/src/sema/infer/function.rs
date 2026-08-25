use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CauseId, CheckFailure, CheckOutcome, Expectation, FlowSite, InferMode, Origin,
    Relation, ValueCheck,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one function value's body in its receiving context.
    pub(in crate::sema) fn check_function_value(
        &mut self,
        site: FlowSite,
        expectation: Option<Expectation>,
        mode: InferMode,
    ) -> CompilerResult<ValueCheck> {
        // read the function value's collected body
        let node = site.node;
        let Some(body) = self.check.lambdas.get(&node).cloned() else {
            return Err(CompilerError::Internal {
                message: format!("function value {node:?} has no body"),
            });
        };

        // read the declared callable and the context the body checks under
        let callable = self.check.symbol_type(body.symbol)?;
        let output_mode = expectation.map_or(mode, |expectation| expectation.mode);
        let target = expectation.map_or(callable, |expectation| expectation.target);

        // constrain the declared callable against its expected type
        let value_type = if let Some(expectation) = expectation {
            // report either rejection as the callable missing its expected type
            let origin = site.origin();
            let rejection = ValueCheck {
                source: callable,
                stored: callable,
                outcome: CheckOutcome::Fails(CheckFailure::Relation),
                target,
            };

            // require the contextual callable type to name a construction
            let contextual = self.contextual_callable(origin, expectation.target)?;
            let Some(construction) = self.construction_value(origin, contextual)? else {
                self.check.commit_node_type(node, callable)?;

                return Ok(rejection);
            };

            // take the contextual parameter and return types as the callable's own holes
            self.adopt_contextual_signature(origin, expectation.cause, callable, construction)?;

            // require the declared callable to be assignable to that value
            if !self
                .check
                .constrain_type(
                    origin,
                    expectation.cause,
                    Relation::Assignable,
                    callable,
                    construction,
                )?
                .holds()
            {
                self.check.commit_node_type(node, callable)?;

                return Ok(rejection);
            }

            // keep the source under satisfies, take the storage form otherwise
            match expectation.relation {
                Relation::Satisfies => callable,
                _ => self.replace_form_value(origin, expectation.target, callable)?,
            }
        }
        // otherwise carry the declared callable as written
        else {
            callable
        };

        // check the body against the filled-in signature
        let parent = expectation.map(|expectation| expectation.cause);
        let checked = body.check(self.check, output_mode, parent)?;
        let outcome = checked.map_or(CheckOutcome::Holds, |check| check.outcome);

        // commit the function value's own type at its node
        self.check.commit_node_type(node, value_type)?;

        Ok(ValueCheck {
            source: value_type,
            stored: value_type,
            outcome,
            target,
        })
    }

    /// Equate the callable's open parameter slots and return hole with the contextual signature.
    fn adopt_contextual_signature(
        &mut self,
        origin: Origin,
        cause: CauseId,
        callable: dir::GlobalTypeId,
        construction: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // read both signatures
        let (Some((source_module, source)), Some((target_module, target))) = (
            self.callable_signature(callable)?,
            self.callable_signature(construction)?,
        ) else {
            return Ok(());
        };

        // keep the callable's own parameter slots
        let source_parameters = self
            .check
            .signature_parameters(source_module, source.parameters)?
            .to_vec();

        // expand an open rest pack, spreading its tuple over the slots
        let mut packs = Vec::new();
        for parameter in self
            .check
            .signature_parameters(target_module, target.parameters)?
        {
            if parameter.is_rest
                && let Some(root) = self.check.root_variable(parameter.ty)?
            {
                packs.push(root);
            }
        }

        if !packs.is_empty() {
            self.check.resolve_variables(&packs)?;
        }

        // give each unannotated positional slot its contextual slot type
        let target_slots = self
            .check
            .expand_parameters(target_module, target.parameters)?;
        for (index, slot) in source_parameters.iter().enumerate() {
            // skip the annotated and rest slots
            if slot.is_rest || self.check.root_variable(slot.ty)?.is_none() {
                continue;
            }

            // read the contextual slot, spreading a trailing rest over the tail
            let contextual = match target_slots.get(index) {
                Some(target) if target.is_rest => self.check.rest_element_type(target.ty)?,
                Some(target) => target.ty,
                None => match target_slots.last() {
                    Some(target) if target.is_rest => self.check.rest_element_type(target.ty)?,
                    _ => continue,
                },
            };

            self.check
                .constrain_type(origin, cause, Relation::Equal, contextual, slot.ty)?;
        }

        // equate an inferred return with the contextual return
        if let (Some(hole), Some(contextual)) = (source.return_type, target.return_type)
            && self.check.root_variable(hole)?.is_some()
        {
            self.check
                .constrain_type(origin, cause, Relation::Equal, contextual, hole)?;
        }

        Ok(())
    }

    /// Return the signature one callable type carries, with the module owning its rows.
    fn callable_signature(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(ModuleId, dir::FunctionSignatureType)>> {
        let ty = self.shallow_resolve(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionSignature(_) => ty,
            _ => return Ok(None),
        };
        let head = self.check.signature_head(signature)?;

        Ok(head.map(|head| (signature.module_id, head)))
    }

    /// Return the type a callable takes its contextual signature from.
    ///
    /// A union expectation contributes its sole callable arm.
    fn contextual_callable(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // take a plain expectation as it is
        let head = self.check.structurally_normalize(origin, target)?;
        if !matches!(self.check.ty(head)?, dir::Type::Union(_)) {
            return Ok(target);
        }

        // flatten nested union members through their aliases, visiting each member once
        let mut pending = vec![head];
        let mut visited = FxIndexSet::default();
        let mut callables = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        while let Some(member) = pending.pop() {
            let member = self.check.structurally_normalize(origin, member)?;
            if !visited.insert(member) {
                continue;
            }

            // queue the members of a nested union
            if let dir::Type::Union(union) = self.check.ty(member)? {
                pending.extend(
                    self.check
                        .type_ids(member.module_id, union.elements)?
                        .iter()
                        .copied(),
                );
            }
            // keep the members carrying a signature
            else if self.callable_signature_type(origin, member)?.is_some() {
                callables.push(member);
            }
        }

        Ok(match callables.as_slice() {
            [callable] => *callable,
            _ => target,
        })
    }
}
