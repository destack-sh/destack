use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{BodyState, Origin, Relation, Value, ValueUse, VariableKind, VariableRole};

/// The context one expression is inferred in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum InferMode {
    /// Aggregate slots widen the fresh literals they store.
    Regular,
    /// An `as const` context keeps literals and makes its aggregates readonly.
    Const,
}

impl InferMode {
    /// Return whether aggregates inferred in this mode are readonly.
    pub(in crate::sema) fn is_readonly(self) -> bool {
        self == Self::Const
    }
}

impl BodyState<'_, '_> {
    /// Return the numeric family one fresh value's literals belong to.
    fn fresh_numeric_kind(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<VariableKind>> {
        // read the arms the value carries
        let ty = self.shallow_resolve(ty)?;
        let arms = match self.ty(ty)? {
            dir::Type::Union(union) => self.type_ids(ty.module_id, union.elements)?.to_vec(),
            _ => vec![ty],
        };

        // join the numeric family across every arm
        let mut kind = None;
        for arm in arms {
            let arm = self.shallow_resolve(arm)?;
            let arm_kind = match self.ty(arm)? {
                dir::Type::Literal(dir::Literal::Integer(_)) => VariableKind::Integer,
                dir::Type::Literal(dir::Literal::Float(_)) => VariableKind::Float,
                _ => return Ok(None),
            };
            kind = Some(kind.map_or(arm_kind, |kind: VariableKind| kind.join(arm_kind)));
        }

        Ok(kind)
    }

    /// Commit the widening one fresh literal node takes to its stored type.
    fn commit_widening(&mut self, value: Value, target: dir::GlobalTypeId) -> CompilerResult<()> {
        if let Some(node) = value.node
            && target != value.ty
        {
            let coercion = dir::Coercion::new(
                value.ty,
                vec![dir::CoercionAdjustment::Widen { target }],
                dir::CastOrigin::Implicit,
            );
            self.check.commit_coercion(node, coercion)?;
        }

        Ok(())
    }

    /// Widen one fresh value stored in a mutable slot.
    pub(in crate::sema) fn widen_fresh(
        &mut self,
        value: Value,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !value.is_fresh {
            return Ok(value.ty);
        }

        // a numeric family widens to its fallback, every other literal to its base type
        let widened = match self
            .fresh_numeric_kind(value.ty)?
            .and_then(VariableKind::fallback)
        {
            Some(fallback) => self.intern_type(fallback)?,
            None => self.widen_type(value.ty)?,
        };
        self.commit_widening(value, widened)?;

        Ok(widened)
    }

    /// Widen one fresh value into its binding slot.
    pub(in crate::sema) fn widen_fresh_slot(
        &mut self,
        slot: dir::TypeVariableId,
        value: Value,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !value.is_fresh {
            return Ok(value.ty);
        }

        // a value outside the numeric families widens to its base type
        let Some(kind) = self.fresh_numeric_kind(value.ty)? else {
            let widened = self.widen_type(value.ty)?;
            self.commit_widening(value, widened)?;

            return Ok(widened);
        };

        // join the slot's own family and take its type
        self.join_variable_kind(slot, kind)?;
        let ty = self.variable_type(slot)?;
        self.commit_widening(value, ty)?;

        Ok(ty)
    }

    /// Open one numeric variable for a fresh value meeting an inference destination.
    pub(in crate::sema) fn fresh_variable(
        &mut self,
        origin: Origin,
        value: Value,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !value.is_fresh {
            return Ok(value.ty);
        }

        // one literal node opens one variable, however often its conversion reruns
        if let Some(Some(widened)) = value
            .node
            .and_then(|node| self.check.fresh_nodes.get(&node))
        {
            return Ok(*widened);
        }

        // a numeric family opens a variable, every other literal widens to its base type
        let widened = match self.fresh_numeric_kind(value.ty)? {
            Some(kind) => {
                let variable = self.open_variable_of(origin, kind, VariableRole::Regular);
                self.variable_type(variable)?
            }
            None => self.widen_type(value.ty)?,
        };

        // remember the node's variable for the reruns that follow
        if let Some(node) = value.node {
            self.check.fresh_nodes.insert(node, Some(widened));
        }

        Ok(widened)
    }

    /// Return the open variable one fresh literal's destination settles into.
    ///
    /// The variable sits beneath the destination's memory forms, standing alone or as the sole
    /// arm of its literal family in a union.
    fn destination_variable(
        &mut self,
        value: Value,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        // read beneath the destination's memory forms
        let mut target = self.shallow_resolve(target)?;
        while let dir::Type::Form(form) = self.ty(target)? {
            let payload = self.shallow_resolve(form.value)?;
            if payload == target {
                break;
            }
            target = payload;
        }

        // take the variable itself, or the first variable arm of a union
        let destination = match self.ty(target)? {
            dir::Type::Union(union) => {
                let literal = self.shallow_resolve(value.ty)?;
                let family = match self.ty(literal)? {
                    dir::Type::Literal(literal) => literal.scalar_domain(),
                    _ => None,
                };
                let arms = self.type_ids(target.module_id, union.elements)?.to_vec();
                let mut destination = None;
                for arm in arms {
                    let arm = self.shallow_resolve(arm)?;
                    match self.ty(arm)? {
                        dir::Type::Literal(arm) if arm.scalar_domain() == family => {
                            return Ok(None);
                        }
                        dir::Type::Variable(variable) if destination.is_none() => {
                            destination = Some(variable);
                        }
                        _ => {}
                    }
                }
                let Some(destination) = destination else {
                    return Ok(None);
                };
                destination
            }
            dir::Type::Variable(variable) => variable,
            _ => return Ok(None),
        };

        Ok(self
            .open_root(destination)?
            .is_some()
            .then_some(destination))
    }

    /// Return the candidate one fresh literal contributes to its destination variable: the
    /// literal itself where the destination keeps literals, else the variable the literal opens.
    pub(in crate::sema) fn literal_candidate(
        &mut self,
        origin: Origin,
        value: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Value> {
        if !value.is_fresh || use_ == ValueUse::Const {
            return Ok(value);
        }

        // read the destination variable one fresh literal settles into
        let Some(destination) = self.destination_variable(value, target)? else {
            return Ok(value);
        };

        let keeps_literals = match self.infer.variable_role(destination)? {
            VariableRole::Instantiation { parameter } => {
                let binding = self.require_generic_parameter(parameter)?.clone();
                let mut keeps = self.parameter_keeps_literals(origin, parameter)?;
                let template =
                    dir::GlobalGenericTemplateId::new(parameter.module_id, binding.template);
                let owner = self
                    .generic_template(template)
                    .and_then(|template| template.symbol);

                // a string or boolean literal keeps its type at a signature's top-level return
                if use_ != ValueUse::Store
                    && self.fresh_numeric_kind(value.ty)?.is_none()
                    && let Some(owner) = owner
                    && let Some(ty) = self.adopt_symbol_type_maybe(owner)?
                    && let Some(head) = self.signature_head(ty)?
                    && let Some(return_type) = head.return_type
                {
                    keeps |= self.has_exposed_type(return_type, binding.ty)?;
                }
                keeps
            }
            _ => false,
        };
        if keeps_literals {
            return Ok(Value {
                is_fresh: false,
                ..value
            });
        }
        let ty = self.fresh_variable(origin, value)?;

        Ok(Value {
            ty,
            is_fresh: false,
            ..value
        })
    }

    /// Return the type one aggregate slot stores for a checked value.
    pub(in crate::sema) fn slot_storage(
        &mut self,
        origin: Origin,
        relation: Relation,
        slot: dir::GlobalTypeId,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep the precise value under a check-only relation or into an open slot
        if relation == Relation::Satisfies {
            return Ok(source);
        }
        let slot = self.deeply_resolve(origin, slot)?;
        if self.type_flags(slot)?.has_variable() {
            return Ok(source);
        }

        Ok(slot)
    }

    /// Return the type of one literal expression.
    pub(in crate::sema) fn literal_type(
        &mut self,
        value: dir::Literal,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::Literal::RegexString { .. } => self.language_type(dir::LanguageItem::RegExp, &[]),
            dir::Literal::Null => self.intern_type(dir::Type::Null),
            dir::Literal::Undefined => self.intern_type(dir::Type::Undefined),
            value => self.intern_type(dir::Type::Literal(value)),
        }
    }
}
