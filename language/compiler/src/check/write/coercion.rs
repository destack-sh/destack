use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    Answer, CheckState, Constraint, ConstraintId, ConstraintState, Origin, Relation, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return implicit coercions from solved value constraints.
    pub(in crate::check) fn implicit_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let mut constraints = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            let state = self.solver.constraints.state(id)?;
            if state == ConstraintState::Holds && constraint.origin().module() == module {
                constraints.push(id);
            }
        }

        let mut coercions = IndexMap::new();
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
        coercions: &mut IndexMap<dir::GlobalNodeIdAny, dir::Coercion>,
        (node, coercion): (dir::GlobalNodeIdAny, dir::Coercion),
    ) -> CompilerResult<()> {
        let Some(previous) = coercions.get(&node).copied() else {
            coercions.insert(node, coercion);

            return Ok(());
        };

        if self.coercions_match(Origin::Node(node), previous, coercion)? {
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
            ValueUse::Store | ValueUse::Argument | ValueUse::Output
        ) {
            return Ok(None);
        }

        let source = self.settled_root(left)?;
        let target = self.settled_root(right)?;
        if !self.type_variables(source)?.is_empty() || !self.type_variables(target)?.is_empty() {
            return Ok(None);
        }

        let Some(coercion) = self.implicit_coercion(origin, source, target)? else {
            return Ok(None);
        };

        Ok(Some((node.into_any(), coercion)))
    }

    /// Return the implicit coercion required by one solved source-target pair.
    fn implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Coercion>> {
        if !self.requires_implicit_coercion(origin, source, target)? {
            return Ok(None);
        }

        let coercion = dir::Coercion::new(source, target, dir::CastOrigin::Implicit);

        Ok(Some(coercion))
    }

    /// Return whether one accepted value constraint changes stored representation.
    fn requires_implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if source == target {
            return Ok(false);
        }

        let source = self.coercion_type(origin, source)?;
        let target = self.coercion_type(origin, target)?;
        if source == target {
            return Ok(false);
        }

        if self.types_are_equal(origin, source, target)? {
            return Ok(false);
        }

        if self.type_has_runtime_header(source)? || self.type_has_runtime_header(target)? {
            return Ok(true);
        }

        if self.memory_representation_changes(source, target)?
            || self.intrinsic_representation_changes(source, target)?
        {
            return Ok(true);
        }

        if self.scalar_stores_directly(source, target)? {
            return Ok(false);
        }

        if self.stored_scalars_convert(source, target)? {
            return Ok(true);
        }

        Ok(false)
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

    /// Return whether a type stores a runtime header.
    fn type_has_runtime_header(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let has_header = matches!(
            self.ty(ty)?,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Dynamic(_)
                | dir::Type::Union(_)
        );

        Ok(has_header)
    }

    /// Return whether one source-target pair crosses a memory representation.
    fn memory_representation_changes(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = self.ty(source)?;
        let target = self.ty(target)?;
        let changes = match (source, target) {
            // compare two memory forms as source and target
            (dir::Type::Form(source), dir::Type::Form(target)) => {
                Self::memory_form_representation_changes(source.form, target.form)
            }

            // placement and readonly do not create casts to or from their payloads
            (dir::Type::Form(form), _) | (_, dir::Type::Form(form))
                if matches!(form.form, dir::Form::Placed { .. } | dir::Form::Readonly) =>
            {
                false
            }
            // managed is transparent after type reduction in this write path
            (dir::Type::Form(form), _) | (_, dir::Type::Form(form))
                if form.form == dir::Form::Managed =>
            {
                false
            }

            // owned, borrowed, and raw cross a value carrier boundary
            (dir::Type::Form(_), _) | (_, dir::Type::Form(_)) => true,

            // non-form types do not cross a memory representation
            _ => false,
        };

        Ok(changes)
    }

    /// Return whether two memory form constructors have different stored representations.
    fn memory_form_representation_changes(source: dir::Form, target: dir::Form) -> bool {
        match (source, target) {
            // placement, readonly, and managed are static or transparent after reduction
            (dir::Form::Placed { .. } | dir::Form::Readonly | dir::Form::Managed, _)
            | (_, dir::Form::Placed { .. } | dir::Form::Readonly | dir::Form::Managed) => false,

            // static borrow parameters do not change the pointer representation
            (dir::Form::Borrowed { .. }, dir::Form::Borrowed { .. }) => false,

            // equal runtime carriers do not need an emitted conversion
            (dir::Form::Owned, dir::Form::Owned) | (dir::Form::Raw, dir::Form::Raw) => false,

            // different runtime carriers need an emitted conversion
            _ => true,
        }
    }

    /// Return whether one source-target pair crosses an intrinsic representation.
    fn intrinsic_representation_changes(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let changes = matches!(
            (self.ty(source)?, self.ty(target)?),
            (dir::Type::Array(_), dir::Type::Slice(_))
                | (dir::Type::FixedArray(_), dir::Type::Slice(_))
                | (dir::Type::FunctionPointer(_), dir::Type::Function(_))
        );

        Ok(changes)
    }

    /// Return whether a scalar singleton stores directly in the target type.
    fn scalar_stores_directly(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let target_type = self.ty(target)?;
        let stores = match self.ty(source)? {
            dir::Type::Literal(literal) => literal.widens_to(&target_type),
            dir::Type::Range(range) => range.widens_to(&target_type),
            _ => false,
        };

        Ok(stores)
    }

    /// Return whether stored scalar carriers need conversion instructions.
    fn stored_scalars_convert(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let converts = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => {
                source.widens_to(target)
            }
            _ => false,
        };

        Ok(converts)
    }
}
