use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Mutation, Origin, Relation};

impl CheckState<'_> {
    /// Decide one relation between two closed type roots.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // reduce both roots before structural comparison
        let left = match self.evaluate_root(origin, left)? {
            Answer::Ready(left) => left,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let right = match self.evaluate_root(origin, right)? {
            Answer::Ready(right) => right,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        if left == right {
            return Ok(Answer::Ready(true));
        }

        // reuse memoized answers, assuming in-flight pairs hold so recursive types terminate
        if let Some(holds) = self.relations.lookup(relation, left, right) {
            return Ok(Answer::Ready(holds));
        }
        let frame = self.relations.enter(relation, left, right);

        let decision = match relation {
            Relation::Equal => self.decide_equal(origin, left, right),
            Relation::Assignable => self.decide_assignable(origin, left, right),
            Relation::Writable => self.decide_writable(origin, left, right),
            Relation::Castable => self.decide_castable(origin, left, right),
            Relation::Satisfies | Relation::Extends | Relation::Implements => {
                self.decide_satisfies(origin, relation, left, right)
            }
        };

        // memoize settled decisions, forget pending or failed attempts
        match &decision {
            Ok(Answer::Ready(holds)) => {
                let settled = self.relations.finish(frame, *holds);
                for (relation, left, right) in settled {
                    self.journal.record(Mutation::RelationDecided {
                        relation,
                        left,
                        right,
                    });
                }
            }
            _ => self.relations.cancel(frame),
        }

        decision
    }

    /// Decide whether one value may be written into one place type.
    fn decide_writable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // shape literals construct structs member-wise
        if let (dir::Type::Shape(_), dir::Type::Reference(instance)) =
            (self.ty(source)?, self.ty(target)?)
        {
            let instance = instance.clone();
            let is_struct = matches!(
                self.definition(instance.symbol),
                Some(dir::Definition::Struct(_))
            );
            if is_struct {
                return self.decide_struct_construction(origin, source, target, &instance);
            }
        }

        self.decide_assignable(origin, source, target)
    }

    /// Return whether one fixed array count equals one literal length.
    pub(in crate::check) fn fixed_count_equals(
        &mut self,
        count: dir::GlobalTypeId,
        length: usize,
    ) -> CompilerResult<bool> {
        let count = self.resolve_root(count)?;
        let dir::Type::Literal(dir::ScalarLiteral::Integer(count)) = self.ty(count)? else {
            return Ok(false);
        };

        Ok(i64::try_from(length).is_ok_and(|length| *count == length))
    }

    /// Decide explicit castability.
    fn decide_castable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // lossless numeric widening requires explicit cast
        if let (dir::Type::Primitive(source), dir::Type::Primitive(target)) =
            (self.ty(source)?, self.ty(target)?)
            && source.widens_to(*target)
        {
            return Ok(Answer::Ready(true));
        }

        let forward = self.decide_assignable(origin, source, target)?;
        if forward == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        let backward = self.decide_assignable(origin, target, source)?;
        if backward == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        Ok(forward.or(backward))
    }

    /// Decide exact equality of two reduced types.
    pub(in crate::check) fn decide_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (self.ty(left)?, self.ty(right)?) {
            // error types poison silently instead of cascading
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // unit atoms compare by kind
            (dir::Type::Null, dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Undefined)
            | (dir::Type::Void, dir::Type::Void)
            | (dir::Type::Never, dir::Type::Never)
            | (dir::Type::Any, dir::Type::Any)
            | (dir::Type::Unknown, dir::Type::Unknown)
            | (dir::Type::Object, dir::Type::Object)
            | (dir::Type::This, dir::Type::This) => Answer::Ready(true),
            // atoms compare structurally
            (dir::Type::Literal(left), dir::Type::Literal(right)) => Answer::Ready(left == right),
            // nullish literals equal their canonical type types
            (dir::Type::Null, dir::Type::Literal(dir::ScalarLiteral::Null))
            | (dir::Type::Literal(dir::ScalarLiteral::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::ScalarLiteral::Undefined))
            | (dir::Type::Literal(dir::ScalarLiteral::Undefined), dir::Type::Undefined) => {
                Answer::Ready(true)
            }
            (dir::Type::Primitive(left), dir::Type::Primitive(right)) => {
                Answer::Ready(left == right)
            }
            // lifetime validity defers to Verify, like assignability
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (dir::Type::Memory(left), dir::Type::Memory(right)) => Answer::Ready(left == right),
            (dir::Type::Static(left), dir::Type::Static(right)) => Answer::Ready(left == right),
            (dir::Type::Parameter(left), dir::Type::Parameter(right)) => {
                Answer::Ready(left == right)
            }
            (dir::Type::Range(left), dir::Type::Range(right)) => Answer::Ready(left == right),

            // same-symbol references compare argument-wise
            (dir::Type::Reference(left), dir::Type::Reference(right)) => {
                if left.symbol != right.symbol || left.arguments.len() != right.arguments.len() {
                    Answer::Ready(false)
                } else {
                    let pairs = left
                        .arguments
                        .iter()
                        .copied()
                        .zip(right.arguments.iter().copied())
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }

            // memory forms compare constructor and payload
            (dir::Type::Form(left), dir::Type::Form(right)) => {
                let (left_form, right_form) = (left.form, right.form);
                let (left_value, right_value) = (left.value, right.value);
                let constructor = self.decide_form_equal(origin, left_form, right_form)?;
                if constructor != Answer::Ready(true) {
                    return Ok(constructor);
                }

                self.decide_relation(origin, Relation::Equal, left_value, right_value)?
            }

            // collections compare element-wise
            (dir::Type::Array(left), dir::Type::Array(right)) => {
                let (left, right) = (left.element, right.element);

                self.decide_relation(origin, Relation::Equal, left, right)?
            }
            (dir::Type::Slice(left), dir::Type::Slice(right)) => {
                let (left, right) = (left.element, right.element);

                self.decide_relation(origin, Relation::Equal, left, right)?
            }
            (dir::Type::FixedArray(left), dir::Type::FixedArray(right)) => {
                let (left_element, right_element) = (left.element, right.element);
                let (left_count, right_count) = (left.count, right.count);
                let element =
                    self.decide_relation(origin, Relation::Equal, left_element, right_element)?;
                let count =
                    self.decide_relation(origin, Relation::Equal, left_count, right_count)?;

                element.and(count)
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_equal(origin, left, right)?
            }

            // structural shapes and functions
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_equal(origin, left, right)?
            }
            (dir::Type::Function(_), dir::Type::Function(_)) => {
                self.decide_function_equal(origin, left, right)?
            }
            (dir::Type::Closure(left), dir::Type::Closure(right)) => {
                let (left_function, right_function) = (left.function, right.function);
                let (left_environment, right_environment) = (left.environment, right.environment);
                let function =
                    self.decide_relation(origin, Relation::Equal, left_function, right_function)?;
                let environment = self.decide_relation(
                    origin,
                    Relation::Equal,
                    left_environment,
                    right_environment,
                )?;

                function.and(environment)
            }

            // algebraic composites compare element-wise in order
            (dir::Type::Union(left), dir::Type::Union(right)) => {
                if left.elements.len() != right.elements.len() {
                    Answer::Ready(false)
                } else {
                    let pairs = left
                        .elements
                        .iter()
                        .copied()
                        .zip(right.elements.iter().copied())
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }
            (dir::Type::Intersection(left), dir::Type::Intersection(right)) => {
                if left.elements.len() != right.elements.len() {
                    Answer::Ready(false)
                } else {
                    let pairs = left
                        .elements
                        .iter()
                        .copied()
                        .zip(right.elements.iter().copied())
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }

            // dynamic wrappers compare constraints
            (dir::Type::Dynamic(left), dir::Type::Dynamic(right)) => {
                let (left, right) = (left.constraint, right.constraint);

                self.decide_relation(origin, Relation::Equal, left, right)?
            }

            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide assignability from one reduced source to one reduced target.
    pub(in crate::check) fn decide_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        let decision = match (self.ty(source)?, self.ty(target)?) {
            // top and error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // check never judges lifetime validity: relations collect
            // lifetimes into signatures, and Verify decides outlives
            // on MIR where flow anchors live
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => Answer::Ready(true),
            (dir::Type::Any, _) => Answer::Ready(true),
            (dir::Type::Never, _) => Answer::Ready(true),

            // readonly views relate their payload elements covariantly
            (dir::Type::Form(source), dir::Type::Form(target))
                if source.form == dir::Form::Readonly && target.form == dir::Form::Readonly =>
            {
                let (source_value, target_value) = (source.value, target.value);

                self.decide_readonly_assignable(origin, source_value, target_value)?
            }
            // memory forms check constructor then payload
            (dir::Type::Form(source), dir::Type::Form(target)) => {
                let (source_form, target_form) = (source.form, target.form);
                let (source_value, target_value) = (source.value, target.value);
                let constructor = self.decide_form_assignable(origin, source_form, target_form)?;
                if constructor != Answer::Ready(true) {
                    return Ok(constructor);
                }

                self.decide_relation(origin, Relation::Assignable, source_value, target_value)?
            }
            // managed values materialize against unqualified targets
            (dir::Type::Form(source), _) if source.form == dir::Form::Managed => {
                let value = source.value;

                self.decide_relation(origin, Relation::Assignable, value, target)?
            }

            // union sources need every element assignable
            (dir::Type::Union(union), _) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.decide_all(origin, &elements, target)?
            }
            // union targets need one viable element
            (_, dir::Type::Union(union)) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.decide_any(origin, source, &elements)?
            }

            // literals fit primitives by value
            (dir::Type::Literal(literal), dir::Type::Primitive(primitive)) => {
                Answer::Ready(literal.fits_primitive(*primitive))
            }
            (dir::Type::Literal(literal), dir::Type::Range(range)) => {
                Answer::Ready(literal.fits_range(range))
            }
            (dir::Type::Range(range), dir::Type::Primitive(primitive)) => {
                Answer::Ready(range.fits_primitive(*primitive))
            }
            (dir::Type::Range(source), dir::Type::Range(target)) => {
                Answer::Ready(target.contains(source))
            }

            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            // fresh array literals of the exact length fill fixed arrays
            (dir::Type::Array(array), dir::Type::FixedArray(fixed)) => {
                let (source_element, target_element) = (array.element, fixed.element);
                let count = fixed.count;
                match self.fresh_array_literal_length(source)? {
                    Some(length) if self.fixed_count_equals(count, length)? => self
                        .decide_relation(
                            origin,
                            Relation::Assignable,
                            source_element,
                            target_element,
                        )?,
                    _ => Answer::Ready(false),
                }
            }
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let (source_element, target_element) = (source.element, target.element);
                let (source_count, target_count) = (source.count, target.count);
                let element = self.decide_relation(
                    origin,
                    Relation::Assignable,
                    source_element,
                    target_element,
                )?;
                let count =
                    self.decide_relation(origin, Relation::Equal, source_count, target_count)?;

                element.and(count)
            }
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Assignable, source, target)?
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, source, target)?
            }

            // structural shapes and nominal boundaries
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_assignable(origin, source, target)?
            }
            (dir::Type::Shape(_), dir::Type::Reference(reference)) => {
                let reference = reference.clone();

                self.decide_source_against_reference(origin, source, &reference)?
            }
            (dir::Type::Reference(reference), dir::Type::Shape(_)) => {
                let reference = reference.clone();

                self.decide_reference_against_target(origin, &reference, target)?
            }
            (dir::Type::Reference(source_instance), dir::Type::Reference(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                // relate argument pairs by their parameter variances
                let symbol = source_instance.symbol;
                let source_arguments = source_instance
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                let target_arguments = target_instance
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_arguments_by_variance(
                    origin,
                    symbol,
                    &source_arguments,
                    &target_arguments,
                )?
            }
            (dir::Type::Reference(_), dir::Type::Reference(_)) => {
                self.decide_nominal_assignable(origin, source, target)?
            }

            // parameters assign through their constraints
            (dir::Type::Parameter(parameter), _) => {
                let parameter = *parameter;

                self.decide_parameter_assignable(origin, parameter, target)?
            }

            // functions assign by signature variance
            (dir::Type::Function(_), dir::Type::Function(_)) => {
                self.decide_function_assignable(origin, source, target)?
            }

            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide assignability under one deep readonly view.
    ///
    /// Aliasing containers relax to covariant element relations because
    ///  no mutation can flow back through the view.
    fn decide_readonly_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // readable collections relate elements covariantly
        let elements = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                Some((source.element, target.element))
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                Some((source.element, target.element))
            }
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                Some((source.element, target.element))
            }
            _ => None,
        };

        match elements {
            Some((source, target)) => self.decide_readonly_assignable(origin, source, target),
            None => self.decide_assignable(origin, source, target),
        }
    }

    /// Decide whether one parameter's constraint carries the assignment.
    fn decide_parameter_assignable(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(Answer::Ready(false));
        };
        let Some(constraint) = binding.constraint else {
            return Ok(Answer::Ready(false));
        };

        self.decide_relation(origin, Relation::Assignable, constraint, target)
    }

    /// Decide every relation in one pair list.
    pub(in crate::check) fn decide_each(
        &mut self,
        origin: Origin,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (left, right) in pairs.iter().copied() {
            match self.decide_relation(origin, relation, left, right)? {
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Ready(true) => {}
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Decide whether every union element assigns to one target.
    fn decide_all(
        &mut self,
        origin: Origin,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for source in sources {
            match self.decide_relation(origin, Relation::Assignable, *source, target)? {
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Ready(true) => {}
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Decide whether one source assigns to any union element.
    fn decide_any(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for target in targets {
            match self.decide_relation(origin, Relation::Assignable, source, *target)? {
                Answer::Ready(true) => return Ok(Answer::Ready(true)),
                Answer::Ready(false) => {}
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(false))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Decide equality of two memory form constructors.
    fn decide_form_equal(
        &mut self,
        origin: Origin,
        left: dir::Form,
        right: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        match (left, right) {
            (
                dir::Form::Borrowed {
                    lifetime: left_lifetime,
                    access: left_access,
                },
                dir::Form::Borrowed {
                    lifetime: right_lifetime,
                    access: right_access,
                },
            ) => {
                // relations collect lifetimes without judging them:
                // open annotation slots gather the flowing component,
                // and Verify decides outlives on MIR
                self.collect_lifetime(left_lifetime, right_lifetime)?;
                self.collect_lifetime(right_lifetime, left_lifetime)?;

                self.decide_relation(origin, Relation::Equal, left_access, right_access)
            }
            (dir::Form::Placed { place: left }, dir::Form::Placed { place: right }) => {
                self.decide_relation(origin, Relation::Equal, left, right)
            }
            _ => Ok(Answer::Ready(
                std::mem::discriminant(&left) == std::mem::discriminant(&right),
            )),
        }
    }

    /// Decide assignability of two memory form constructors.
    fn decide_form_assignable(
        &mut self,
        origin: Origin,
        source: dir::Form,
        target: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        // exact constructors assign, readonly accepts any borrowed view
        match (source, target) {
            (dir::Form::Borrowed { .. }, dir::Form::Readonly) => Ok(Answer::Ready(true)),
            // borrows widen their provenance and downgrade their access
            (
                dir::Form::Borrowed {
                    lifetime: source_lifetime,
                    access: source_access,
                },
                dir::Form::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                // relations collect lifetimes without judging them:
                // open annotation slots gather the flowing component,
                // and Verify decides outlives on MIR
                self.collect_lifetime(source_lifetime, target_lifetime)?;

                self.decide_access_assignable(origin, source_access, target_access)
            }
            _ => self.decide_form_equal(origin, source, target),
        }
    }

    /// Collect one flowing lifetime into an open lifetime slot.
    /// Lifetime validity is never judged here; open annotation slots
    /// solve from the components that flow through them.
    fn collect_lifetime(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let target = self.resolve_root(target)?;
        if let Some(variable) = self.root_variable(target)? {
            self.push_lower_bound(variable, source)?;
        }

        Ok(())
    }

    /// Decide whether one borrow access satisfies a demanded access.
    ///
    /// Stronger access downgrades: exclusive covers mutable and readonly,
    ///  and mutable covers readonly.
    fn decide_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = match self.evaluate_root(origin, source)? {
            Answer::Ready(source) => source,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let target = match self.evaluate_root(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        match (self.ty(source)?, self.ty(target)?) {
            (
                dir::Type::Memory(dir::MemoryLiteral::Access(source)),
                dir::Type::Memory(dir::MemoryLiteral::Access(target)),
            ) => {
                let downgrades = match (source, target) {
                    (left, right) if left == right => true,
                    (dir::Access::Exclusive, _) => true,
                    (dir::Access::Mutable, dir::Access::Readonly) => true,
                    _ => false,
                };

                Ok(Answer::Ready(downgrades))
            }
            // symbolic accesses compare exactly
            _ => self.decide_relation(origin, Relation::Equal, source, target),
        }
    }
}
