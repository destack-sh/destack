use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{CauseId, CheckState, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Relate two reduced types under exact equality.
    pub(in crate::check) fn relate_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // error and hole types poison silently instead of cascading
            (dir::Type::Error | dir::Type::Hole(_), _)
            | (_, dir::Type::Error | dir::Type::Hole(_)) => true,
            // compare lifetime pairs equal, MIR Verify enforces outlives
            (_, _)
                if self.is_lifetime_slot_type(source)? && self.is_lifetime_slot_type(target)? =>
            {
                true
            }
            // unit types compare by kind
            (dir::Type::Null, dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Undefined)
            | (dir::Type::Void, dir::Type::Void)
            | (dir::Type::Never, dir::Type::Never)
            | (dir::Type::Any, dir::Type::Any)
            | (dir::Type::Unknown, dir::Type::Unknown)
            | (dir::Type::This, dir::Type::This) => true,
            // unit values are the concrete value representation of void
            (dir::Type::Void, dir::Type::Tuple(tuple))
            | (dir::Type::Tuple(tuple), dir::Type::Void)
                if tuple.form == dir::TupleForm::Tuple && tuple.elements.is_empty() =>
            {
                true
            }
            // scalar types compare structurally
            (dir::Type::Literal(source), dir::Type::Literal(target)) => source == target,
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => source == target,
            // nullish literals equal their canonical unit types
            (dir::Type::Null, dir::Type::Literal(dir::ScalarLiteral::Null))
            | (dir::Type::Literal(dir::ScalarLiteral::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::ScalarLiteral::Undefined))
            | (dir::Type::Literal(dir::ScalarLiteral::Undefined), dir::Type::Undefined) => true,
            // memory singleton values compare against their authored string text
            (dir::Type::Memory(memory), dir::Type::Literal(dir::ScalarLiteral::String(text)))
            | (dir::Type::Literal(dir::ScalarLiteral::String(text)), dir::Type::Memory(memory)) => {
                text == dir::StringId::for_text(memory.text())
            }
            (dir::Type::Memory(source), dir::Type::Memory(target)) => source == target,
            (dir::Type::Static(source), dir::Type::Static(target)) => source == target,
            (dir::Type::Parameter(source), dir::Type::Parameter(target)) => source == target,
            (dir::Type::Erased(source), dir::Type::Erased(target)) => source == target,
            (dir::Type::Range(source), dir::Type::Range(target)) => source == target,
            // unions and intersections compare as unordered type sets
            (dir::Type::Union(source_union), dir::Type::Union(target_union)) => {
                let source: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, source_union.elements)?
                    .into();
                let target: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, target_union.elements)?
                    .into();

                self.relate_type_sets_equal(origin, cause, &source, &target)?
            }
            (
                dir::Type::Intersection(source_intersection),
                dir::Type::Intersection(target_intersection),
            ) => {
                let source: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, source_intersection.elements)?
                    .into();
                let target: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, target_intersection.elements)?
                    .into();

                self.relate_type_sets_equal(origin, cause, &source, &target)?
            }
            // memory forms compare constructor and payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.relate_form_equal(
                    origin,
                    cause,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if !constructor {
                    return Ok(constructor);
                }

                self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_form.value,
                    target_form.value,
                )?
            }
            // structural shapes and functions
            (dir::Type::Shape(_), dir::Type::Shape(_))
            | (dir::Type::Object(_), dir::Type::Object(_)) => {
                self.relate_shape_equal(origin, cause, source, target)?
            }
            // composites compare fixed slots beneath one shared constructor
            _ => match self.decompose_type_pair(source, target)? {
                Some(pairs) => self.relate_each(origin, cause, Relation::Equal, &pairs)?,
                None => false,
            },
        };

        Ok(decision)
    }

    /// Relate equality between two unordered type sets.
    pub(in crate::check) fn relate_type_sets_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        if source.len() != target.len() {
            return Ok(false);
        }
        let mut source = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(source);
        let mut target = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(target);

        // remove established equal elements before constraining open elements
        let mut source_index = 0;
        while source_index < source.len() {
            let source_type = source[source_index];
            let is_source_open = !self.type_variables(source_type)?.is_empty();
            let mut matched = None;
            for (target_index, target_type) in target.iter().copied().enumerate() {
                let is_target_open = !self.type_variables(target_type)?.is_empty();
                if is_source_open || is_target_open {
                    if self.shallow_resolve(source_type)? == self.shallow_resolve(target_type)? {
                        matched = Some(target_index);
                    }
                } else if self.evaluate_relation(
                    origin,
                    Relation::Equal,
                    source_type,
                    target_type,
                )? {
                    matched = Some(target_index);
                }
                if matched.is_some() {
                    break;
                }
            }
            if let Some(target_index) = matched {
                source.remove(source_index);
                target.remove(target_index);
            } else {
                source_index += 1;
            }
        }

        // equate one residual pair, which is unambiguous
        if let ([source], [target]) = (source.as_slice(), target.as_slice()) {
            return self.constrain_type(origin, cause, Relation::Equal, *source, *target);
        }
        if source.is_empty() {
            return Ok(true);
        }

        // unresolved variables on either side leave the equation ambiguous
        let open = self.open_type_variables(source.into_iter().chain(target))?;
        if !open.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("open variables {open:?} reached set equality"),
            });
        }

        Ok(false)
    }
}
