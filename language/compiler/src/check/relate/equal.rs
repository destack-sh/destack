use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CauseId, CheckState, Dependency, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide exact equality of two reduced types.
    pub(in crate::check) fn decide_equal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // error types poison silently instead of cascading
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // unit types compare by kind
            (dir::Type::Null, dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Undefined)
            | (dir::Type::Void, dir::Type::Void)
            | (dir::Type::Never, dir::Type::Never)
            | (dir::Type::Any, dir::Type::Any)
            | (dir::Type::Unknown, dir::Type::Unknown)
            | (dir::Type::This, dir::Type::This) => Answer::Ready(true),
            // unit values are the concrete value representation of void
            (dir::Type::Void, dir::Type::Tuple(tuple))
            | (dir::Type::Tuple(tuple), dir::Type::Void)
                if tuple.form == dir::TupleForm::Tuple && tuple.elements.is_empty() =>
            {
                Answer::Ready(true)
            }
            // scalar types compare structurally
            (dir::Type::Literal(source), dir::Type::Literal(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => {
                Answer::Ready(source == target)
            }
            // nullish literals equal their canonical unit types
            (dir::Type::Null, dir::Type::Literal(dir::ScalarLiteral::Null))
            | (dir::Type::Literal(dir::ScalarLiteral::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::ScalarLiteral::Undefined))
            | (dir::Type::Literal(dir::ScalarLiteral::Undefined), dir::Type::Undefined) => {
                Answer::Ready(true)
            }
            // memory singleton values compare against their authored string text
            (dir::Type::Memory(memory), dir::Type::Literal(dir::ScalarLiteral::String(text)))
            | (dir::Type::Literal(dir::ScalarLiteral::String(text)), dir::Type::Memory(memory)) => {
                Answer::Ready(text == dir::StringId::for_text(memory.text()))
            }
            // defer lifetime outlives checks to Verify
            (source, target)
                if self.is_lifetime_shaped(source) && self.is_lifetime_shaped(target) =>
            {
                Answer::Ready(true)
            }
            (dir::Type::Memory(source), dir::Type::Memory(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Static(source), dir::Type::Static(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Parameter(source), dir::Type::Parameter(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Erased(source), dir::Type::Erased(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Range(source), dir::Type::Range(target)) => Answer::Ready(source == target),
            // unions and intersections compare as unordered type sets
            (dir::Type::Union(source_union), dir::Type::Union(target_union)) => {
                let source = self
                    .type_ids(source.module_id, source_union.elements)?
                    .to_vec();
                let target = self
                    .type_ids(target.module_id, target_union.elements)?
                    .to_vec();

                self.decide_type_sets_equal(origin, &source, &target)?
            }
            (
                dir::Type::Intersection(source_intersection),
                dir::Type::Intersection(target_intersection),
            ) => {
                let source = self
                    .type_ids(source.module_id, source_intersection.elements)?
                    .to_vec();
                let target = self
                    .type_ids(target.module_id, target_intersection.elements)?
                    .to_vec();

                self.decide_type_sets_equal(origin, &source, &target)?
            }
            // memory forms compare constructor and payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.decide_form_equal(
                    origin,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if !constructor.is_ready_true() {
                    return Ok(constructor);
                }

                self.decide_relation(
                    origin,
                    Relation::Equal,
                    source_form.value,
                    target_form.value,
                )?
            }
            // structural shapes and functions
            (dir::Type::Shape(_), dir::Type::Shape(_))
            | (dir::Type::Object(_), dir::Type::Object(_)) => {
                self.decide_shape_equal(origin, source, target)?
            }
            // composites compare fixed slots beneath one shared constructor
            _ => match self.decompose_type_pair(source, target)? {
                Some(pairs) => self.decide_each(origin, Relation::Equal, &pairs)?,
                None => Answer::Ready(false),
            },
        };

        Ok(decision)
    }

    /// Return whether one type names a lifetime literal or parameter.
    fn is_lifetime_shaped(&self, ty: dir::Type) -> bool {
        match ty {
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)) => true,
            dir::Type::Parameter(parameter) => {
                self.generic_parameter(parameter).is_some_and(|binding| {
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                })
            }
            _ => false,
        }
    }

    /// Decide equality between two unordered type sets.
    fn decide_type_sets_equal(
        &mut self,
        origin: Origin,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if source.len() != target.len() {
            return Ok(Answer::Ready(false));
        }
        let mut unmatched = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(target);

        // consume one equal target for each source element
        for source in source.iter().copied() {
            let mut matched = None;
            let mut blockers = SmallVec::<[Dependency; 2]>::new();
            for (index, target) in unmatched.iter().copied().enumerate() {
                match self.decide_relation(origin, Relation::Equal, source, target)? {
                    Answer::Ready(true) => {
                        matched = Some(index);

                        break;
                    }
                    Answer::Ready(false) => {}
                    Answer::Pending(dependencies) => blockers.extend(dependencies),
                }
            }
            if let Some(index) = matched {
                unmatched.remove(index);
            } else if !blockers.is_empty() {
                return Ok(Answer::Pending(blockers));
            } else {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Constrain equality between two unordered type sets.
    pub(in crate::check) fn constrain_type_sets_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if source.len() != target.len() {
            return Ok(Answer::Ready(false));
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
                    if self.settled_root(source_type)? == self.settled_root(target_type)? {
                        matched = Some(target_index);
                    }
                } else if answer!(self.decide_relation(
                    origin,
                    Relation::Equal,
                    source_type,
                    target_type,
                )?) {
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

        // one residual pair gives an unambiguous equation
        if let ([source], [target]) = (source.as_slice(), target.as_slice()) {
            return self.constrain_type(origin, cause, Relation::Equal, *source, *target);
        }
        if source.is_empty() {
            return Ok(Answer::Ready(true));
        }

        // wait for the unresolved variables on both sides
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for ty in source.into_iter().chain(target) {
            for variable in self.type_variables(ty)? {
                let dependency = Dependency::Variable(variable);
                if !blockers.contains(&dependency) {
                    blockers.push(dependency);
                }
            }
        }
        if blockers.is_empty() {
            Ok(Answer::Ready(false))
        } else {
            Ok(Answer::Pending(blockers))
        }
    }
}
