use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{CauseId, CheckState, Origin, PropertySource, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Relate two reduced types under exact equality.
    pub(in crate::sema) fn relate_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // error types poison silently
        if matches!(self.ty(source)?, dir::Type::Error)
            || matches!(self.ty(target)?, dir::Type::Error)
        {
            return Ok(Verdict::Holds);
        }

        // relate region terms by the region rule
        if let Some(verdict) =
            self.relate_region_terms(origin, cause, Relation::Equal, source, target)?
        {
            return Ok(verdict);
        }

        // compare named types by what they name
        let source = self.normalize_named(origin, source)?;
        let target = self.normalize_named(origin, target)?;

        // decide equality by the heads standing on both sides
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // unit types compare by kind
            (dir::Type::Null, dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Undefined)
            | (dir::Type::Void, dir::Type::Void)
            | (dir::Type::Never, dir::Type::Never)
            | (dir::Type::Unknown, dir::Type::Unknown)
            | (dir::Type::This, dir::Type::This) => Verdict::Holds,
            // unit values are the concrete value representation of void
            (dir::Type::Void, dir::Type::Tuple(tuple))
            | (dir::Type::Tuple(tuple), dir::Type::Void)
                if tuple.form == dir::TupleForm::Tuple && tuple.elements.is_empty() =>
            {
                Verdict::Holds
            }
            // scalar types compare structurally
            (dir::Type::Literal(source), dir::Type::Literal(target)) => {
                Verdict::decided(source == target)
            }
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => {
                Verdict::decided(source == target)
            }
            // nullish literals equal their canonical unit types
            (dir::Type::Null, dir::Type::Literal(dir::Literal::Null))
            | (dir::Type::Literal(dir::Literal::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::Literal::Undefined))
            | (dir::Type::Literal(dir::Literal::Undefined), dir::Type::Undefined) => Verdict::Holds,
            (dir::Type::Static(source), dir::Type::Static(target)) => {
                Verdict::decided(source == target)
            }
            (dir::Type::Parameter(source), dir::Type::Parameter(target)) => {
                Verdict::decided(source == target)
            }
            (dir::Type::Erased(source), dir::Type::Erased(target)) => {
                Verdict::decided(source == target)
            }
            (dir::Type::Range(source), dir::Type::Range(target)) => {
                Verdict::decided(source == target)
            }
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
                if constructor == Verdict::Fails {
                    return Ok(Verdict::Fails);
                }

                let payload = self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_form.value,
                    target_form.value,
                )?;

                constructor.and(payload)
            }
            // compare anonymous classes by shape
            (dir::Type::Object(_), dir::Type::Object(_)) => self.relate_shape(
                origin,
                cause,
                Relation::Equal,
                PropertySource::Stored,
                source,
                target,
            )?,
            // equate an owned form over a value type with the value, an open payload deciding later
            (dir::Type::Form(form), _) | (_, dir::Type::Form(form))
                if form.form == dir::Form::Owned =>
            {
                let other = match self.ty(source)? {
                    dir::Type::Form(_) => target,
                    _ => source,
                };
                match self.default_ownership(origin, form.value)? {
                    Some(dir::Ownership::Owned) => {
                        self.constrain_type(origin, cause, Relation::Equal, form.value, other)?
                    }
                    Some(_) => Verdict::Fails,
                    None if self.root_variable(form.value)?.is_some() => Verdict::Ambiguous,
                    None => Verdict::Fails,
                }
            }
            // compare the fixed children of composites under one shared constructor
            _ => match self.decompose_type_pair(source, target)? {
                Some(pairs) => self.relate_each(origin, cause, Relation::Equal, &pairs)?,
                None => Verdict::Fails,
            },
        };

        Ok(decision)
    }

    /// Relate equality between two unordered type sets.
    pub(in crate::sema) fn relate_type_sets_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        if source.len() != target.len() {
            return Ok(Verdict::Fails);
        }

        // work on removable copies of both sides
        let mut source = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(source);
        let mut target = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(target);

        // remove established equal elements before constraining open elements
        let mut source_index = 0;
        while source_index < source.len() {
            let source_type = source[source_index];
            let is_source_open = !self.type_variables(source_type)?.is_empty();
            let mut matched = None;

            // search the target side for an element that already equals this one
            for (target_index, target_type) in target.iter().copied().enumerate() {
                let is_target_open = !self.type_variables(target_type)?.is_empty();
                if is_source_open || is_target_open {
                    if self.shallow_resolve(source_type)? == self.shallow_resolve(target_type)? {
                        matched = Some(target_index);
                    }
                } else if self
                    .decide_relation(origin, Relation::Equal, source_type, target_type)?
                    .holds()
                {
                    matched = Some(target_index);
                }
                if matched.is_some() {
                    break;
                }
            }

            // drop the matched pair, otherwise carry this element forward
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

        // hold once every element found its partner
        if source.is_empty() {
            return Ok(Verdict::Holds);
        }

        // reject open variables reaching set equality
        let open = self.collect_open_variables(source.into_iter().chain(target))?;
        if !open.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("open variables {open:?} reached set equality"),
            });
        }

        Ok(Verdict::Fails)
    }
}
