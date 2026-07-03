use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, AutoInterface, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide one relation between two closed type roots, growing the stack.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        destack_core::ensure_sufficient_stack(|| {
            self.decide_relation_inner(origin, relation, left, right)
        })
    }

    /// Decide one relation on the grown stack.
    fn decide_relation_inner(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // reduce both operands before structural comparison
        let left = answer!(self.reduce_type(origin, left)?);
        let right = answer!(self.reduce_type(origin, right)?);
        if left == right {
            return Ok(Answer::Ready(true));
        }
        if let (Some(left_key), Some(right_key)) = (
            self.static_key_from_type(left)?,
            self.static_key_from_type(right)?,
        ) {
            return Ok(Answer::Ready(left_key == right_key));
        }

        // key parameter and this queries by their assuming scope
        let flags = self.type_flags(left)? | self.type_flags(right)?;
        let scope = match flags.has_parameter() || flags.has_this() {
            true => self.origin_scope(origin),
            false => None,
        };

        // reuse memoized answers, treating in-flight pairs as recursive cycles
        if let Some(holds) = self.relations().lookup(relation, left, right, scope) {
            return Ok(Answer::Ready(holds));
        }
        let frame = self.relations().enter(relation, left, right, scope);

        let decision = match relation {
            Relation::Equal => self.decide_equal(origin, left, right),
            Relation::Assignable => self.decide_assignable(origin, left, right),
            Relation::MethodAssignable => self.decide_method_assignable(origin, left, right),
            Relation::Writable => self.decide_writable(origin, left, right),
            Relation::Castable => self.decide_castable(origin, left, right),
            Relation::Satisfies | Relation::Extends | Relation::Implements => {
                self.decide_satisfies(origin, relation, left, right)
            }
        };

        // memoize settled decisions, forget pending or failed attempts
        match &decision {
            Ok(Answer::Ready(holds)) => {
                self.relations().finish(frame, *holds);
            }
            _ => self.relations().cancel(frame),
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
        if let (dir::Type::Shape(_), dir::Type::Instance(instance)) =
            (self.ty(source)?, self.ty(target)?)
        {
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
            && source.widens_to(target)
        {
            return Ok(Answer::Ready(true));
        }

        // concrete newtypes project explicitly to their backing type
        if let dir::Type::Instance(instance) = self.ty(source)?
            && let Some(backing) = self.newtype_backing_type(origin, source.module_id, &instance)?
        {
            let projected = self.decide_relation(origin, Relation::Castable, backing, target)?;
            if !matches!(projected, Answer::Ready(false)) {
                return Ok(projected);
            }
        }

        let forward = self.decide_assignable(origin, source, target)?;
        if forward.is_ready_true() {
            return Ok(Answer::Ready(true));
        }

        let backward = self.decide_assignable(origin, target, source)?;
        if backward.is_ready_true() {
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
            // unit values are the concrete value representation of void
            (dir::Type::Void, dir::Type::Tuple(tuple))
            | (dir::Type::Tuple(tuple), dir::Type::Void)
                if tuple.form == dir::TupleForm::Tuple && tuple.elements.is_empty() =>
            {
                Answer::Ready(true)
            }
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
            // memory singleton values compare against their authored string spelling
            (dir::Type::Memory(memory), dir::Type::Literal(dir::ScalarLiteral::String(text)))
            | (dir::Type::Literal(dir::ScalarLiteral::String(text)), dir::Type::Memory(memory)) => {
                Answer::Ready(text == dir::StringId::for_text(memory.text()))
            }
            // defer lifetime outlives checks to Verify
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (dir::Type::Memory(left), dir::Type::Memory(right)) => Answer::Ready(left == right),
            (dir::Type::Static(left), dir::Type::Static(right)) => Answer::Ready(left == right),
            (dir::Type::Parameter(left), dir::Type::Parameter(right)) => {
                Answer::Ready(left == right)
            }
            (dir::Type::EnumMember(left), dir::Type::EnumMember(right)) => {
                if left.member != right.member {
                    Answer::Ready(false)
                } else {
                    self.decide_relation(origin, Relation::Equal, left.owner, right.owner)?
                }
            }
            (dir::Type::Range(left), dir::Type::Range(right)) => Answer::Ready(left == right),
            (dir::Type::Operation(left_operation), dir::Type::Operation(right_operation)) => self
                .decide_operation_equal(
                origin,
                left.module_id,
                &left_operation,
                right.module_id,
                &right_operation,
            )?,

            // same-symbol references compare argument-wise
            (dir::Type::Instance(left_instance), dir::Type::Instance(right_instance)) => {
                if left_instance.symbol != right_instance.symbol
                    || left_instance.arguments.len() != right_instance.arguments.len()
                {
                    Answer::Ready(false)
                } else {
                    let pairs = self
                        .type_ids(left.module_id, left_instance.arguments)?
                        .iter()
                        .copied()
                        .zip(
                            self.type_ids(right.module_id, right_instance.arguments)?
                                .iter()
                                .copied(),
                        )
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }

            // memory forms compare constructor and payload
            (dir::Type::Form(left), dir::Type::Form(right)) => {
                let (left_form, right_form) = (left.form, right.form);
                let (left_value, right_value) = (left.value, right.value);
                let constructor = self.decide_form_equal(origin, left_form, right_form)?;
                if !constructor.is_ready_true() {
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
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.decide_function_equal(origin, left, right)?
            }
            (dir::Type::Function(left), dir::Type::Function(right)) => {
                let (left_function, right_function) = (left.signature, right.signature);
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
            (dir::Type::FunctionPointer(left), dir::Type::FunctionPointer(right)) => {
                self.decide_relation(origin, Relation::Equal, left.signature, right.signature)?
            }

            // algebraic composites compare element-wise in order
            (dir::Type::Union(left_union), dir::Type::Union(right_union)) => {
                if left_union.elements.len() != right_union.elements.len() {
                    Answer::Ready(false)
                } else {
                    let pairs = self
                        .type_ids(left.module_id, left_union.elements)?
                        .iter()
                        .copied()
                        .zip(
                            self.type_ids(right.module_id, right_union.elements)?
                                .iter()
                                .copied(),
                        )
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }
            (
                dir::Type::Intersection(left_intersection),
                dir::Type::Intersection(right_intersection),
            ) => {
                if left_intersection.elements.len() != right_intersection.elements.len() {
                    Answer::Ready(false)
                } else {
                    let pairs = self
                        .type_ids(left.module_id, left_intersection.elements)?
                        .iter()
                        .copied()
                        .zip(
                            self.type_ids(right.module_id, right_intersection.elements)?
                                .iter()
                                .copied(),
                        )
                        .collect::<SmallVec<[_; 4]>>();

                    self.decide_each(origin, Relation::Equal, &pairs)?
                }
            }

            // dynamic types compare constraints
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
        if self.decide_equal(origin, source, target)?.is_ready_true() {
            return Ok(Answer::Ready(true));
        }
        if answer!(self.widens_to(origin, source, target)?) {
            return Ok(Answer::Ready(true));
        }

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // top and error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // collect lifetime components, Verify checks outlives on MIR
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => Answer::Ready(true),
            (dir::Type::Any, _) => Answer::Ready(true),
            (dir::Type::Never, _) => Answer::Ready(true),

            // exact property keys flow into their primitive key domains
            (_, dir::Type::Primitive(primitive))
                if self
                    .static_key_from_type(source)?
                    .is_some_and(|key| primitive_accepts_key(primitive, key)) =>
            {
                Answer::Ready(true)
            }

            // readonly forms relate their payload elements covariantly
            (dir::Type::Form(source), dir::Type::Form(target))
                if source.form == dir::Form::Readonly && target.form == dir::Form::Readonly =>
            {
                let (source_value, target_value) = (source.value, target.value);

                self.decide_readonly_assignable(origin, source_value, target_value)?
            }
            // values can flow into readonly forms by dropping write access
            (_, dir::Type::Form(target)) if target.form == dir::Form::Readonly => {
                let target_value = target.value;

                self.decide_readonly_assignable(origin, source, target_value)?
            }
            // memory forms check constructor then payload
            (dir::Type::Form(source), dir::Type::Form(target)) => {
                let (source_form, target_form) = (source.form, target.form);
                let (source_value, target_value) = (source.value, target.value);
                let constructor = self.decide_form_assignable(origin, source_form, target_form)?;
                if !constructor.is_ready_true() {
                    return Ok(constructor);
                }

                self.decide_relation(origin, Relation::Assignable, source_value, target_value)?
            }
            // values can materialize into a concrete storage place
            (_, dir::Type::Form(target)) if matches!(target.form, dir::Form::Placed { .. }) => {
                let target_value = target.value;

                self.decide_relation(origin, Relation::Assignable, source, target_value)?
            }
            // reading from a concrete storage place yields its payload value
            (dir::Type::Form(source), _) if matches!(source.form, dir::Form::Placed { .. }) => {
                let source_value = source.value;

                self.decide_relation(origin, Relation::Assignable, source_value, target)?
            }
            // managed values materialize against unqualified targets
            (dir::Type::Form(source), _) if source.form == dir::Form::Managed => {
                let value = source.value;

                self.decide_relation(origin, Relation::Assignable, value, target)?
            }

            // union sources need every element assignable
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

                self.decide_all_assignable(origin, &elements, target)?
            }
            // parameters assign through their constraints before target decomposition
            (dir::Type::Parameter(parameter), _) => {
                self.decide_parameter_assignable(origin, parameter, target)?
            }
            // this assigns through its enclosing interface hypotheses
            (dir::Type::This, _) => self.decide_this_assignable(origin, target)?,
            // intersection sources assign through any element
            (dir::Type::Intersection(intersection), _) => {
                let elements = self
                    .type_ids(source.module_id, intersection.elements)?
                    .to_vec();
                let mut decision = Answer::Ready(false);
                for element in elements {
                    decision = decision.or(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        element,
                        target,
                    )?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                decision
            }
            // union targets need one viable element
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_assignable(origin, source, &elements)?
            }
            // intersection targets need every element
            (_, dir::Type::Intersection(intersection)) => {
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();
                let mut decision = Answer::Ready(true);
                for element in elements {
                    decision = decision.and(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        source,
                        element,
                    )?);
                    if decision.is_ready_false() {
                        break;
                    }
                }

                decision
            }
            // erase compatible values into dynamic targets
            (_, dir::Type::Dynamic(dynamic)) => {
                let constraint = dynamic.constraint;

                self.decide_dynamic_assignable(origin, source, constraint)?
            }

            // literals and intervals widen by value
            (dir::Type::Literal(literal), target) => Answer::Ready(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Answer::Ready(range.widens_to(&target)),
            (dir::Type::EnumMember(member), _) => {
                self.decide_relation(origin, Relation::Assignable, member.owner, target)?
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
            (dir::Type::Array(_), dir::Type::FixedArray(_)) => Answer::Ready(false),
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
            (dir::Type::Reference(_), dir::Type::Shape(_)) => {
                self.decide_reference_shape_assignable(origin, source, target)?
            }
            (dir::Type::Shape(_), dir::Type::Instance(reference)) => {
                self.decide_source_against_reference(origin, source, target.module_id, &reference)?
            }
            (dir::Type::Instance(reference), dir::Type::Shape(_)) => {
                self.decide_reference_against_target(origin, source.module_id, &reference, target)?
            }
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                // relate argument pairs by their parameter variances
                let symbol = source_instance.symbol;
                let source_arguments = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(source.module_id, source_instance.arguments)?,
                );
                let target_arguments = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(target.module_id, target_instance.arguments)?,
                );

                self.decide_type_arguments(origin, symbol, &source_arguments, &target_arguments)?
            }
            (dir::Type::Instance(_), dir::Type::Instance(_)) => {
                self.decide_nominal_assignable(origin, source, target)?
            }

            // functions assign by signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.decide_function_assignable(origin, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.decide_function_assignable(origin, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.decide_function_assignable(origin, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.decide_function_assignable(origin, source, target)?
            }

            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one source value can erase into `Dynamic<constraint>`.
    fn decide_dynamic_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => source,
        };

        if !answer!(self.satisfies_auto_interface(origin, source, AutoInterface::DynamicSafe,)?) {
            return Ok(Answer::Ready(false));
        }

        self.decide_relation(origin, Relation::Assignable, source, constraint)
    }

    /// Decide assignability under one deep readonly form.
    ///
    /// Aliasing containers relax to covariant element relations because
    ///  no mutation can flow back through the form.
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
        // prove through any declared or assumed bound
        let mut decision = Answer::Ready(false);
        for bound in self.parameter_bounds(origin, parameter)? {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, bound, target)?);
            if decision.is_ready_true() {
                break;
            }
        }

        Ok(decision)
    }

    /// Decide whether one assumed bound carries `this`.
    fn decide_this_assignable(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // prove through any assumed bound on this
        let mut decision = Answer::Ready(false);
        for bound in self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This))? {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, bound, target)?);
            if decision.is_ready_true() {
                break;
            }
        }

        Ok(decision)
    }

    /// Decide equality of two type-level operations.
    /// `left_module`/`right_module` are the owners of each operation's list payloads.
    fn decide_operation_equal(
        &mut self,
        origin: Origin,
        left_module: destack_source::ModuleId,
        left: &dir::TypeOperation,
        right_module: destack_source::ModuleId,
        right: &dir::TypeOperation,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (left, right) {
            // string mappings compare operation and mapped target
            (
                dir::TypeOperation::StringMapping {
                    mapping: left_mapping,
                    target: left_target,
                },
                dir::TypeOperation::StringMapping {
                    mapping: right_mapping,
                    target: right_target,
                },
            ) if left_mapping == right_mapping => {
                self.decide_relation(origin, Relation::Equal, *left_target, *right_target)?
            }
            (
                dir::TypeOperation::StringMapping { .. },
                dir::TypeOperation::StringMapping { .. },
            ) => Answer::Ready(false),

            // conditional types compare their operands and branches structurally
            (dir::TypeOperation::Conditional(left), dir::TypeOperation::Conditional(right)) => {
                let fields = [
                    (left.left, right.left),
                    (left.right, right.right),
                    (left.then_type, right.then_type),
                    (left.else_type, right.else_type),
                ];
                let fields = self.decide_each(origin, Relation::Equal, &fields)?;

                fields.and(Answer::Ready(left.is_distributive == right.is_distributive))
            }

            // narrowed types compare source, target, and branch polarity
            (dir::TypeOperation::Narrow(left), dir::TypeOperation::Narrow(right)) => {
                let fields = [(left.source, right.source), (left.target, right.target)];
                let fields = self.decide_each(origin, Relation::Equal, &fields)?;

                fields.and(Answer::Ready(left.is_positive == right.is_positive))
            }

            // mapped types compare the binder identity and mapped payload
            (dir::TypeOperation::Mapped(left), dir::TypeOperation::Mapped(right)) => {
                let parameters = left.parameter.name == right.parameter.name
                    && left.parameter.parameter == right.parameter.parameter
                    && left.modifiers == right.modifiers;
                let parameters = Answer::Ready(parameters);
                let constraint = self.decide_relation(
                    origin,
                    Relation::Equal,
                    left.parameter.constraint,
                    right.parameter.constraint,
                )?;
                let key_remap = self.decide_optional_equal(
                    origin,
                    left.parameter.key_remap,
                    right.parameter.key_remap,
                )?;
                let value =
                    self.decide_relation(origin, Relation::Equal, left.value, right.value)?;

                parameters.and(constraint).and(key_remap).and(value)
            }

            // indexed access compares owner and index through the relation system
            (dir::TypeOperation::Index(left), dir::TypeOperation::Index(right)) => {
                let fields = [(left.left, right.left), (left.index, right.index)];

                self.decide_each(origin, Relation::Equal, &fields)?
            }

            // type queries compare their referenced source path
            (dir::TypeOperation::TypeOf(left), dir::TypeOperation::TypeOf(right)) => {
                Answer::Ready(left.value == right.value)
            }

            // template literals compare strings directly and spans structurally
            (
                dir::TypeOperation::TemplateLiteral(left),
                dir::TypeOperation::TemplateLiteral(right),
            ) if self.template_strings(left_module, left.strings)?
                == self.template_strings(right_module, right.strings)?
                && left.spans.len() == right.spans.len() =>
            {
                let fields = self
                    .type_ids(left_module, left.spans)?
                    .iter()
                    .copied()
                    .zip(self.type_ids(right_module, right.spans)?.iter().copied())
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_each(origin, Relation::Equal, &fields)?
            }
            (dir::TypeOperation::TemplateLiteral(_), dir::TypeOperation::TemplateLiteral(_)) => {
                Answer::Ready(false)
            }

            // infer binders compare name and optional constraint
            (dir::TypeOperation::Infer(left), dir::TypeOperation::Infer(right))
                if left.name == right.name =>
            {
                self.decide_optional_equal(origin, left.constraint, right.constraint)?
            }
            (dir::TypeOperation::Infer(_), dir::TypeOperation::Infer(_)) => Answer::Ready(false),

            // unary operations compare their targets
            (dir::TypeOperation::KeyOf(left), dir::TypeOperation::KeyOf(right))
            | (dir::TypeOperation::NoInfer(left), dir::TypeOperation::NoInfer(right))
            | (dir::TypeOperation::Awaited(left), dir::TypeOperation::Awaited(right)) => {
                self.decide_relation(origin, Relation::Equal, left.target, right.target)?
            }

            // try projections compare their projected value
            (
                dir::TypeOperation::TryOutput { value: left },
                dir::TypeOperation::TryOutput { value: right },
            )
            | (
                dir::TypeOperation::TryResidual { value: left },
                dir::TypeOperation::TryResidual { value: right },
            ) => self.decide_relation(origin, Relation::Equal, *left, *right)?,

            // static binary operations compare operator and operands
            (dir::TypeOperation::StaticBinary(left), dir::TypeOperation::StaticBinary(right))
                if left.operator == right.operator =>
            {
                let fields = [(left.left, right.left), (left.right, right.right)];

                self.decide_each(origin, Relation::Equal, &fields)?
            }
            (dir::TypeOperation::StaticBinary(_), dir::TypeOperation::StaticBinary(_)) => {
                Answer::Ready(false)
            }

            // static unary operations compare operator and operand
            (dir::TypeOperation::StaticUnary(left), dir::TypeOperation::StaticUnary(right))
                if left.operator == right.operator =>
            {
                self.decide_relation(origin, Relation::Equal, left.target, right.target)?
            }
            (dir::TypeOperation::StaticUnary(_), dir::TypeOperation::StaticUnary(_)) => {
                Answer::Ready(false)
            }

            // different operation constructors are not equal
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide equality of optional type operands.
    fn decide_optional_equal(
        &mut self,
        origin: Origin,
        left: Option<dir::GlobalTypeId>,
        right: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        match (left, right) {
            (Some(left), Some(right)) => self.decide_relation(origin, Relation::Equal, left, right),
            (None, None) => Ok(Answer::Ready(true)),
            _ => Ok(Answer::Ready(false)),
        }
    }

    /// Decide every relation in one pair list.
    pub(in crate::check) fn decide_each(
        &mut self,
        origin: Origin,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (left, right) in pairs.iter().copied() {
            decision = decision.and(self.decide_relation(origin, relation, left, right)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether every union element assigns to one target.
    fn decide_all_assignable(
        &mut self,
        origin: Origin,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for source in sources {
            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                *source,
                target,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source assigns to any union element.
    fn decide_any_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for target in targets {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, source, *target)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
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
                // collect open lifetime annotation bounds
                self.push_lifetime_lower_bound_if_open(origin, left_lifetime, right_lifetime)?;
                self.push_lifetime_lower_bound_if_open(origin, right_lifetime, left_lifetime)?;

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
        // exact constructors assign, readonly accepts any borrowed form
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
                // collect open lifetime annotation bounds on either side
                self.push_lifetime_lower_bound_if_open(origin, source_lifetime, target_lifetime)?;
                self.push_lifetime_lower_bound_if_open(origin, target_lifetime, source_lifetime)?;

                self.decide_access_assignable(origin, source_access, target_access)
            }
            _ => self.decide_form_equal(origin, source, target),
        }
    }

    /// Push one flowing lifetime into an open lifetime slot.
    ///
    /// Open annotation slots solve from the components that flow through them.
    fn push_lifetime_lower_bound_if_open(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let target = self.settled_root(target)?;
        if let Some(variable) = self.root_variable(target)? {
            let bound_source = self
                .origin_source_node(origin)?
                .into_global(origin.module());
            self.push_lower_bound(variable, bound_source, source)?;
        }

        Ok(())
    }

    /// Decide whether one borrow access satisfies a required access.
    ///
    /// Stronger access downgrades: exclusive covers mutable and readonly,
    ///  and mutable covers readonly.
    fn decide_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let target = answer!(self.reduce_type_head(origin, target)?);

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

/// Return whether one primitive key domain accepts one exact property key.
fn primitive_accepts_key(primitive: dir::PrimitiveType, key: dir::StaticKey) -> bool {
    match primitive {
        dir::PrimitiveType::String => key.is_string_like(),
        dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => key.is_symbol_like(),
        dir::PrimitiveType::Integer(_) => key.is_number_like(),
        dir::PrimitiveType::Boolean
        | dir::PrimitiveType::Character
        | dir::PrimitiveType::Float(_)
        | dir::PrimitiveType::Bigint => false,
    }
}
