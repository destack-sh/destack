use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, AutoInterface, AutoInterfaceObligation, CheckState, Condition, Dependency, Obligation,
    Origin, Relation, RepresentationObligation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Enforce one applied generic argument against its declared constraint.
    pub(in crate::check) fn constrain_generic_argument(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        condition: Condition,
        argument: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let constraint = answer!(self.reduce_type_root(origin, constraint)?);

        // dispatch compiler-known constraints
        match self.constraint_language_item(constraint)? {
            Some(dir::LanguageItem::Concrete) => {
                self.push_obligation(Obligation::Representation(RepresentationObligation {
                    source,
                    condition,
                    ty: argument,
                }));

                Ok(Answer::Ready(true))
            }
            Some(item) if let Some(interface) = AutoInterface::from_language_item(item) => {
                self.push_obligation(Obligation::AutoInterface(AutoInterfaceObligation {
                    source,
                    condition,
                    ty: argument,
                    interface,
                }));

                Ok(Answer::Ready(true))
            }
            _ => self.constrain(origin, Relation::Satisfies, argument, constraint),
        }
    }

    /// Return the language item named by one constraint type.
    fn constraint_language_item(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let item = match self.ty(constraint)? {
            dir::Type::Instance(instance) => self.language_item(instance.symbol)?,
            _ => None,
        };

        Ok(item)
    }

    /// Return the call signature carried by one callable value representation.
    pub(in crate::check) fn callable_signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.settled_root(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => Some(function.signature),
            dir::Type::FunctionPointer(function) => Some(function.signature),
            _ => None,
        };

        Ok(signature)
    }

    /// Enforce one relation between two types, bounding open variables.
    /// Failed closed relations report one diagnostic and count as handled.
    pub(in crate::check) fn relate(
        &mut self,
        origin: Origin,
        relation: Relation,
        value_use: Option<ValueUse>,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        if answer!(self.constrain(origin, relation, left, right)?) {
            // reject extra fields only for direct object literal flows
            if matches!(
                relation,
                Relation::Assignable | Relation::Writable | Relation::Satisfies
            ) && matches!(
                value_use,
                Some(ValueUse::Store | ValueUse::Argument | ValueUse::Output)
            ) && self
                .object_literal_excess_property(origin, left, right)?
                .is_some()
            {
                self.report_relation_failure(origin, relation, value_use, left, right)?;
            }

            return Ok(Answer::Ready(()));
        }

        self.report_relation_failure(origin, relation, value_use, left, right)?;

        Ok(Answer::Ready(()))
    }

    /// Match one relation between two types, bounding open variables.
    /// Decides closed relations without reporting, so candidates can test silently.
    pub(in crate::check) fn constrain(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // substitute solved variables before comparing
        let left = self.settled_root(left)?;
        let right = self.settled_root(right)?;
        if left == right {
            return Ok(Answer::Ready(true));
        }

        let left_variable = self.root_variable(left)?;
        let right_variable = self.root_variable(right)?;

        match (left_variable, right_variable, relation) {
            // alias open variables related by equality
            (Some(left), Some(right), Relation::Equal) => {
                self.alias_variables(left, right)?;

                Ok(Answer::Ready(true))
            }
            // bound one open side by the closed side
            (Some(variable), None, Relation::Equal) => {
                self.push_lower_bound(variable, right)?;
                self.push_upper_bound(variable, right)?;

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Equal) => {
                self.push_lower_bound(variable, left)?;
                self.push_upper_bound(variable, left)?;

                Ok(Answer::Ready(true))
            }
            // directed relations bound the open side directionally
            (Some(_), Some(variable), Relation::Assignable | Relation::Castable) => {
                self.push_lower_bound(variable, left)?;

                Ok(Answer::Ready(true))
            }
            (Some(variable), _, Relation::Assignable | Relation::Castable) => {
                self.push_upper_bound(variable, right)?;

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Assignable | Relation::Castable) => {
                self.push_lower_bound(variable, left)?;

                Ok(Answer::Ready(true))
            }
            // check-only relations wait for both sides to close
            (Some(_), _, _) | (_, Some(_), _) => {
                let mut blockers = SmallVec::<[Dependency; 2]>::new();
                blockers.extend(left_variable.map(Dependency::Variable));
                blockers.extend(right_variable.map(Dependency::Variable));

                Ok(Answer::Pending(blockers))
            }
            // decide closed relations
            (None, None, _) => {
                // reduce closed operations before comparing
                let left = answer!(self.reduce_type_root(origin, left)?);
                let right = answer!(self.reduce_type_root(origin, right)?);

                // push bounds into open leaves under matching closed
                // composites; writable flows and cast targets fill
                // leaves like value flows
                let structural = match relation {
                    Relation::Writable | Relation::Castable => Relation::Assignable,
                    relation => relation,
                };
                if matches!(structural, Relation::Equal | Relation::Assignable)
                    && let Some(answer) =
                        self.constrain_structural(origin, structural, left, right)?
                {
                    return Ok(answer);
                }

                self.decide_relation(origin, relation, left, right)
            }
        }
    }

    /// Relate matching composites child by child, bounding open leaves.
    /// Returns none when the pair is not an unambiguous matching composite.
    fn constrain_structural(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        // only composites with open leaves need constraint recursion
        if self.type_variables(left)?.is_empty() && self.type_variables(right)?.is_empty() {
            return Ok(None);
        }

        // reduce both roots so intrinsic spellings constrain
        // structurally; unreduced roots settle on the decide path
        let Some(left) = self.reduce_type_root(origin, left)?.ready() else {
            return Ok(None);
        };
        let Some(right) = self.reduce_type_root(origin, right)?.ready() else {
            return Ok(None);
        };

        // same-symbol applications constrain arguments by variance
        let same_symbol = match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Instance(left), dir::Type::Instance(right))
                if left.symbol == right.symbol && left.arguments.len() == right.arguments.len() =>
            {
                let source = left.arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                let target = right
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                Some((left.symbol, source, target))
            }
            _ => None,
        };
        if let Some((symbol, source, target)) = same_symbol {
            return Ok(Some(self.constrain_arguments_by_variance(
                origin, symbol, &source, &target,
            )?));
        }

        let left_signature = self.callable_signature(left)?;
        let right_signature = self.callable_signature(right)?;

        // collect child pairs with their child relations
        let mut pairs = SmallVec::<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
        match (self.ty(left)?.clone(), self.ty(right)?.clone()) {
            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => {
                pairs.push((Relation::Equal, source_array.element, target_array.element));
            }
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => {
                pairs.push((Relation::Equal, source_slice.element, target_slice.element));
            }
            (dir::Type::Array(source_array), dir::Type::Slice(target_slice)) => {
                pairs.push((Relation::Equal, source_array.element, target_slice.element));
            }
            (dir::Type::FixedArray(source_array), dir::Type::FixedArray(target_array)) => {
                pairs.push((relation, source_array.element, target_array.element));
                pairs.push((Relation::Equal, source_array.count, target_array.count));
            }
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple))
                if source_tuple.form == target_tuple.form
                    && source_tuple.elements.len() == target_tuple.elements.len() =>
            {
                for (source_element, target_element) in source_tuple
                    .elements
                    .iter()
                    .zip(target_tuple.elements.iter())
                {
                    pairs.push((relation, source_element.ty, target_element.ty));
                }
            }
            // shapes relate matching fields, assignability by target keys
            (dir::Type::Shape(left_shape), dir::Type::Shape(right)) => {
                let left = left_shape;

                for right_field in &right.fields {
                    let left_field = left
                        .fields
                        .iter()
                        .find(|field| field.key == right_field.key);

                    match left_field {
                        Some(left_field) => {
                            pairs.push((relation, left_field.ty, right_field.ty));
                        }
                        // missing members satisfy optional targets only
                        None => {
                            if !right_field.is_optional {
                                return Ok(Some(Answer::Ready(false)));
                            }
                        }
                    }
                }
            }
            // functions relate parameters contravariantly and returns covariantly
            (_, dir::Type::FunctionSignature(_))
                if relation == Relation::Assignable
                    && let Some(left) = left_signature =>
            {
                pairs.push((relation, left, right));
            }
            (dir::Type::FunctionSignature(_), _)
                if relation == Relation::Assignable
                    && let Some(right) = right_signature =>
            {
                pairs.push((relation, left, right));
            }
            (_, _)
                if relation == Relation::Assignable
                    && let (Some(left), Some(right)) = (left_signature, right_signature) =>
            {
                pairs.push((relation, left, right));
            }
            (dir::Type::FunctionSignature(left), dir::Type::FunctionSignature(right)) => {
                let shared = left.parameters.len().min(right.parameters.len());
                for (left, right) in left.parameters[..shared]
                    .iter()
                    .zip(right.parameters[..shared].iter())
                {
                    pairs.push((relation, right.ty, left.ty));
                }
                if let (Some(left), Some(right)) = (left.return_type, right.return_type) {
                    pairs.push((relation, left, right));
                }
            }
            // wrappers relate payloads directly, borrows bind their slots
            (dir::Type::Form(left), dir::Type::Form(right))
                if std::mem::discriminant(&left.form) == std::mem::discriminant(&right.form) =>
            {
                if let (
                    dir::Form::Borrowed {
                        lifetime: left_lifetime,
                        access: left_access,
                    },
                    dir::Form::Borrowed {
                        lifetime: right_lifetime,
                        access: right_access,
                    },
                ) = (left.form, right.form)
                {
                    pairs.push((relation, left_lifetime, right_lifetime));
                    pairs.push((relation, left_access, right_access));
                }
                if let (
                    dir::Form::Placed { place: left_place },
                    dir::Form::Placed { place: right_place },
                ) = (left.form, right.form)
                {
                    pairs.push((Relation::Equal, left_place, right_place));
                }
                pairs.push((relation, left.value, right.value));
            }
            // managed sources materialize against unqualified targets
            (dir::Type::Form(left), _)
                if left.form == dir::Form::Managed && relation == Relation::Assignable =>
            {
                pairs.push((relation, left.value, right));
            }
            // union targets accept when any member accepts
            (_, dir::Type::Union(elements)) if relation == Relation::Assignable => {
                let elements = elements
                    .elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                return Ok(Some(self.constrain_union_target(origin, left, &elements)?));
            }
            // union sources flow every element into the target
            (dir::Type::Union(elements), _) if relation == Relation::Assignable => {
                for element in &elements.elements {
                    pairs.push((relation, *element, right));
                }
            }
            (dir::Type::Dynamic(left), dir::Type::Dynamic(right)) => {
                pairs.push((relation, left.constraint, right.constraint));
            }
            // ambiguous composites wait for their open leaves instead
            _ => return Ok(Some(self.pending_on_open_leaves(left, right)?)),
        }

        // constrain every child pair through the bounding path
        let mut decision = Answer::Ready(true);
        for (relation, left, right) in pairs {
            decision = decision.and(self.constrain(origin, relation, left, right)?);
            if decision.is_ready_false() {
                return Ok(Some(decision));
            }
        }

        Ok(Some(decision))
    }

    /// Constrain one value into a union target.
    fn constrain_union_target(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        // try closed arms before binding open inference arms
        let mut open = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements.iter().copied() {
            if self.type_variables(element)?.is_empty() {
                if answer!(self.constrain(origin, Relation::Assignable, left, element)?) {
                    return Ok(Answer::Ready(true));
                }
            } else {
                open.push(element);
            }
        }

        // try open arms speculatively, keeping the first accepting arm
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in open {
            let probe = self.begin_probe();
            match self.constrain(origin, Relation::Assignable, left, element)? {
                Answer::Ready(true) => {
                    self.reject_probe(probe);

                    return Ok(self.constrain(origin, Relation::Assignable, left, element)?);
                }
                Answer::Ready(false) => self.reject_probe(probe),
                Answer::Pending(dependencies) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(dependencies));
                }
            }
        }

        Ok(Answer::ready_unless_blocked(false, blockers))
    }

    /// Park one ambiguous structural pair on its open leaves.
    ///
    /// The parked relation settles on the decide path once they close.
    fn pending_on_open_leaves(
        &self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for variable in self
            .type_variables(left)?
            .into_iter()
            .chain(self.type_variables(right)?)
        {
            if !blockers.contains(&Dependency::Variable(variable)) {
                blockers.push(Dependency::Variable(variable));
            }
        }

        Ok(Answer::pending(blockers))
    }

    /// Return the root after substituting solved inference variables.
    ///
    /// Returns the type unchanged unless its top is a solved variable, in which
    /// case it returns that variable's solution. Solutions are already settled
    /// by `set_solution`, so the loop usually takes one step. Nested variables
    /// are left intact: this settles the top, not the whole tree.
    pub(in crate::check) fn settled_root(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let Some(solution) = self.solver.solution(*variable)? else {
                return Ok(current);
            };

            current = solution;
        }

        Ok(current)
    }

    /// Return the open variable at one settled type root.
    pub(in crate::check) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.ty(id)? {
            dir::Type::Variable(variable) => Ok(Some(self.solver.representative(*variable)?)),
            _ => Ok(None),
        }
    }

    /// Return whether one accepted value flow changes runtime representation.
    pub(in crate::check) fn requires_implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if source == target {
            return Ok(false);
        }

        let source = self.representation_type(origin, source)?;
        let target = self.representation_type(origin, target)?;
        if source == target {
            return Ok(false);
        }

        // equal reduced types have the same runtime representation
        match self.decide_relation(origin, Relation::Equal, source, target)? {
            Answer::Ready(true) => return Ok(false),
            Answer::Ready(false) => {}
            Answer::Pending(blockers) => {
                let source = self.format_type(source);
                let target = self.format_type(target);

                return Err(CompilerError::Internal {
                    message: format!(
                        "coercion equality from '{source}' to '{target}' is still pending: {blockers:?}"
                    ),
                });
            }
        }

        // memory forms usually change value representation
        if matches!(
            (self.ty(source)?, self.ty(target)?),
            (dir::Type::Form(_), _) | (_, dir::Type::Form(_))
        ) {
            return Ok(true);
        }

        // intrinsic fat values need runtime construction
        if self.requires_slice_coercion(source, target)?
            || self.requires_function_coercion(source, target)?
        {
            return Ok(true);
        }

        // erased and tagged values need runtime headers
        if self.is_dynamic_type(source)?
            || self.is_dynamic_type(target)?
            || self.is_unknown_type(source)?
            || self.is_unknown_type(target)?
            || self.is_union_type(source)?
            || self.is_union_type(target)?
        {
            return Ok(true);
        }

        // scalar literals build directly in the target storage
        if self.scalar_literal_materializes_directly(source, target)? {
            return Ok(false);
        }

        // stored numeric values need conversion instructions
        let coerces = self.is_numeric_type(source)? && self.is_numeric_type(target)?;

        Ok(coerces)
    }

    /// Return the type used to classify runtime representation.
    fn representation_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut ty = match self.reduce_type_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => {
                let ty = self.format_type(ty);

                return Err(CompilerError::Internal {
                    message: format!(
                        "coercion representation type '{ty}' is still pending: {blockers:?}"
                    ),
                });
            }
        };
        while let Some(value) = match self.ty(ty)? {
            dir::Type::Form(form) if form.form == dir::Form::Readonly => Some(form.value),
            _ => None,
        } {
            ty = match self.reduce_type_root(origin, value)? {
                Answer::Ready(ty) => ty,
                Answer::Pending(blockers) => {
                    let ty = self.format_type(value);

                    return Err(CompilerError::Internal {
                        message: format!(
                            "coercion representation type '{ty}' is still pending: {blockers:?}"
                        ),
                    });
                }
            };
        }

        Ok(ty)
    }

    /// Return whether a scalar literal can build directly in one target type.
    fn scalar_literal_materializes_directly(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = self.ty(source)?;
        let target = self.ty(target)?;
        let materializes = match source {
            dir::Type::Literal(literal) => literal.widens_to(target),
            dir::Type::Range(range) => range.widens_to(target),
            _ => false,
        };

        Ok(materializes)
    }

    /// Return whether one flow builds a slice fat pointer.
    fn requires_slice_coercion(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let coerces = matches!(
            (self.ty(source)?, self.ty(target)?),
            (dir::Type::Array(_), dir::Type::Slice(_))
                | (dir::Type::FixedArray(_), dir::Type::Slice(_))
        );

        Ok(coerces)
    }

    /// Return whether one flow builds a callable fat pointer.
    fn requires_function_coercion(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let coerces = matches!(
            (self.ty(source)?, self.ty(target)?),
            (dir::Type::FunctionPointer(_), dir::Type::Function(_))
        );

        Ok(coerces)
    }

    /// Return whether a type is a dynamic runtime value.
    fn is_dynamic_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let dynamic = matches!(self.ty(ty)?, dir::Type::Dynamic(_));

        Ok(dynamic)
    }

    /// Return whether a type is an erased top value.
    fn is_unknown_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let unknown = matches!(self.ty(ty)?, dir::Type::Any | dir::Type::Unknown);

        Ok(unknown)
    }

    /// Return whether a type is a tagged union value.
    fn is_union_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let union = matches!(self.ty(ty)?, dir::Type::Union(_));

        Ok(union)
    }

    /// Return whether a type is a stored numeric scalar.
    fn is_numeric_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let numeric = matches!(
            self.ty(ty)?,
            dir::Type::Primitive(dir::PrimitiveType::Integer(_) | dir::PrimitiveType::Float(_))
                | dir::Type::Literal(dir::ScalarLiteral::Integer(_))
                | dir::Type::Literal(dir::ScalarLiteral::Float(_))
                | dir::Type::Range(_)
        );

        Ok(numeric)
    }

    /// Return the source node behind one origin for type allocation.
    pub(in crate::check) fn origin_source_node(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        match origin {
            Origin::Node(node) => Ok(node.local_id),
            Origin::Symbol(symbol) => self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id),
            Origin::Type(ty) => {
                let module = self.module(ty.module_id);

                Ok(module.types.get_type_source(ty.local_id))
            }
        }
    }
}
