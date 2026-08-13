use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{BodyState, Origin, Relation, VariableRole, Widening};

/// Inference mode for literal expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum InferMode {
    /// Preserve the expression's direct literal precision.
    Exact,
    /// Preserve scalar precision while widening mutable aggregate contents.
    Mutable,
    /// Infer literals as their default widened type.
    Widen,
    /// Infer under `as const` literal-preserving rules.
    Const,
}

impl InferMode {
    /// Return whether object fields inferred in this mode are readonly.
    pub(in crate::check) fn is_readonly(self) -> bool {
        matches!(self, Self::Const)
    }

    /// Return whether mutable aggregate contents widen in this mode.
    pub(in crate::check) fn widens_aggregate(self) -> bool {
        matches!(self, Self::Mutable | Self::Widen)
    }

    /// Return this mode after entering one aggregate member.
    pub(in crate::check) fn descend(self, is_readonly: bool) -> Self {
        match (self, is_readonly) {
            (Self::Const, _) => Self::Const,
            (_, true) => Self::Exact,
            (Self::Exact, false) => Self::Exact,
            (Self::Mutable | Self::Widen, false) => Self::Widen,
        }
    }
}

impl BodyState<'_, '_> {
    /// Select the literal inference mode at one contextual type position.
    pub(in crate::check) fn contextual_literal_mode(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        default_mode: InferMode,
    ) -> CompilerResult<InferMode> {
        // start the search at the written target position
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[target]);
        let mut visited = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut selected = None;

        // collect one consistent policy across transparent alternatives
        while let Some(target) = pending.pop() {
            if visited.contains(&target) {
                continue;
            }
            visited.push(target);

            match self.ty(target)? {
                // take the mode the open target position requires
                dir::Type::Variable(variable) => {
                    let mode = match self.infer.variable_role(variable)? {
                        // take the mode the bound generic parameter requires
                        VariableRole::Instantiation { parameter } => {
                            let is_const = self.require_generic_parameter(parameter)?.is_const;
                            if is_const {
                                InferMode::Const
                            }
                            // a settled contextual expectation already shapes the literal
                            else if self.has_contextual_expectation(variable)? {
                                InferMode::Exact
                            } else {
                                match self.infer.variable(variable)?.widening {
                                    Widening::Never => InferMode::Exact,
                                    Widening::Aggregate => default_mode,
                                    Widening::Multiple => InferMode::Mutable,
                                    Widening::Always => InferMode::Widen,
                                }
                            }
                        }
                        // literals widen into an inferred return
                        VariableRole::Return => match default_mode {
                            InferMode::Const => InferMode::Const,
                            _ => InferMode::Widen,
                        },
                        _ => continue,
                    };
                    if selected.is_some_and(|selected| selected != mode) {
                        return Ok(default_mode);
                    }

                    selected = Some(mode);
                }
                // consume the literal exactly at a literal target position
                dir::Type::Literal(_) => {
                    let mode = InferMode::Exact;
                    if selected.is_some_and(|selected| selected != mode) {
                        return Ok(default_mode);
                    }

                    selected = Some(mode);
                }
                // look through the form to its value
                dir::Type::Form(form) => pending.push(form.value),
                // look through alias, member, and newtype heads to their values
                dir::Type::Application(_) | dir::Type::Member(_) => {
                    let reduced = self.normalize(origin, target)?;
                    if reduced != target {
                        pending.push(reduced);
                    } else if let Some(instance) = self.newtype_payload(origin, target)? {
                        pending.push(instance.backing);
                    }
                }
                // visit every alternative of a union
                dir::Type::Union(union) => {
                    pending.extend(
                        self.type_ids(target.module_id, union.elements)?
                            .iter()
                            .copied(),
                    );
                }
                // visit every member of an intersection
                dir::Type::Intersection(intersection) => {
                    pending.extend(
                        self.type_ids(target.module_id, intersection.elements)?
                            .iter()
                            .copied(),
                    );
                }
                // look through the head of a type operation
                dir::Type::Operation(_) => match self.operation_head(target)? {
                    // template literals consume scalar precision
                    Some(dir::TypeOperation::TemplateLiteral(_)) => {
                        let mode = InferMode::Mutable;
                        if selected.is_some_and(|selected| selected != mode) {
                            return Ok(default_mode);
                        }

                        selected = Some(mode);
                    }
                    // both conditional branches are reachable positions
                    Some(dir::TypeOperation::Conditional(conditional)) => {
                        pending.push(conditional.then_type);
                        pending.push(conditional.else_type);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(selected.unwrap_or(default_mode))
    }

    /// Return the type one literal slot stores.
    pub(in crate::check) fn literal_slot_storage(
        &mut self,
        origin: Origin,
        relation: Relation,
        slot: dir::GlobalTypeId,
        source: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep the precise value under a check-only relation
        if relation == Relation::Satisfies {
            return Ok(source);
        }

        // let an open slot take the inferred candidate
        if self.type_flags(slot)?.has_variable() {
            return self.inference_candidate_type(source, mode);
        }

        // keep the selected member in a union slot under exact mode
        if mode == InferMode::Exact && matches!(self.ty(source)?, dir::Type::Literal(_)) {
            let head = self.structurally_normalize(origin, slot)?;
            if matches!(self.ty(head)?, dir::Type::Union(_)) {
                return Ok(source);
            }
        }

        Ok(slot)
    }

    /// Return one value's candidate type under its inference mode.
    pub(in crate::check) fn inference_candidate_type(
        &mut self,
        ty: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match mode {
            InferMode::Widen => self.widen_type(ty),
            InferMode::Exact | InferMode::Mutable | InferMode::Const => Ok(ty),
        }
    }

    /// Return the type of one scalar literal expression.
    pub(in crate::check) fn scalar_literal_type(
        &mut self,
        _node: dir::GlobalNodeId<dir::Expression>,
        value: dir::ScalarLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::ScalarLiteral::RegexString { .. } => {
                self.language_type(dir::LanguageItem::RegExp, &[])
            }
            dir::ScalarLiteral::Null => self.intern_type(dir::Type::Null),
            dir::ScalarLiteral::Undefined => self.intern_type(dir::Type::Undefined),
            value => self.intern_type(dir::Type::Literal(value)),
        }
    }
}
