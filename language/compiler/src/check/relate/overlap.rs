use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Return whether two types can share at least one runtime inhabitant.
    pub(in crate::check) fn types_may_overlap(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let left = answer!(self.reduce_type_head(origin, left)?);
        let right = answer!(self.reduce_type_head(origin, right)?);

        let left_type = self.ty(left)?.clone();
        let right_type = self.ty(right)?.clone();

        // reject empty inhabitants
        if matches!(left_type, dir::Type::Never) || matches!(right_type, dir::Type::Never) {
            return Ok(Answer::Ready(false));
        }

        if left == right {
            return Ok(Answer::Ready(true));
        }

        // split unions on either side
        if let dir::Type::Union(union) = left_type {
            return self.any_type_arm_may_overlap(origin, union.elements, right);
        }
        if let dir::Type::Union(union) = right_type {
            return self.any_type_arm_may_overlap(origin, union.elements, left);
        }

        // compare generic parameters through their declared bounds
        if let dir::Type::Parameter(parameter) = left_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, constraint, right);
        }
        if let dir::Type::Parameter(parameter) = right_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, left, constraint);
        }

        // keep other open types conservative
        if Self::is_open_inhabitant_type(&left_type) || Self::is_open_inhabitant_type(&right_type) {
            return Ok(Answer::Ready(true));
        }

        // unwrap memory forms around their runtime payload
        if let dir::Type::Form(form) = left_type {
            return self.types_may_overlap(origin, form.value, right);
        }
        if let dir::Type::Form(form) = right_type {
            return self.types_may_overlap(origin, left, form.value);
        }

        // compare exact scalar inhabitant shapes
        if let Some(overlaps) = Self::scalar_types_may_overlap(&left_type, &right_type) {
            return Ok(Answer::Ready(overlaps));
        }

        // reject disjoint scalar domains
        let left_domain = left_type.scalar_domain();
        let right_domain = right_type.scalar_domain();
        if let (Some(left), Some(right)) = (left_domain, right_domain) {
            return Ok(Answer::Ready(left == right));
        }
        if left_domain.is_some() || right_domain.is_some() {
            return Ok(Answer::Ready(false));
        }

        // reject distinct concrete runtime identities
        if let (dir::Type::Instance(left), dir::Type::Instance(right)) = (&left_type, &right_type) {
            return self.generic_instances_may_overlap(origin, left, right);
        }

        Ok(Answer::Ready(true))
    }

    /// Return whether any union arm can overlap one target type.
    fn any_type_arm_may_overlap(
        &mut self,
        origin: Origin,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        for element in elements {
            if answer!(self.types_may_overlap(origin, element, target)?) {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return whether one type is too open to disprove inhabitant overlap.
    fn is_open_inhabitant_type(ty: &dir::Type) -> bool {
        matches!(
            ty,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Variable(_)
                | dir::Type::Dynamic(_)
                | dir::Type::Error
        )
    }

    /// Return exact overlap for scalar singleton and interval types.
    fn scalar_types_may_overlap(left: &dir::Type, right: &dir::Type) -> Option<bool> {
        let overlaps = match (left, right) {
            (dir::Type::Literal(left), dir::Type::Literal(right)) => left == right,
            (dir::Type::Literal(left), dir::Type::Range(right)) => right.contains_literal(*left),
            (dir::Type::Range(left), dir::Type::Literal(right)) => left.contains_literal(*right),
            (dir::Type::Range(left), dir::Type::Range(right)) => left.overlaps_range(right),
            _ => return None,
        };

        Some(overlaps)
    }

    /// Return whether two generic instances can name the same runtime identity.
    fn generic_instances_may_overlap(
        &mut self,
        origin: Origin,
        left: &dir::GenericInstance,
        right: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if left.symbol == right.symbol {
            return self.generic_arguments_may_overlap(origin, &left.arguments, &right.arguments);
        }

        let left_kind = self.symbol_kind(left.symbol);
        let right_kind = self.symbol_kind(right.symbol);
        let left_is_concrete = matches!(
            left_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );
        let right_is_concrete = matches!(
            right_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );

        Ok(Answer::Ready(!(left_is_concrete && right_is_concrete)))
    }

    /// Return whether two argument lists can describe one shared generic instance.
    fn generic_arguments_may_overlap(
        &mut self,
        origin: Origin,
        left: &[dir::GlobalTypeId],
        right: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if left.len() != right.len() {
            return Ok(Answer::Ready(true));
        }

        for (left, right) in left.iter().copied().zip(right.iter().copied()) {
            if !answer!(self.types_may_overlap(origin, left, right)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }
}
