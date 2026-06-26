use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, DumpContext, Origin, Relation, Widening};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Join multiple lower bounds into one best common solution.
    pub(in crate::check) fn best_common(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.solver.variable(variable)?.origin;

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
        // never judge lifetimes, so absorption would otherwise keep
        // one branch's lifetime arbitrarily
        let resolved = self.join_borrow_bounds(variable, origin, &resolved)?;

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
        // each other and collapse to one representative
        match survivors.as_slice() {
            [single] => Ok(*single),
            [] => match resolved.first() {
                Some(first) => Ok(*first),
                None => {
                    let module = variable.module_id;
                    let source = self.origin_source_node(origin)?;

                    self.push_type(module, dir::Type::Never, source)
                }
            },
            _ => {
                // lifetime joins meet: frame is always the minimum,
                // static never is beside another lifetime
                let survivors = self.meet_lifetime_survivors(survivors)?;
                if let [single] = survivors.as_slice() {
                    return Ok(*single);
                }

                let module = variable.module_id;
                let source = self.origin_source_node(origin)?;
                self.normalized_union_type(module, survivors, source)
            }
        }
    }

    /// Meet joined lifetime bounds: a frame bound absorbs the join,
    /// and static bounds drop beside any other lifetime.
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

    /// Join borrow bounds sharing one payload into one borrow whose
    /// lifetime is the union of the branch lifetimes.
    fn join_borrow_bounds(
        &mut self,
        variable: dir::TypeVariableId,
        origin: Origin,
        bounds: &SmallVec<[dir::GlobalTypeId; 4]>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        // collect each bound's borrow components
        let mut borrows = SmallVec::<[(dir::GlobalTypeId, dir::FormType); 4]>::new();
        for bound in bounds.iter().copied() {
            match self.ty(bound)? {
                dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed { .. }) => {
                    borrows.push((bound, *form));
                }
                _ => {}
            }
        }
        if borrows.len() < 2 {
            return Ok(bounds.clone());
        }

        // join groups that agree on payload and access
        let mut joined = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut consumed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for (index, (bound, form)) in borrows.iter().enumerate() {
            if consumed.contains(bound) {
                continue;
            }
            let dir::Form::Borrowed { lifetime, access } = form.form else {
                continue;
            };

            // collect group lifetimes in branch order
            let mut lifetimes = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            lifetimes.push(lifetime);
            for (other_bound, other_form) in borrows.iter().skip(index + 1) {
                let dir::Form::Borrowed {
                    lifetime: other_lifetime,
                    access: other_access,
                } = other_form.form
                else {
                    continue;
                };
                let payloads_equal = self
                    .decide_equal(origin, form.value, other_form.value)?
                    .is_ready_true();
                let accesses_equal = self.ty(access)? == self.ty(other_access)?;
                if payloads_equal && accesses_equal {
                    consumed.push(*other_bound);
                    if !self.lifetime_component_present(&lifetimes, other_lifetime)? {
                        lifetimes.push(other_lifetime);
                    }
                }
            }
            if consumed.is_empty() || lifetimes.len() < 2 {
                // groups of one keep their bound untouched
                if lifetimes.len() < 2 && consumed.iter().all(|other| other != bound) {
                    continue;
                }
            }

            // rebuild the borrow over the joined lifetime
            let lifetimes =
                self.meet_lifetime_survivors(lifetimes.into_iter().collect::<SmallVec<[_; 4]>>())?;
            let module = variable.module_id;
            let source = self.origin_source_node(origin)?;
            let lifetime = match lifetimes.as_slice() {
                [single] => *single,
                _ => self.push_type(
                    module,
                    dir::Type::Union(dir::UnionType {
                        elements: lifetimes.into_iter().collect(),
                    }),
                    source,
                )?,
            };
            let rebuilt = self.push_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed { lifetime, access },
                    value: form.value,
                }),
                source,
            )?;
            joined.push(rebuilt);
            consumed.push(*bound);
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

    /// Return whether one lifetime component is already collected.
    fn lifetime_component_present(
        &mut self,
        collected: &[dir::GlobalTypeId],
        lifetime: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for seen in collected {
            if self.ty(*seen)? == self.ty(lifetime)? {
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
        let origin = self.solver.variable(variable)?.origin;
        let module = variable.module_id;
        let source = self.origin_source_node(origin)?;
        let intersection = dir::Type::Intersection(dir::IntersectionType {
            elements: bounds.to_vec(),
        });

        self.push_type(module, intersection, source)
    }

    /// Widen one solution literal per the variable's policy.
    pub(in crate::check) fn widen_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = variable.module_id;
        let origin = self.solver.variable(variable)?.origin;
        let source = self.origin_source_node(origin)?;

        match widening {
            Widening::Preserve => Ok(solution),
            Widening::Widen => self.widen_type(module, source, solution),
        }
    }

    /// Widen one closed type, rebuilding literal leaves to their bases.
    pub(in crate::check) fn widen_type(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widening rebuilds literal leaves through aggregates and unions
        match self.widen_tree(module, source, ty, 0)? {
            Some(widened) => Ok(widened),
            None => Ok(ty),
        }
    }

    /// Rebuild one widening solution composite with widened leaves.
    ///
    /// Literal leaves widen to their base types, aggregate elements rebuild
    /// around their widened types, and unions collapse the duplicates widening creates.
    /// Returns none when nothing widens.
    fn widen_tree(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        id: dir::GlobalTypeId,
        depth: usize,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // recursive solutions stop widening at a fixed depth
        if depth > 16 {
            return Ok(None);
        }
        let id = self.settled_root(id)?;

        match self.ty(id)? {
            // literal leaves widen to their base types
            dir::Type::Literal(literal) => {
                let widened = literal.widen();

                Ok(Some(self.push_type(module, widened, source)?))
            }
            // enum member leaves widen to the owner enum
            dir::Type::EnumMember(member) => Ok(Some(member.owner)),
            // managed wrappers rebuild around their payloads
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                let value = form.value;
                let Some(widened) = self.widen_tree(module, source, value, depth + 1)? else {
                    return Ok(None);
                };
                let managed = dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value: widened,
                });

                Ok(Some(self.push_type(module, managed, source)?))
            }
            // collections rebuild around widened elements
            dir::Type::Array(array) => {
                let element = array.element;
                let Some(widened) = self.widen_tree(module, source, element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.push_type(
                    module,
                    dir::Type::Array(dir::ArrayType { element: widened }),
                    source,
                )?))
            }
            dir::Type::Slice(slice) => {
                let element = slice.element;
                let Some(widened) = self.widen_tree(module, source, element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.push_type(
                    module,
                    dir::Type::Slice(dir::SliceType { element: widened }),
                    source,
                )?))
            }
            dir::Type::FixedArray(array) => {
                let element = array.element;
                let count = array.count;
                let Some(widened) = self.widen_tree(module, source, element, depth + 1)? else {
                    return Ok(None);
                };

                Ok(Some(self.push_type(
                    module,
                    dir::Type::FixedArray(dir::FixedArrayType {
                        element: widened,
                        count,
                    }),
                    source,
                )?))
            }
            // tuples widen their element types in place
            dir::Type::Tuple(tuple) => {
                let mut tuple = tuple.clone();
                let mut changed = false;
                for element in &mut tuple.elements {
                    let ty = self.settled_root(element.ty)?;
                    if let Some(widened) = self.widen_tree(module, source, ty, depth + 1)? {
                        element.ty = widened;
                        changed = true;
                    } else {
                        element.ty = ty;
                    }
                }
                if !changed {
                    return Ok(None);
                }

                Ok(Some(self.push_type(
                    module,
                    dir::Type::Tuple(tuple),
                    source,
                )?))
            }
            // unions widen each member and collapse the duplicates
            dir::Type::Union(union) => {
                let elements = union.elements.clone();
                let mut widened = Vec::with_capacity(elements.len());
                let mut changed = false;
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.widen_tree(module, source, element, depth + 1)? {
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
                    let repeated = {
                        let ty = self.ty(element)?;
                        let mut repeated = false;
                        for kept in &distinct {
                            if self.ty(*kept)? == ty {
                                repeated = true;
                                break;
                            }
                        }
                        repeated
                    };
                    if !repeated {
                        distinct.push(element);
                    }
                }
                if let [single] = distinct.as_slice() {
                    return Ok(Some(*single));
                }

                Ok(Some(self.normalized_union_type(module, distinct, source)?))
            }
            // object literal shapes rebuild with widened field types
            dir::Type::Shape(shape) => {
                let mut shape = shape.clone();
                let mut changed = false;
                for field in &mut shape.fields {
                    let ty = self.settled_root(field.ty)?;
                    match self.widen_tree(module, source, ty, depth + 1)? {
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

                Ok(Some(self.push_type(
                    module,
                    dir::Type::Shape(shape),
                    source,
                )?))
            }
            _ => Ok(None),
        }
    }

    /// Return whether a source type widens directly to one target type.
    pub(in crate::check) fn widens_to(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let widens = match (self.ty(source)?.clone(), self.ty(target)?.clone()) {
            (dir::Type::Union(source), _) => self.union_widens_to(origin, &source, target)?,
            (_, dir::Type::Union(target)) => self.widens_to_union(origin, source, &target)?,
            _ => self.widens_to_single(origin, source, target)?,
        };

        Ok(widens)
    }

    /// Return whether a source type widens directly to one non-union type.
    fn widens_to_single(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let widens = match (self.ty(source)?.clone(), self.ty(target)?.clone()) {
            (dir::Type::Literal(literal), target) => literal.widens_to(&target),
            (dir::Type::Range(range), target) => range.widens_to(&target),
            (dir::Type::EnumMember(member), _) => self
                .decide_relation(origin, Relation::Equal, member.owner, target)?
                .is_ready_true(),
            (dir::Type::FixedArray(source_array), dir::Type::FixedArray(target_array)) => {
                let count_matches = self
                    .decide_equal(origin, source_array.count, target_array.count)?
                    .is_ready_true();

                count_matches
                    && self.widens_to(origin, source_array.element, target_array.element)?
            }
            _ => false,
        };

        Ok(widens)
    }

    /// Return whether a source type widens into one finite scalar union.
    fn widens_to_union(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: &dir::UnionType,
    ) -> CompilerResult<bool> {
        let Some(domain) = self.union_scalar_domain(origin, target)? else {
            return Ok(false);
        };
        if self.scalar_domain_type(origin, source)? != Some(domain) {
            return Ok(false);
        }

        // one finite member must contain the source without a representation change
        for element in &target.elements {
            let element = self.reduce_widening_type(origin, *element)?;
            if self.widens_to_single(origin, source, element)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one finite scalar union widens into one non-union target.
    fn union_widens_to(
        &mut self,
        origin: Origin,
        source: &dir::UnionType,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.union_scalar_domain(origin, source)?.is_none() {
            return Ok(false);
        }

        // every finite member must widen to the same target representation
        for element in &source.elements {
            let element = self.reduce_widening_type(origin, *element)?;
            if !self.widens_to_single(origin, element, target)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return the scalar storage family shared by all union elements.
    fn union_scalar_domain(
        &mut self,
        origin: Origin,
        union: &dir::UnionType,
    ) -> CompilerResult<Option<dir::ScalarDomain>> {
        let mut domain = None;
        for element in &union.elements {
            let element = self.reduce_widening_type(origin, *element)?;
            let Some(element_domain) = self.scalar_domain_type(origin, element)? else {
                return Ok(None);
            };

            match domain {
                Some(domain) if domain != element_domain => return Ok(None),
                Some(_) => {}
                None => domain = Some(element_domain),
            }
        }

        Ok(domain)
    }

    /// Return one widening operand after root reduction.
    fn reduce_widening_type(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let element = self.settled_root(element)?;
        match self.reduce_type_root(origin, element)? {
            Answer::Ready(element) => Ok(element),
            Answer::Pending(blockers) => {
                let context = DumpContext::new(self);
                let blockers = context.dependency_state_list_label(&blockers);

                Err(CompilerError::Internal {
                    message: format!("finished widening operand is still pending: {blockers}"),
                })
            }
        }
    }

    /// Return the scalar storage family for one finite scalar type.
    fn scalar_domain_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarDomain>> {
        let ty = self.reduce_widening_type(origin, ty)?;
        let domain = self.ty(ty)?.scalar_domain();

        Ok(domain)
    }
}
