use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, Constraint, ConstraintId, ConstraintState, Origin, Relation, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Record one already accepted value constraint for coercion derivation.
    pub(in crate::check) fn push_solved_constraint(
        &mut self,
        origin: Origin,
        use_: ValueUse,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let origin = self.intern_origin(origin);
        let constraint = Constraint::value(
            Relation::Assignable,
            source,
            target,
            origin,
            origin,
            Some(use_),
        );
        let id = self.solver.allocate_constraint(constraint);
        self.solver
            .set_constraint_state(id, ConstraintState::Holds)?;

        Ok(())
    }

    /// Return implicit coercions from solved value constraints.
    pub(in crate::check) fn implicit_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let mut constraints = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            let state = self.solver.constraints.state(id)?;
            if state == ConstraintState::Holds
                && self.solver.origin(constraint.origin()).module() == module
            {
                constraints.push(id);
            }
        }

        let mut coercions = FxIndexMap::default();
        for constraint in constraints {
            if let Some(coercion) = self.constraint_coercion(module, constraint)? {
                self.push_implicit_coercion(&mut coercions, coercion)?;
            }
        }

        Ok(coercions.into_iter().collect())
    }

    /// Keep one implicit coercion per value node.
    fn push_implicit_coercion(
        &mut self,
        coercions: &mut FxIndexMap<dir::GlobalNodeIdAny, dir::Coercion>,
        (node, coercion): (dir::GlobalNodeIdAny, dir::Coercion),
    ) -> CompilerResult<()> {
        let Some(previous) = coercions.get(&node).copied() else {
            coercions.insert(node, coercion);

            return Ok(());
        };

        if self.coercions_match(self.node_site(node)?.origin(), previous, coercion)? {
            return Ok(());
        }

        Err(CompilerError::Internal {
            message: format!(
                "node {node:?} received conflicting implicit coercions {previous:?} and {coercion:?}"
            ),
        })
    }

    /// Return whether two coercions perform the same representation change.
    fn coercions_match(
        &mut self,
        origin: Origin,
        left: dir::Coercion,
        right: dir::Coercion,
    ) -> CompilerResult<bool> {
        if left.origin != right.origin {
            return Ok(false);
        }
        if !self.types_are_equal(origin, left.source, right.source)? {
            return Ok(false);
        }

        self.types_are_equal(origin, left.target, right.target)
    }

    /// Return one implicit coercion from one solved value constraint.
    fn constraint_coercion(
        &mut self,
        module: ModuleId,
        constraint: ConstraintId,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let (relation, use_, left, right, origin) = {
            let constraint = self.solver.constraints.get(constraint)?;

            let Constraint::Value(constraint) = constraint else {
                return Ok(None);
            };

            (
                constraint.relation,
                constraint.use_,
                constraint.source,
                constraint.target,
                constraint.origin,
            )
        };

        let origin = self.solver.origin(origin);
        let Some(node) = origin.expression() else {
            return Ok(None);
        };
        if node.module_id != module {
            return Ok(None);
        }
        if relation != Relation::Assignable {
            return Ok(None);
        }
        if !matches!(
            use_,
            Some(ValueUse::Store | ValueUse::Argument | ValueUse::Output)
        ) {
            return Ok(None);
        }

        let source = self.settled_root(left)?;
        let source = self.settled_union_root(source)?;
        let target = self.settled_root(right)?;
        let target = self.settled_union_root(target)?;

        // guard open leaves on reduced heads, keeping written types for display
        let origin = self.node_site(node.into_any())?.origin();
        let Answer::Ready(reduced_source) = self.reduce_type_head(origin, source)? else {
            return Ok(None);
        };
        let Answer::Ready(reduced_target) = self.reduce_type_head(origin, target)? else {
            return Ok(None);
        };
        if !self.type_variables(reduced_source)?.is_empty()
            || !self.type_variables(reduced_target)?.is_empty()
        {
            return Ok(None);
        }

        // coercions never create ownership: value forms that reduce away drop
        let target = self.coercion_target(target, reduced_target)?;
        let Some(coercion) = self.implicit_coercion(origin, source, target)? else {
            return Ok(None);
        };

        Ok(Some((node.into_any(), coercion)))
    }

    /// Return the coercion target, dropping one value form head that reduces away.
    fn coercion_target(
        &self,
        target: dir::GlobalTypeId,
        reduced_target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Form(form) = self.ty(target)? else {
            return Ok(target);
        };
        if matches!(self.ty(reduced_target)?, dir::Type::Form(_)) {
            return Ok(target);
        }

        Ok(form.value)
    }

    /// Collapse one union whose elements all settle to the same root.
    fn settled_union_root(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Union(union) = self.ty(ty)? else {
            return Ok(ty);
        };
        let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
        let mut settled = Vec::with_capacity(elements.len());
        for element in elements {
            settled.push(self.settled_root(element)?);
        }

        match settled.as_slice() {
            [first, rest @ ..] if rest.iter().all(|element| element == first) => Ok(*first),
            _ => Ok(ty),
        }
    }

    /// Return the implicit coercion required by one solved source-target pair.
    fn implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Coercion>> {
        // identical or equal settled types store directly
        if source == target {
            return Ok(None);
        }
        let judged_source = self.coercion_type(origin, source)?;
        let judged_target = self.coercion_type(origin, target)?;
        if judged_source == judged_target
            || self.types_are_equal(origin, judged_source, judged_target)?
        {
            return Ok(None);
        }

        // classify the settled heads at the DIR level
        let source_head = self.ty(judged_source)?;
        let target_head = self.ty(judged_target)?;

        // tuples reshape when their element layouts differ
        if let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
            (source_head, target_head)
        {
            let reshapes = self.tuple_layouts_differ(
                judged_source,
                &source_tuple,
                judged_target,
                &target_tuple,
            )?;

            return Ok(reshapes.then(|| {
                dir::Coercion::new(
                    source,
                    target,
                    dir::CoercionKind::Carrier,
                    dir::CastOrigin::Implicit,
                )
            }));
        }

        let Some(kind) = dir::Coercion::classify(&source_head, &target_head) else {
            return Ok(None);
        };

        Ok(Some(dir::Coercion::new(
            source,
            target,
            kind,
            dir::CastOrigin::Implicit,
        )))
    }

    /// Return whether two tuple types store their elements differently.
    fn tuple_layouts_differ(
        &self,
        source: dir::GlobalTypeId,
        source_tuple: &dir::TupleType,
        target: dir::GlobalTypeId,
        target_tuple: &dir::TupleType,
    ) -> CompilerResult<bool> {
        let source_elements = self.tuple_elements(source.module_id, source_tuple.elements)?;
        let target_elements = self.tuple_elements(target.module_id, target_tuple.elements)?;
        if source_elements.len() != target_elements.len() {
            return Ok(true);
        }

        // optional slots store as their unioned value, so only rest
        //  slots change the stored layout at equal arity
        let differs = source_elements
            .iter()
            .zip(target_elements)
            .any(|(source, target)| source.is_rest != target.is_rest);

        Ok(differs)
    }

    /// Return the type used to judge coercion representation.
    fn coercion_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut ty = self.reduce_type_ready(origin, ty, "coercion representation")?;
        while let dir::Type::Form(form) = self.ty(ty)?
            && form.form == dir::Form::Readonly
        {
            ty = self.reduce_type_ready(origin, form.value, "coercion readonly payload")?;
        }

        Ok(ty)
    }

    /// Return whether two closed types are equal at write time.
    fn types_are_equal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.decide_relation(origin, Relation::Equal, source, target)? {
            Answer::Ready(holds) => Ok(holds),
            Answer::Pending(blockers) => {
                let source = self.format_type(source);
                let target = self.format_type(target);

                Err(CompilerError::Internal {
                    message: format!(
                        "coercion equality from '{source}' to '{target}' is still pending: {blockers:?}"
                    ),
                })
            }
        }
    }

    /// Return one reduced type or fail when write sees unsolved state.
    fn reduce_type_ready(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        operation: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.reduce_type_head(origin, ty)? {
            Answer::Ready(ty) => Ok(ty),
            Answer::Pending(blockers) => {
                let ty = self.format_type(ty);

                Err(CompilerError::Internal {
                    message: format!("{operation} for '{ty}' is still pending: {blockers:?}"),
                })
            }
        }
    }
}
