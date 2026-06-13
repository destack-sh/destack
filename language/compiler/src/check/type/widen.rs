use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Origin, Relation, Widening};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Join multiple lower bounds into one best common solution.
    pub(in crate::check) fn best_common(
        &mut self,
        variable: dir::TypeVariableId,
        bounds: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.variables.get(variable)?.origin;

        // resolve bounds through solved variables
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in bounds {
            let bound = self.resolve_root(*bound).map_err(|error| {
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
                    && self.decide_relation(origin, Relation::Assignable, bound, other)?
                        == Answer::Ready(true)
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
                let union = dir::Type::Union(dir::UnionType {
                    elements: survivors.into_iter().collect(),
                });

                self.push_type(module, union, source)
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
                let payloads_equal =
                    self.decide_equal(origin, form.value, other_form.value)? == Answer::Ready(true);
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
        let origin = self.variables.get(variable)?.origin;
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
        if matches!(widening, Widening::Preserve) {
            return Ok(solution);
        }

        let module = variable.module_id;
        let origin = self.variables.get(variable)?.origin;
        let source = self.origin_source_node(origin)?;

        self.widen_type(module, source, solution)
    }

    /// Widen one closed type, rebuilding literal leaves to their bases.
    pub(in crate::check) fn widen_type(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widening rebuilds the solution tree, dropping freshness and
        // widening literal leaves through aggregates and unions
        match self.widen_tree(module, source, ty, 0)? {
            Some(widened) => Ok(widened),
            None => Ok(ty),
        }
    }

    /// Rebuild one widening solution composite with widened leaves.
    /// Literal leaves widen to their base types, aggregates rebuild
    /// around their widened parts, and unions collapse the duplicates
    /// widening creates. Returns none when nothing widens; the rebuilt
    /// copies allocate new ids, so fresh literals also lose freshness.
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
        let id = self.resolve_root(id)?;

        match self.ty(id)? {
            // literal leaves widen to their base types
            dir::Type::Literal(literal) => {
                let widened = literal.widen();

                Ok(Some(self.push_type(module, widened, source)?))
            }
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
            // tuples widen their element types in place
            dir::Type::Tuple(tuple) => {
                let mut tuple = tuple.clone();
                let mut changed = false;
                for element in &mut tuple.elements {
                    let ty = self.resolve_root(element.ty)?;
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
                    let element = self.resolve_root(element)?;
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

                Ok(Some(self.push_type(
                    module,
                    dir::Type::Union(dir::UnionType { elements: distinct }),
                    source,
                )?))
            }
            // fresh shapes rebuild with widened field types
            dir::Type::Shape(shape) => {
                if !self.is_fresh_literal(id)? {
                    return Ok(None);
                }
                let mut shape = shape.clone();
                for field in &mut shape.fields {
                    let ty = self.resolve_root(field.ty)?;
                    match self.widen_tree(module, source, ty, depth + 1)? {
                        Some(widened) => field.ty = widened,
                        None => field.ty = ty,
                    }
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
}

/// One scalar representation family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScalarKind {
    /// Machine integers of any width.
    Integer,
    /// Machine floats of any width.
    Float,
    /// Arbitrary-precision integers.
    Bigint,
}

impl ScalarKind {
    /// Return the scalar representation family of one type.
    pub(super) fn of(ty: &dir::Type) -> Option<ScalarKind> {
        match ty {
            dir::Type::Literal(literal) => Self::of_literal(literal),
            dir::Type::Primitive(dir::PrimitiveType::Integer(_)) => Some(ScalarKind::Integer),
            dir::Type::Primitive(dir::PrimitiveType::Float(_)) => Some(ScalarKind::Float),
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => Some(ScalarKind::Bigint),
            dir::Type::Range(range) => range
                .start
                .as_ref()
                .or(range.end.as_ref())
                .and_then(Self::of_literal),
            _ => None,
        }
    }

    /// Return the scalar representation family of one literal.
    fn of_literal(literal: &dir::ScalarLiteral) -> Option<ScalarKind> {
        match literal {
            dir::ScalarLiteral::Integer(_) => Some(ScalarKind::Integer),
            dir::ScalarLiteral::Float(_) => Some(ScalarKind::Float),
            dir::ScalarLiteral::Bigint(_) => Some(ScalarKind::Bigint),
            _ => None,
        }
    }
}
