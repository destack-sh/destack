use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{Cause, CauseKind, CheckState, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Join multiple lower bounds into one best common solution.
    pub(in crate::sema) fn best_common(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.infer.variable(variable)?.origin;
        let origin = self.infer.origin(origin);

        // resolve bounds through solved variables
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in bounds {
            let bound = self.shallow_resolve(*bound).map_err(|error| {
                CompilerError::Internal {
                    message: format!(
                        "best_common bound resolution failed for {variable:?} bounds={bounds:?}: {error:?}"
                    ),
                }
            })?;
            if !resolved.contains(&bound) {
                resolved.push(bound);
            }
        }

        // lifetime variables join through the provenance meet, never through relations
        if self.variable_memory_parameter(variable)? == Some(dir::MemoryParameter::Lifetime) {
            let survivors = self.meet_lifetime_survivors(resolved)?;

            return match survivors.as_slice() {
                [] => self.intern_type(dir::Type::Never),
                [single] => Ok(*single),
                _ => self.normalized_union_type(survivors.iter().copied()),
            };
        }

        // join borrow bounds over one payload before absorption picks a branch
        let resolved = self.join_borrow_bounds(origin, &resolved)?;

        // drop bounds absorbed by another bound
        let mut survivors = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in resolved.iter().copied() {
            let mut absorbed = false;
            for other in resolved.iter().copied() {
                if other == bound {
                    continue;
                }

                absorbed = self
                    .evaluate_relation(origin, Relation::Assignable, bound, other)?
                    .holds();
                if absorbed {
                    break;
                }
            }
            if !absorbed {
                survivors.push(bound);
            }
        }

        // join the survivors, collapsing mutually absorbed bounds to their first
        match survivors.as_slice() {
            [single] => Ok(*single),
            [] => match resolved.first() {
                Some(first) => Ok(*first),
                None => {
                    let _module = origin.module();

                    self.intern_type(dir::Type::Never)
                }
            },
            _ => self.normalized_union_type(survivors),
        }
    }

    /// Meet lifetime bounds, where frame absorbs the join and static drops beside others.
    fn meet_lifetime_survivors(
        &mut self,
        survivors: SmallVec<[dir::GlobalTypeId; 4]>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        if survivors.len() < 2 {
            return Ok(survivors);
        }

        for bound in survivors.iter().copied() {
            if matches!(
                self.ty(bound)?,
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame))
            ) {
                return Ok(SmallVec::from_slice(&[bound]));
            }
        }

        let mut kept = SmallVec::new();
        for bound in survivors.iter().copied() {
            if !matches!(
                self.ty(bound)?,
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Static))
            ) {
                kept.push(bound);
            }
        }
        if kept.is_empty() {
            return Ok(survivors);
        }

        Ok(kept)
    }

    /// Join borrow bounds sharing one payload into one borrow over the joined lifetimes.
    fn join_borrow_bounds(
        &mut self,
        origin: Origin,
        bounds: &SmallVec<[dir::GlobalTypeId; 4]>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        // collect each bound's borrow components
        let mut borrows =
            SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId, dir::BorrowForm); 4]>::new();
        for bound in bounds.iter().copied() {
            if let dir::Type::Form(form) = self.ty(bound)?
                && let dir::Form::Borrowed(borrow) = form.form
            {
                let borrow = self.type_borrow(bound.module_id, borrow)?;
                borrows.push((bound, form.value, borrow));
            }
        }
        if borrows.len() < 2 {
            return Ok(bounds.clone());
        }

        // join groups that agree on payload and access
        let mut joined = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut consumed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for (index, (bound, value, borrow)) in borrows.iter().copied().enumerate() {
            if consumed.contains(&bound) {
                continue;
            }

            // collect group lifetimes in branch order
            let mut lifetimes = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            lifetimes.push(borrow.lifetime);
            for (other_bound, other_value, other_borrow) in borrows.iter().copied().skip(index + 1)
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                let payloads_equal = self
                    .relate_equal(origin, cause, value, other_value)?
                    .holds();
                let accesses_equal = self.ty(borrow.access)? == self.ty(other_borrow.access)?;
                if payloads_equal && accesses_equal {
                    consumed.push(other_bound);
                    if !self.contains_type_head(&lifetimes, other_borrow.lifetime)? {
                        lifetimes.push(other_borrow.lifetime);
                    }
                }
            }

            // keep single-lifetime groups untouched, absorbing consumed duplicates
            if lifetimes.len() < 2 {
                continue;
            }

            // rebuild the borrow over the joined lifetime
            let lifetimes =
                self.meet_lifetime_survivors(lifetimes.into_iter().collect::<SmallVec<[_; 4]>>())?;
            let lifetime = match lifetimes.as_slice() {
                [single] => *single,
                _ => {
                    let elements = self.intern_type_ids(&lifetimes)?;

                    self.intern_type(dir::Type::Union(dir::UnionType { elements }))?
                }
            };
            let joined_form = self.intern_borrow(lifetime, borrow.access)?;
            let rebuilt = self.intern_type(dir::Type::Form(dir::FormType {
                form: joined_form,
                value,
            }))?;
            joined.push(rebuilt);
            consumed.push(bound);
        }
        if joined.is_empty() {
            return Ok(bounds.clone());
        }

        // keep unjoined bounds in order, then the joined borrows
        let mut merged = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in bounds.iter().copied() {
            if !consumed.contains(&bound) {
                merged.push(bound);
            }
        }
        merged.extend(joined);

        Ok(merged)
    }

    /// Return whether one type head is already collected.
    fn contains_type_head(
        &self,
        collected: &[dir::GlobalTypeId],
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let head = self.ty(ty)?;
        for seen in collected {
            if self.ty(*seen)? == head {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Meet multiple contextual upper bounds into one solution.
    pub(in crate::sema) fn intersect_bounds(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.infer.variable(variable)?.origin;
        let _origin = self.infer.origin(origin);

        self.normalized_intersection_type(bounds.iter().copied())
    }

    /// Widen one closed type, rebuilding literal leaves to their bases.
    pub(in crate::sema) fn widen_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widening rebuilds literal leaves through aggregates and unions
        let mut active = FxIndexSet::default();
        match self.widen_tree(ty.module_id, ty, &mut active)? {
            Some(widened) => Ok(widened),
            None => Ok(ty),
        }
    }

    /// Widen one closed type, keeping const integer literals in the integer family.
    pub(in crate::sema) fn widen_const_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widen an integer literal to the declared integer default
        if let dir::Type::Literal(literal @ dir::ScalarLiteral::Integer(_)) = self.ty(ty)? {
            let widened = literal.widen_integer();

            return self.intern_type(widened);
        }

        self.widen_type(ty)
    }

    /// Rebuild one widening solution composite with widened leaves.
    fn widen_tree(
        &mut self,
        module: ModuleId,
        id: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // self-referential solutions keep their recursive leaves
        let id = self.shallow_resolve(id)?;
        if !active.insert(id) {
            return Ok(None);
        }
        let widened = self.widen_tree_children(module, id, active);
        active.swap_remove(&id);

        widened
    }

    /// Rebuild one reduced composite root with widened children.
    fn widen_tree_children(
        &mut self,
        module: ModuleId,
        id: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match self.ty(id)? {
            // literal leaves widen to their base types
            dir::Type::Literal(literal) => {
                let widened = literal.widen();

                Ok(Some(self.intern_type(widened)?))
            }
            // enum member leaves widen to the owner enum
            dir::Type::Variant(member) => Ok(Some(member.owner)),
            // managed forms rebuild around their payloads
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                let Some(widened) = self.widen_tree(module, form.value, active)? else {
                    return Ok(None);
                };
                let managed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value: widened,
                });

                Ok(Some(self.intern_type(managed)?))
            }
            // collections rebuild around widened elements
            _ if let Some(element) = self.array_element(id)? => {
                let Some(widened) = self.widen_tree(module, element, active)? else {
                    return Ok(None);
                };

                Ok(Some(self.array_type(widened)?))
            }
            dir::Type::Slice(slice) => {
                let Some(widened) = self.widen_tree(module, slice.element, active)? else {
                    return Ok(None);
                };

                Ok(Some(self.intern_type(dir::Type::Slice(
                    dir::SliceType { element: widened },
                ))?))
            }
            dir::Type::FixedArray(array) => {
                let Some(widened) = self.widen_tree(module, array.element, active)? else {
                    return Ok(None);
                };

                Ok(Some(self.intern_type(dir::Type::FixedArray(
                    dir::FixedArrayType {
                        element: widened,
                        count: array.count,
                    },
                ))?))
            }
            // tuples widen their element types in place
            dir::Type::Tuple(tuple) => {
                let mut elements = SmallVec::<[dir::TypeElement; 8]>::from_slice(
                    self.tuple_elements(module, tuple.elements)?,
                );
                let mut changed = false;
                for element in &mut elements {
                    let ty = self.shallow_resolve(element.ty)?;
                    if let Some(widened) = self.widen_tree(module, ty, active)? {
                        element.ty = widened;
                        changed = true;
                    } else {
                        element.ty = ty;
                    }
                }
                if !changed {
                    return Ok(None);
                }

                let elements = self.intern_elements(&elements)?;
                let tuple = dir::TupleType {
                    form: tuple.form,
                    elements,
                };

                Ok(Some(self.intern_type(dir::Type::Tuple(tuple))?))
            }
            // unions widen each member and collapse the duplicates
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> = self.type_ids(module, union.elements)?.into();
                let mut widened = Vec::with_capacity(elements.len());
                let mut changed = false;
                for element in elements {
                    let element = self.shallow_resolve(element)?;
                    match self.widen_tree(module, element, active)? {
                        Some(wide) => {
                            widened.push(wide);
                            changed = true;
                        }
                        None => widened.push(element),
                    }
                }
                if !changed {
                    return Ok(None);
                }

                // drop members that repeat an earlier member's type
                let mut distinct = Vec::<dir::GlobalTypeId>::new();
                for element in widened {
                    if !self.contains_type_head(&distinct, element)? {
                        distinct.push(element);
                    }
                }
                if let [single] = distinct.as_slice() {
                    return Ok(Some(*single));
                }

                Ok(Some(self.normalized_union_type(distinct)?))
            }
            // object literal shapes rebuild with widened field types
            dir::Type::Object(shape) => {
                let mut fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
                    self.shape_properties(module, shape.properties)?,
                );
                let mut changed = false;
                for field in &mut fields {
                    field.access = match field.access {
                        dir::PropertyAccess::Read(ty) => {
                            let (ty, widened) = self.widen_property_slot(module, ty, active)?;
                            changed |= widened;

                            dir::PropertyAccess::Read(ty)
                        }
                        dir::PropertyAccess::Write(ty) => {
                            let (ty, widened) = self.widen_property_slot(module, ty, active)?;
                            changed |= widened;

                            dir::PropertyAccess::Write(ty)
                        }
                        dir::PropertyAccess::ReadWrite { read, write } => {
                            let (read, read_widened) =
                                self.widen_property_slot(module, read, active)?;
                            let (write, write_widened) =
                                self.widen_property_slot(module, write, active)?;
                            changed |= read_widened | write_widened;

                            dir::PropertyAccess::ReadWrite { read, write }
                        }
                    };
                }
                if !changed {
                    return Ok(None);
                }

                let fields = self.intern_properties(&fields)?;
                let shape = dir::ShapeType {
                    properties: fields,
                    call_signatures: shape.call_signatures,
                    construct_signatures: shape.construct_signatures,
                    index_signatures: shape.index_signatures,
                };

                Ok(Some(self.intern_type(dir::Type::Object(shape))?))
            }
            _ => Ok(None),
        }
    }
    /// Widen one property value slot toward its settled root.
    fn widen_property_slot(
        &mut self,
        module: ModuleId,
        ty: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<(dir::GlobalTypeId, bool)> {
        let ty = self.shallow_resolve(ty)?;

        match self.widen_tree(module, ty, active)? {
            Some(widened) => Ok((widened, true)),
            None => Ok((ty, false)),
        }
    }
}
