use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, Relation};
use crate::{CompilerError, CompilerResult};

/// Maximum recursive widening depth for self-referential solution graphs.
const MAX_WIDENING_DEPTH: usize = 16;

impl CheckState<'_> {
    /// Join multiple lower bounds into one best common solution.
    pub(in crate::check) fn best_common(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.solver.variable(variable)?.origin;
        let origin = self.solver.origin(origin);

        // resolve bounds through solved variables
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in bounds {
            let bound = self.settled_root(*bound).map_err(|error| {
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

        // borrows over one payload join their lifetimes: relations
        //  never judge lifetimes, so absorption would otherwise keep
        //  one branch's lifetime arbitrarily
        let resolved = self.join_borrow_bounds(origin, &resolved)?;

        // drop bounds absorbed by another bound
        let mut survivors = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in resolved.iter().copied() {
            let mut absorbed = false;
            for other in resolved.iter().copied() {
                if other != bound
                    && self
                        .decide_relation(origin, Relation::Assignable, bound, other)?
                        .is_ready_true()
                {
                    absorbed = true;

                    break;
                }
            }
            if !absorbed {
                survivors.push(bound);
            }
        }

        // join the surviving bounds; mutually equivalent bounds absorb
        //  each other and collapse to one representative
        match survivors.as_slice() {
            [single] => Ok(*single),
            [] => match resolved.first() {
                Some(first) => Ok(*first),
                None => {
                    let module = origin.module();

                    self.intern_type(module, dir::Type::Never)
                }
            },
            _ => {
                // lifetime joins meet: frame is always the minimum,
                //  static never is beside another lifetime
                let survivors = self.meet_lifetime_survivors(survivors)?;
                if let [single] = survivors.as_slice() {
                    return Ok(*single);
                }

                let module = origin.module();
                self.normalized_union_type(module, survivors)
            }
        }
    }

    /// Meet joined lifetime bounds: a frame bound absorbs the join, and
    /// static drops beside any other lifetime.
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

    /// Join borrow bounds sharing one payload into one borrow over the
    /// union of their lifetimes.
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
                let payloads_equal = self
                    .decide_equal(origin, value, other_value)?
                    .is_ready_true();
                let accesses_equal = self.ty(borrow.access)? == self.ty(other_borrow.access)?;
                if payloads_equal && accesses_equal {
                    consumed.push(other_bound);
                    if !self.contains_type_head(&lifetimes, other_borrow.lifetime)? {
                        lifetimes.push(other_borrow.lifetime);
                    }
                }
            }

            // groups without a second lifetime keep their bound untouched,
            //  absorbing consumed equal-lifetime duplicates
            if lifetimes.len() < 2 {
                continue;
            }

            // rebuild the borrow over the joined lifetime
            let lifetimes =
                self.meet_lifetime_survivors(lifetimes.into_iter().collect::<SmallVec<[_; 4]>>())?;
            let module = origin.module();
            let lifetime = match lifetimes.as_slice() {
                [single] => *single,
                _ => {
                    let elements = self.intern_type_ids(module, &lifetimes)?;

                    self.intern_type(module, dir::Type::Union(dir::UnionType { elements }))?
                }
            };
            let joined_form = self.intern_borrow(module, lifetime, borrow.access)?;
            let rebuilt = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: joined_form,
                    value,
                }),
            )?;
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
    pub(in crate::check) fn intersect_bounds(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // repeated bounds contribute one intersection member
        let mut unique = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in bounds {
            if !unique.contains(bound) {
                unique.push(*bound);
            }
        }
        let origin = self.solver.variable(variable)?.origin;
        let origin = self.solver.origin(origin);
        let module = origin.module();
        if let [single] = unique.as_slice() {
            return Ok(*single);
        }
        let elements = self.intern_type_ids(module, &unique)?;
        let intersection = dir::Type::Intersection(dir::IntersectionType { elements });

        self.intern_type(module, intersection)
    }

    /// Widen one closed type, rebuilding literal leaves to their bases.
    pub(in crate::check) fn widen_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widening rebuilds literal leaves through aggregates and unions
        match self.widen_tree(ty.module_id, ty, 0)? {
            Some(widened) => Ok(widened),
            None => Ok(ty),
        }
    }

    /// Rebuild one widening solution composite with widened leaves.
    fn widen_tree(
        &mut self,
        module: ModuleId,
        id: dir::GlobalTypeId,
        depth: usize,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // recursive solutions stop widening at a fixed depth
        if depth > MAX_WIDENING_DEPTH {
            return Ok(None);
        }
        let id = self.settled_root(id)?;

        match self.ty(id)? {
            // literal leaves widen to their base types
            dir::Type::Literal(literal) => {
                let widened = literal.widen();

                Ok(Some(self.intern_type(module, widened)?))
            }
            // enum member leaves widen to the owner enum
            dir::Type::EnumMember(member) => Ok(Some(member.owner)),
            // managed forms rebuild around their payloads
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                let Some(widened) = self.widen_tree(module, form.value, depth + 1)? else {
                    return Ok(None);
                };
                let managed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value: widened,
                });

                Ok(Some(self.intern_type(module, managed)?))
            }
            // collections rebuild around widened elements
            dir::Type::Array(array) => {
                let Some(widened) = self.widen_tree(module, array.element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.intern_type(
                    module,
                    dir::Type::Array(dir::ArrayType { element: widened }),
                )?))
            }
            dir::Type::Slice(slice) => {
                let Some(widened) = self.widen_tree(module, slice.element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.intern_type(
                    module,
                    dir::Type::Slice(dir::SliceType { element: widened }),
                )?))
            }
            dir::Type::FixedArray(array) => {
                let Some(widened) = self.widen_tree(module, array.element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.intern_type(
                    module,
                    dir::Type::FixedArray(dir::FixedArrayType {
                        element: widened,
                        count: array.count,
                    }),
                )?))
            }
            // tuples widen their element types in place
            dir::Type::Tuple(tuple) => {
                let mut elements = SmallVec::<[dir::TypeElement; 8]>::from_slice(
                    self.tuple_elements(module, tuple.elements)?,
                );
                let mut changed = false;
                for element in &mut elements {
                    let ty = self.settled_root(element.ty)?;
                    if let Some(widened) = self.widen_tree(module, ty, depth + 1)? {
                        element.ty = widened;
                        changed = true;
                    } else {
                        element.ty = ty;
                    }
                }
                if !changed {
                    return Ok(None);
                }

                let elements = self.intern_elements(module, &elements)?;
                let tuple = dir::TupleType {
                    form: tuple.form,
                    elements,
                };

                Ok(Some(self.intern_type(module, dir::Type::Tuple(tuple))?))
            }
            // unions widen each member and collapse the duplicates
            dir::Type::Union(union) => {
                let elements = self.type_ids(module, union.elements)?.to_vec();
                let mut widened = Vec::with_capacity(elements.len());
                let mut changed = false;
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.widen_tree(module, element, depth + 1)? {
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

                Ok(Some(self.normalized_union_type(module, distinct)?))
            }
            // object literal shapes rebuild with widened field types
            dir::Type::Shape(shape) => {
                let mut fields = SmallVec::<[dir::TypeField; 8]>::from_slice(
                    self.shape_fields(module, shape.fields)?,
                );
                let mut changed = false;
                for field in &mut fields {
                    let ty = self.settled_root(field.ty)?;
                    match self.widen_tree(module, ty, depth + 1)? {
                        Some(widened) => {
                            field.ty = widened;
                            changed = true;
                        }
                        None => field.ty = ty,
                    }
                }
                if !changed {
                    return Ok(None);
                }

                let fields = self.intern_fields(module, &fields)?;
                let shape = dir::ShapeType {
                    fields,
                    call_signatures: shape.call_signatures,
                    construct_signatures: shape.construct_signatures,
                    index_signatures: shape.index_signatures,
                };

                Ok(Some(self.intern_type(module, dir::Type::Shape(shape))?))
            }
            _ => Ok(None),
        }
    }
}
