use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, AutoInterface, AutoInterfaceObligation, CheckState, ConstraintState, ConstraintSubject,
    Dependency, Obligation, Origin, Relation, RepresentationObligation, ValueUse, answer,
};

impl CheckState<'_> {
    /// Enforce one applied generic argument against its declared constraint.
    pub(in crate::check) fn constrain_generic_bound(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let constraint = answer!(self.reduce_type_head(origin, constraint)?);

        // check conjunction bounds element-wise, each on its own path
        if let dir::Type::Intersection(intersection) = self.ty(constraint)? {
            let elements = self
                .type_ids(constraint.module_id, intersection.elements)?
                .to_vec();
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision =
                    decision.and(self.constrain_generic_bound(origin, source, argument, element)?);
                if decision.is_ready_false() {
                    break;
                }
            }

            return Ok(decision);
        }

        // normalize compiler-known static domains before checking bounds
        let item = self
            .type_symbol(constraint)?
            .map(|symbol| self.language_item(symbol))
            .transpose()?
            .flatten();

        match item {
            Some(
                item @ (dir::LanguageItem::Access
                | dir::LanguageItem::Lifetime
                | dir::LanguageItem::Place
                | dir::LanguageItem::Space),
            ) => {
                let argument = self.normalize_memory_domain_value(origin, argument, item)?;

                self.constrain(origin, Relation::Satisfies, argument, constraint)
            }
            Some(dir::LanguageItem::Concrete) => {
                let scope = self.origin_scope(origin);
                self.push_obligation(
                    Obligation::Representation(RepresentationObligation {
                        source,
                        ty: argument,
                    }),
                    scope,
                );

                Ok(Answer::Ready(true))
            }
            Some(item) if let Some(interface) = AutoInterface::from_language_item(item) => {
                let scope = self.origin_scope(origin);
                self.push_obligation(
                    Obligation::AutoInterface(AutoInterfaceObligation {
                        source,
                        ty: argument,
                        interface,
                    }),
                    scope,
                );

                Ok(Answer::Ready(true))
            }
            _ => self.constrain(origin, Relation::Satisfies, argument, constraint),
        }
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
        match self.apply_relation(origin, relation, value_use, left, right)? {
            Answer::Ready(_) => Ok(Answer::Ready(())),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Apply one required relation and report failed closed checks.
    pub(in crate::check) fn apply_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        value_use: Option<ValueUse>,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ConstraintState>> {
        let holds = answer!(self.constrain(origin, relation, left, right)?);

        // reject extra fields only for direct property literal flows
        if holds
            && matches!(
                relation,
                Relation::Assignable | Relation::Writable | Relation::Satisfies
            )
            && self
                .property_literal_excess_property(origin, left, right)?
                .is_some()
        {
            self.report_relation_failure(origin, relation, value_use, left, right)?;

            return Ok(Answer::Ready(ConstraintState::Fails));
        }

        // report the ordinary failed relation
        if !holds {
            self.report_relation_failure(origin, relation, value_use, left, right)?;

            return Ok(Answer::Ready(ConstraintState::Fails));
        }

        Ok(Answer::Ready(ConstraintState::Holds))
    }

    /// Apply one type constraint and report failed closed checks.
    pub(in crate::check) fn apply_type_constraint(
        &mut self,
        origin: Origin,
        relation: Relation,
        subject: Option<ConstraintSubject>,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ConstraintState>> {
        let Some(subject) = subject else {
            return self.apply_relation(origin, relation, None, left, right);
        };

        let holds = match subject {
            ConstraintSubject::GenericArgument { source } => {
                answer!(self.constrain_generic_bound(origin, source, left, right)?)
            }
        };
        if !holds {
            let origin = match subject {
                ConstraintSubject::GenericArgument { source } => Origin::Node(source, None),
            };
            self.report_relation_failure(origin, relation, None, left, right)?;

            return Ok(Answer::Ready(ConstraintState::Fails));
        }

        Ok(Answer::Ready(ConstraintState::Holds))
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
        let bound_source = self
            .origin_source_node(origin)?
            .into_global(origin.module());

        match (left_variable, right_variable, relation) {
            // alias open variables related by equality
            (Some(left), Some(right), Relation::Equal) => {
                self.alias_variables(left, right)?;

                Ok(Answer::Ready(true))
            }
            // bound one open side by the closed side
            (Some(variable), None, Relation::Equal) => {
                self.push_lower_bound(variable, bound_source, right)?;
                self.push_upper_bound(variable, bound_source, right)?;

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Equal) => {
                self.push_lower_bound(variable, bound_source, left)?;
                self.push_upper_bound(variable, bound_source, left)?;

                Ok(Answer::Ready(true))
            }
            // directed relations bound the open side directionally
            (Some(_), Some(variable), Relation::Assignable | Relation::Castable) => {
                self.push_lower_bound(variable, bound_source, left)?;

                Ok(Answer::Ready(true))
            }
            (Some(variable), _, Relation::Assignable | Relation::Castable) => {
                self.push_upper_bound(variable, bound_source, right)?;

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Assignable | Relation::Castable) => {
                self.push_lower_bound(variable, bound_source, left)?;

                Ok(Answer::Ready(true))
            }
            // check-only relations wait for both sides to close
            (Some(_), _, _) | (_, Some(_), _) => {
                let mut blockers = SmallVec::<[Dependency; 2]>::new();
                blockers.extend(left_variable.map(Dependency::Variable));
                blockers.extend(right_variable.map(Dependency::Variable));

                Ok(Answer::Pending(blockers))
            }
            // decompose open composites before full graph reduction
            (None, None, _) => {
                let structural = match relation {
                    Relation::Writable | Relation::Castable => Relation::Assignable,
                    relation => relation,
                };

                // reduce aliases and intrinsics at the root only
                let left = answer!(self.reduce_type_head(origin, left)?);
                let right = answer!(self.reduce_type_head(origin, right)?);

                // push bounds into known composites that still contain holes
                if !self.type_variables(left)?.is_empty() || !self.type_variables(right)?.is_empty()
                {
                    return match structural {
                        Relation::Equal | Relation::Assignable => {
                            match self.constrain_structural(origin, structural, left, right)? {
                                Some(answer) => Ok(answer),
                                None => Ok(self.pending_on_open_leaves(left, right)?),
                            }
                        }
                        _ => Ok(self.pending_on_open_leaves(left, right)?),
                    };
                }

                // reduce closed operands before comparing relation truth
                let left = answer!(self.reduce_type(origin, left)?);
                let right = answer!(self.reduce_type(origin, right)?);

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

        // same-symbol applications constrain arguments by variance
        let same_symbol = match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Instance(left_instance), dir::Type::Instance(right_instance))
                if left_instance.symbol == right_instance.symbol
                    && left_instance.arguments.len() == right_instance.arguments.len() =>
            {
                let source = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(left.module_id, left_instance.arguments)?,
                );
                let target = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(right.module_id, right_instance.arguments)?,
                );

                Some((left_instance.symbol, source, target))
            }
            _ => None,
        };
        if let Some((symbol, source, target)) = same_symbol {
            return Ok(Some(
                self.relate_type_arguments(origin, symbol, &source, &target)?,
            ));
        }

        let left_signature = self.callable_signature(left)?;
        let right_signature = self.callable_signature(right)?;

        // collect child pairs with their child relations
        let mut pairs = SmallVec::<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
        match (self.ty(left)?, self.ty(right)?) {
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
                let source_elements = self.tuple_elements(left.module_id, source_tuple.elements)?;
                let target_elements =
                    self.tuple_elements(right.module_id, target_tuple.elements)?;
                for (source_element, target_element) in
                    source_elements.iter().zip(target_elements.iter())
                {
                    pairs.push((relation, source_element.ty, target_element.ty));
                }
            }
            // shapes relate matching fields, assignability by target keys
            (dir::Type::Shape(left_shape), dir::Type::Shape(right_shape)) => {
                let left_fields = self.shape_fields(left.module_id, left_shape.fields)?;
                let right_fields = self.shape_fields(right.module_id, right_shape.fields)?;

                for right_field in right_fields {
                    let left_field = left_fields
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
            (
                dir::Type::FunctionSignature(left_function),
                dir::Type::FunctionSignature(right_function),
            ) => {
                let left_parameters =
                    self.signature_parameters(left.module_id, left_function.parameters)?;
                let right_parameters =
                    self.signature_parameters(right.module_id, right_function.parameters)?;
                let shared = left_parameters.len().min(right_parameters.len());
                for (left_parameter, right_parameter) in left_parameters[..shared]
                    .iter()
                    .zip(right_parameters[..shared].iter())
                {
                    pairs.push((relation, right_parameter.ty, left_parameter.ty));
                }
                if let (Some(left_return), Some(right_return)) =
                    (left_function.return_type, right_function.return_type)
                {
                    pairs.push((relation, left_return, right_return));
                }
            }
            // memory forms relate payloads directly, borrows bind their slots
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
                let elements = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(right.module_id, elements.elements)?,
                );

                return Ok(Some(self.constrain_union_target(origin, left, &elements)?));
            }
            // union sources flow every element into the target
            (dir::Type::Union(elements), _) if relation == Relation::Assignable => {
                let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                    self.type_ids(left.module_id, elements.elements)?,
                );
                for element in elements {
                    pairs.push((relation, element, right));
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
                    self.commit_probe(probe);

                    return Ok(Answer::Ready(true));
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
            let Some(solution) = self.solver.solution(variable)? else {
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
            dir::Type::Variable(variable) => Ok(Some(self.solver.representative(variable)?)),
            _ => Ok(None),
        }
    }

    /// Return the source node behind one origin for type allocation.
    pub(in crate::check) fn origin_source(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let module = origin.module();

        Ok(self.origin_source_node(origin)?.into_global(module))
    }

    /// Return the local source node anchoring one work origin.
    pub(in crate::check) fn origin_source_node(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        match origin {
            Origin::Node(node, _) => Ok(node.local_id),
            Origin::Symbol(symbol) => self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id),
        }
    }
}
