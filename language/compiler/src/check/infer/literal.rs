use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, Origin, Relation, VariableRole, Widening, answer};

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
    /// Select literal inference at one contextual type position.
    pub(in crate::check) fn contextual_literal_mode(
        &self,
        target: dir::GlobalTypeId,
        default_mode: InferMode,
    ) -> CompilerResult<InferMode> {
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
                dir::Type::Variable(variable) => {
                    let VariableRole::Instantiation { parameter } =
                        self.solver.variable_role(variable)?
                    else {
                        continue;
                    };
                    let binding = self.require_generic_parameter(parameter)?;
                    let mode = if binding.is_const {
                        InferMode::Const
                    } else {
                        match self.solver.variable(variable)?.widening {
                            Widening::Never => InferMode::Exact,
                            Widening::Aggregate => default_mode,
                            Widening::Multiple => InferMode::Mutable,
                            Widening::Always => InferMode::Widen,
                        }
                    };
                    if selected.is_some_and(|selected| selected != mode) {
                        return Ok(default_mode);
                    }
                    selected = Some(mode);
                }
                dir::Type::Form(form) => pending.push(form.value),
                dir::Type::Union(union) => {
                    pending.extend(
                        self.type_ids(target.module_id, union.elements)?
                            .iter()
                            .copied(),
                    );
                }
                dir::Type::Intersection(intersection) => {
                    pending.extend(
                        self.type_ids(target.module_id, intersection.elements)?
                            .iter()
                            .copied(),
                    );
                }
                dir::Type::Operation(_) => match self.operation_head(target)? {
                    Some(dir::TypeOperation::TemplateLiteral(_)) => {
                        let mode = InferMode::Mutable;
                        if selected.is_some_and(|selected| selected != mode) {
                            return Ok(default_mode);
                        }
                        selected = Some(mode);
                    }
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

    /// Return the literal storage type selected by one contextual target.
    pub(in crate::check) fn contextual_literal_type(
        &mut self,
        origin: Origin,
        precise: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let candidate = self.inference_candidate_type(precise, mode)?;
        if candidate == precise {
            return Ok(Answer::Ready(precise));
        }

        let accepts =
            answer!(
                self.check
                    .decide_relation(origin, Relation::Satisfies, candidate, target,)?
            );
        let selected = match accepts {
            true => candidate,
            false => precise,
        };

        Ok(Answer::Ready(selected))
    }

    /// Return the type of one scalar literal expression.
    pub(in crate::check) fn scalar_literal_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::ScalarLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::ScalarLiteral::RegexString { .. } => {
                self.language_type(node.module_id, dir::LanguageItem::RegExp, &[])
            }
            dir::ScalarLiteral::Null => self.intern_type(node.module_id, dir::Type::Null),
            dir::ScalarLiteral::Undefined => self.intern_type(node.module_id, dir::Type::Undefined),
            value => self.intern_type(node.module_id, dir::Type::Literal(value)),
        }
    }
}
