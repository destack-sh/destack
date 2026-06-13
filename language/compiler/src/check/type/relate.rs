use destack_artifact::DiagnosticAnchor;
use destack_dir as dir;
use smallvec::SmallVec;

use super::widen::ScalarKind;
use crate::CompilerResult;
use crate::check::{
    Answer, CheckError, CheckState, Condition, Constraint, ConstraintCause, Dependency, Mutation,
    Origin, Relation,
};

impl CheckState<'_> {
    /// Enforce one relation between two types, bounding open variables.
    /// Failed closed relations report one diagnostic and count as handled.
    pub(in crate::check) fn relate(
        &mut self,
        origin: Origin,
        relation: Relation,
        cause: ConstraintCause,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        match self.constrain(origin, relation, left, right)? {
            Answer::Ready(true) => {
                self.record_implicit_coercion(origin, relation, cause, left, right)?;

                Ok(Answer::Ready(()))
            }
            Answer::Ready(false) => {
                self.report_relation_failure(origin, relation, cause, left, right)?;

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Record one implicit representation change at a value node.
    /// Accepted value flows whose representation differs from their
    /// target materialize as implicit casts beside the checked tables.
    /// Flows into still-open variables defer through one check-only
    /// constraint that parks until the solution names the target.
    fn record_implicit_coercion(
        &mut self,
        origin: Origin,
        relation: Relation,
        cause: ConstraintCause,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // only value flows coerce
        if !matches!(relation, Relation::Assignable | Relation::Writable) {
            return Ok(());
        }
        let Origin::Node(node) = origin else {
            return Ok(());
        };
        if !self.modules.contains_key(&node.module_id) {
            return Ok(());
        }

        let source = self.resolve_root(left)?;
        let target = self.resolve_root(right)?;
        if self.root_variable(source)?.is_some() {
            return Ok(());
        }

        // closed scalars flowing into open variables settle their
        // representation when the variable solves; the check-only
        // relation re-enters here with the solution
        if self.root_variable(target)?.is_some() {
            if relation == Relation::Assignable && ScalarKind::of(self.ty(source)?).is_some() {
                self.push_constraint(Constraint {
                    relation: Relation::Writable,
                    left: source,
                    right: target,
                    origin,
                    condition: Condition::Always,
                    cause,
                });
            }

            return Ok(());
        }

        self.record_coercion(node, source, target)
    }

    /// Record one implicit coercion when two closed types differ in
    /// representation.
    pub(in crate::check) fn record_coercion(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if !self.coerces_representation(source, target)? {
            return Ok(());
        }

        let coercion = dir::Coercion::new(source, target, dir::CastOrigin::Implicit);
        let previous = self.coercions.insert(node, coercion);
        self.journal
            .record(Mutation::CoercionSet { node, previous });

        Ok(())
    }

    /// Return whether one accepted flow changes value representation.
    pub(in crate::check) fn coerces_representation(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // erasing into a dynamic container boxes the value
        if matches!(self.ty(target)?, dir::Type::Dynamic(_))
            && !matches!(self.ty(source)?, dir::Type::Dynamic(_))
        {
            return Ok(true);
        }

        // scalar values materialize at their target kind
        let (Some(source), Some(target)) = (
            ScalarKind::of(self.ty(source)?),
            ScalarKind::of(self.ty(target)?),
        ) else {
            return Ok(false);
        };

        Ok(source != target)
    }

    /// Match one relation between two types, bounding open variables.
    /// Decides closed relations without reporting, so candidate probes
    /// can test and discard hypotheses silently.
    pub(in crate::check) fn constrain(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // chase variable roots through aliases and solutions
        let left = self.resolve_root(left)?;
        let right = self.resolve_root(right)?;
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
            (Some(variable), _, Relation::Assignable | Relation::Castable) => {
                if let Some(right_variable) = right_variable {
                    // park value flow between open variables on both
                    // ends; either side closing makes progress
                    Ok(Answer::pending([
                        Dependency::Variable(variable),
                        Dependency::Variable(right_variable),
                    ]))
                } else {
                    self.push_upper_bound(variable, right)?;

                    Ok(Answer::Ready(true))
                }
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
        let left = match self.evaluate_root(origin, left)? {
            Answer::Ready(left) => left,
            Answer::Pending(_) => return Ok(None),
        };
        let right = match self.evaluate_root(origin, right)? {
            Answer::Ready(right) => right,
            Answer::Pending(_) => return Ok(None),
        };

        // fresh array literals of the exact length fill fixed arrays,
        // solving open counts to the literal length
        let fixed_fill = match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Array(array), dir::Type::FixedArray(fixed))
                if relation == Relation::Assignable =>
            {
                Some((array.element, fixed.element, fixed.count))
            }
            _ => None,
        };
        if let Some((source_element, target_element, count)) = fixed_fill {
            let count = self.resolve_root(count)?;
            let fillable = match self.fresh_array_literal_length(left)? {
                // open counts solve to the literal length
                Some(length) => match self.root_variable(count)? {
                    Some(variable) => match i64::try_from(length) {
                        Ok(length) => {
                            let source = self.origin_source_node(origin)?;
                            let literal = self.push_type(
                                origin.module(),
                                dir::Type::Literal(dir::ScalarLiteral::Integer(length)),
                                source,
                            )?;
                            self.push_lower_bound(variable, literal)?;
                            self.push_upper_bound(variable, literal)?;

                            true
                        }
                        Err(_) => false,
                    },
                    None => self.fixed_count_equals(count, length)?,
                },
                None => false,
            };
            // mismatched fills settle on the decide path once leaves close
            if !fillable {
                return Ok(Some(self.pending_on_open_leaves(left, right)?));
            }

            return Ok(Some(self.constrain(
                origin,
                Relation::Assignable,
                source_element,
                target_element,
            )?));
        }

        // same-symbol applications constrain arguments by variance
        let same_symbol = match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Reference(left), dir::Type::Reference(right))
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

        // collect child pairs with their child relations
        let mut pairs = SmallVec::<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
        match (self.ty(left)?, self.ty(right)?) {
            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(left), dir::Type::Array(right)) => {
                pairs.push((Relation::Equal, left.element, right.element));
            }
            (dir::Type::Slice(left), dir::Type::Slice(right)) => {
                pairs.push((Relation::Equal, left.element, right.element));
            }
            (dir::Type::Array(left), dir::Type::Slice(right))
                if relation == Relation::Assignable =>
            {
                pairs.push((Relation::Equal, left.element, right.element));
            }
            (dir::Type::FixedArray(left), dir::Type::FixedArray(right)) => {
                pairs.push((relation, left.element, right.element));
                pairs.push((Relation::Equal, left.count, right.count));
            }
            (dir::Type::Tuple(left), dir::Type::Tuple(right))
                if left.form == right.form && left.elements.len() == right.elements.len() =>
            {
                for (left, right) in left.elements.iter().zip(right.elements.iter()) {
                    pairs.push((relation, left.ty, right.ty));
                }
            }
            // shapes relate matching fields, assignability by target keys
            (dir::Type::Shape(left_shape), dir::Type::Shape(right)) => {
                // fresh literals may only supply known properties
                if right.index_signatures.is_empty() && self.is_fresh_literal(left)? {
                    for left_field in &left_shape.fields {
                        if !right.fields.iter().any(|field| field.key == left_field.key) {
                            return Ok(Some(Answer::Ready(false)));
                        }
                    }
                }
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
            (dir::Type::Function(left), dir::Type::Function(right)) => {
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
            // union sources flow every element into the target
            (dir::Type::Union(elements), _) if relation == Relation::Assignable => {
                for element in &elements.elements {
                    pairs.push((relation, *element, right));
                }
            }
            (dir::Type::Dynamic(left), dir::Type::Dynamic(right)) => {
                pairs.push((Relation::Equal, left.constraint, right.constraint));
            }
            // ambiguous composites wait for their open leaves instead
            _ => return Ok(Some(self.pending_on_open_leaves(left, right)?)),
        }

        // constrain every child pair through the bounding path
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (relation, left, right) in pairs {
            match self.constrain(origin, relation, left, right)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Some(Answer::Ready(false))),
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Some(Answer::Ready(true)))
        } else {
            Ok(Some(Answer::pending(blockers)))
        }
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

    /// Resolve one type root through variable aliases and solutions.
    pub(in crate::check) fn resolve_root(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // chase solved variables to their solutions
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let Some(solution) = self.variables.solution(*variable)? else {
                return Ok(current);
            };

            current = solution;
        }

        Ok(current)
    }

    /// Return the open variable at one resolved type root.
    pub(in crate::check) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.ty(id)? {
            dir::Type::Variable(variable) => Ok(Some(self.variables.representative(*variable)?)),
            _ => Ok(None),
        }
    }

    /// Report one failed closed relation.
    /// The constraint's source context picks the reported diagnostic.
    fn report_relation_failure(
        &mut self,
        origin: Origin,
        relation: Relation,
        cause: ConstraintCause,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let source = self.format_type(left);
        let target = self.format_type(right);

        // label the written annotation that demanded the target type
        let written = match self.written_type_anchor(right)? {
            Some(written) if written != anchor => {
                Some((written, format!("expected '{target}' from this annotation")))
            }
            _ => None,
        };

        // fresh literals name their excess property directly
        if matches!(
            relation,
            Relation::Assignable | Relation::Writable | Relation::Satisfies
        ) && let Some(key) = self.fresh_excess_property(origin, left, right)?
        {
            let error = CheckError::ExcessProperty {
                anchor,
                module,
                key,
                target,
            };
            let mut diagnostic = destack_artifact::DiagnosticBuilder::new(error)
                .note("object literals may only specify known properties");
            if let Some((written, label)) = written {
                diagnostic = diagnostic.label(written, label);
            }
            self.module_mut(module).diagnostics.push(diagnostic);

            return Ok(());
        }

        let error = match (relation, cause) {
            // explicit casts report their own failure shape
            (Relation::Castable, _) => CheckError::InvalidCast {
                anchor,
                module,
                source,
                target,
            },
            // check-only relations report their judgment kind
            (Relation::Satisfies, _) => CheckError::ConstraintNotSatisfied {
                anchor,
                module,
                source,
                target,
            },
            (Relation::Extends, _) => CheckError::DoesNotExtend {
                anchor,
                module,
                source,
                target,
            },
            (Relation::Implements, _) => CheckError::DoesNotImplement {
                anchor,
                module,
                source,
                target,
            },
            // value flow reports by its source context
            (_, ConstraintCause::Condition) => CheckError::NonBooleanCondition {
                anchor,
                module,
                actual: source,
            },
            (_, ConstraintCause::Argument) => CheckError::ArgumentNotAssignable {
                anchor,
                module,
                source,
                target,
            },
            (_, ConstraintCause::Return | ConstraintCause::Yield) => {
                CheckError::ReturnNotAssignable {
                    anchor,
                    module,
                    source,
                    target,
                }
            }
            (_, ConstraintCause::General) => CheckError::NotAssignable {
                anchor,
                module,
                source,
                target,
            },
        };

        let mut diagnostic = destack_artifact::DiagnosticBuilder::new(error);
        if let Some((written, label)) = written {
            diagnostic = diagnostic.label(written, label);
        }
        self.module_mut(module).diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return the source anchor of one type written as an annotation,
    /// so failures can point at the demanding annotation.
    fn written_type_anchor(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<DiagnosticAnchor>> {
        let id = self.resolve_root(id)?;

        // only component modules carry anchorable source
        let Some(module) = self.modules.get(&id.module_id) else {
            return Ok(None);
        };
        // working types are synthesized, not written
        if module.working.types.get_type_maybe(id.local_id).is_some() {
            return Ok(None);
        }

        let source = module.type_table().get_type_source(id.local_id);

        Ok(Some(self.diagnostic_anchor(id.module_id, source)))
    }

    /// Return the first excess property one fresh literal supplies to one target.
    fn fresh_excess_property(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        // peel the literal's managed wrapper
        let left = self.resolve_root(left)?;
        let left = match self.ty(left)? {
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                self.resolve_root(form.value)?
            }
            _ => left,
        };
        if !self.is_fresh_literal(left)? {
            return Ok(None);
        }
        let dir::Type::Shape(shape) = self.ty(left)? else {
            return Ok(None);
        };
        let keys = shape
            .fields
            .iter()
            .map(|field| field.key)
            .collect::<SmallVec<[_; 8]>>();

        // open targets accept any key
        let right = match self.evaluate_root(origin, right)? {
            Answer::Ready(right) => right,
            Answer::Pending(_) => return Ok(None),
        };
        let Some(accepted) = self.accepted_property_keys(origin, right)? else {
            return Ok(None);
        };

        for key in keys {
            if !accepted.contains(&key) {
                return Ok(Some(self.format_static_key(&key)));
            }
        }

        Ok(None)
    }

    /// Collect the property keys one target accepts, none when it accepts any.
    fn accepted_property_keys(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::StaticKey; 8]>>> {
        match self.ty(target)? {
            dir::Type::Shape(shape) => {
                // index signatures accept arbitrary keys
                if !shape.index_signatures.is_empty() {
                    return Ok(None);
                }

                Ok(Some(shape.fields.iter().map(|field| field.key).collect()))
            }
            dir::Type::Reference(instance) => {
                let symbol = instance.symbol;

                Ok(Some(self.nominal_member_keys(symbol)))
            }
            // unions accept keys known to any alternative
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut keys = SmallVec::new();
                for element in elements {
                    let element = match self.evaluate_root(origin, element)? {
                        Answer::Ready(element) => element,
                        Answer::Pending(_) => return Ok(None),
                    };
                    match self.accepted_property_keys(origin, element)? {
                        None => return Ok(None),
                        Some(element_keys) => keys.extend(element_keys),
                    }
                }

                Ok(Some(keys))
            }
            // form wrappers accept what their payloads accept
            dir::Type::Form(form) => {
                let value = self.resolve_root(form.value)?;

                self.accepted_property_keys(origin, value)
            }
            _ => Ok(None),
        }
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
                let table = module.type_table();

                Ok(table.get_type_source(ty.local_id))
            }
        }
    }
}
