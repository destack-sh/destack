use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

/// The depth same type comparison descends into applications, forms, and unions.
const SAME_TYPE_DEPTH: u32 = 8;

/// One union type split around its nullish elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct NullishSplit {
    /// The type after removing nullish elements.
    pub(in crate::sema) value: dir::GlobalTypeId,
    /// The removed nullish part.
    pub(in crate::sema) rejected: NullishPart,
}

/// Nullish part removed from one union type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum NullishPart {
    /// Null alone.
    Null,
    /// Undefined alone.
    Undefined,
    /// Null and undefined together.
    NullOrUndefined,
}

impl NullishPart {
    /// Return the diagnostic label for this nullish part.
    pub(in crate::sema) fn label(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Undefined => "undefined",
            Self::NullOrUndefined => "null or undefined",
        }
    }
}

impl CheckState<'_> {
    /// Rebuild one union without the members a rejecting position strips.
    pub(in crate::sema) fn without_union_members(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        rejects: impl Fn(&dir::Type) -> bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // resolve named heads before matching the rejected members
        let resolved = self.structurally_normalize(origin, ty)?;

        // keep the members the position accepts
        let mut kept = Vec::new();
        // collect the members by the head of the value
        match self.ty(resolved)? {
            dir::Type::Union(union) => {
                let elements = self.type_ids(resolved.module_id, union.elements)?;
                for element in elements {
                    if !rejects(&self.ty(*element)?) {
                        kept.push(*element);
                    }
                }
            }
            member if rejects(&member) => {}
            _ => return Ok(ty),
        }

        self.normalized_union_type(kept)
    }

    /// Return a normalized union type.
    pub(in crate::sema) fn normalized_union_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep the arms in declaration order, the physical case order every instance inherits
        let elements = self.union_elements(elements)?;

        // join the deduplicated elements
        match elements.as_slice() {
            [single] => Ok(*single),
            _ => {
                let elements = self.intern_type_ids(&elements)?;

                self.intern_type(dir::Type::Union(dir::UnionType { elements }))
            }
        }
    }

    /// Canonicalize one union application through import binders and aliases to one form.
    fn canonical_union_application(
        &mut self,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Application(instance) = self.ty(element)? else {
            return Ok(element);
        };

        // resolve the applied symbol to its declaration
        let mut symbol = instance.symbol;
        loop {
            let resolved = self.resolve_symbol_alias(symbol)?;
            let resolved = match self.import_binder_target(resolved)? {
                Some(target) => target,
                None => resolved,
            };
            if resolved == symbol {
                break;
            }
            symbol = resolved;
        }

        // keep the element whose symbol resolved to itself
        if symbol == instance.symbol {
            return Ok(element);
        }

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments: instance.arguments,
        }))
    }

    /// Return flattened and deduplicated union elements.
    fn union_elements(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        // keep each element beside whether it is a singleton key
        let mut kept = SmallVec::<[(dir::GlobalTypeId, bool); 4]>::new();
        let mut keys = FxIndexSet::default();
        let mut key_domains = SmallVec::<[dir::PrimitiveType; 2]>::new();
        for element in elements {
            let element = self.shallow_resolve(element)?;
            let element = self.canonical_union_application(element)?;

            // flatten nested unions into one element list
            let elements = match self.ty(element)? {
                dir::Type::Union(union) => SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(element.module_id, union.elements)?,
                ),
                _ => SmallVec::from_slice(&[element]),
            };

            // drop the elements a broader element already covers
            for element in elements {
                let element = self.shallow_resolve(element)?;

                // deduplicate singleton keys by identity, primitive domain, and broader elements
                if let Some(key) = self.static_key_from_type(element)? {
                    let broader = kept.iter().filter(|(_, is_key)| !is_key);
                    let is_covered = !keys.insert(key)
                        || key_domains
                            .iter()
                            .any(|primitive| key.widens_to_primitive(*primitive))
                        || self.union_contains(broader.map(|(kept, _)| *kept), element)?;
                    if !is_covered {
                        kept.push((element, true));
                    }

                    continue;
                }

                // skip the elements a kept element already covers or absorbs
                if self.union_contains(kept.iter().map(|(kept, _)| *kept), element)? {
                    continue;
                }
                if self.merge_borrowed_union_element(&mut kept, element)? {
                    continue;
                }

                // keep this element and drop the narrower ones it covers
                self.remove_covered_union_elements(&mut kept, element)?;
                if let dir::Type::Primitive(primitive) = self.ty(element)? {
                    key_domains.push(primitive);
                }
                kept.push((element, false));
            }
        }

        // drop the key flags
        let mut kept = kept
            .into_iter()
            .map(|(element, _)| element)
            .collect::<SmallVec<[dir::GlobalTypeId; 4]>>();

        // both boolean literals together are the boolean primitive
        let mut booleans = [false, false];
        for element in &kept {
            if let dir::Type::Literal(dir::Literal::Boolean(value)) = self.ty(*element)? {
                booleans[value as usize] = true;
            }
        }
        if booleans == [true, true] {
            let mut collapsed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
            for element in kept {
                match self.ty(element)? {
                    dir::Type::Literal(dir::Literal::Boolean(_)) => {
                        if !collapsed.contains(&boolean) {
                            collapsed.push(boolean);
                        }
                    }
                    _ => collapsed.push(element),
                }
            }
            kept = collapsed;
        }

        Ok(kept)
    }

    /// Merge borrows of one payload and access by joining their lifetimes.
    fn merge_borrowed_union_element(
        &mut self,
        kept: &mut SmallVec<[(dir::GlobalTypeId, bool); 4]>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Form(form) = self.ty(element)? else {
            return Ok(false);
        };
        let dir::Form::Borrowed(borrow) = form.form else {
            return Ok(false);
        };
        let borrow = self.type_borrow(element.module_id, borrow)?;

        // layer the borrow over each kept arm
        for (slot, _) in kept.iter_mut() {
            let dir::Type::Form(existing) = self.ty(*slot)? else {
                continue;
            };
            let dir::Form::Borrowed(existing_borrow) = existing.form else {
                continue;
            };
            let existing_borrow = self.type_borrow(slot.module_id, existing_borrow)?;
            if existing.value != form.value || existing_borrow.access != borrow.access {
                continue;
            }
            if existing_borrow.region == borrow.region {
                return Ok(true);
            }

            // factor borrows sharing one monomorphic space by joining their extents
            let existing_region = self.shallow_resolve(existing_borrow.region)?;
            let region = self.shallow_resolve(borrow.region)?;
            let (dir::Type::Region(existing_pair), dir::Type::Region(pair)) =
                (self.ty(existing_region)?, self.ty(region)?)
            else {
                continue;
            };
            if existing_pair.space != pair.space {
                continue;
            }
            let extent = self.normalized_union_type([existing_pair.extent, pair.extent])?;
            let joined = self.intern_region(extent, pair.space)?;
            let joined_form = self.intern_borrow(joined, borrow.access)?;
            *slot = self.intern_type(dir::Type::Form(dir::FormType {
                form: joined_form,
                value: existing.value,
            }))?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Return whether a union element list already covers one type.
    fn union_contains(
        &self,
        kept: impl IntoIterator<Item = dir::GlobalTypeId>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for candidate in kept {
            if self.union_element_covers(candidate, element)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Remove elements covered by one broader union element.
    fn remove_covered_union_elements(
        &self,
        kept: &mut SmallVec<[(dir::GlobalTypeId, bool); 4]>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let mut index = 0;
        while index < kept.len() {
            if self.union_element_covers(element, kept[index].0)? {
                kept.remove(index);
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    /// Return whether two types are the same type by content.
    pub(in crate::sema) fn is_same_type(
        &self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        self.is_same_type_at(left, right, 0)
    }

    /// Return whether two types are the same type down to one depth.
    fn is_same_type_at(
        &self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        depth: u32,
    ) -> CompilerResult<bool> {
        let left = self.shallow_resolve(left)?;
        let right = self.shallow_resolve(right)?;
        if left == right {
            return Ok(true);
        }
        if depth >= SAME_TYPE_DEPTH {
            return Ok(false);
        }
        Ok(match (self.ty(left)?, self.ty(right)?) {
            (dir::Type::Application(left_instance), dir::Type::Application(right_instance)) => {
                left_instance.symbol == right_instance.symbol
                    && self.are_same_type_lists(
                        left.module_id,
                        left_instance.arguments,
                        right.module_id,
                        right_instance.arguments,
                        depth,
                    )?
            }
            (dir::Type::Reference(left_reference), dir::Type::Reference(right_reference)) => {
                left_reference.symbol == right_reference.symbol
                    && self.are_same_type_lists(
                        left.module_id,
                        left_reference.arguments,
                        right.module_id,
                        right_reference.arguments,
                        depth,
                    )?
            }
            (dir::Type::Form(left_form), dir::Type::Form(right_form)) => {
                left_form.form == right_form.form
                    && self.is_same_type_at(left_form.value, right_form.value, depth + 1)?
            }
            (dir::Type::Union(left_union), dir::Type::Union(right_union)) => self
                .are_same_type_lists(
                    left.module_id,
                    left_union.elements,
                    right.module_id,
                    right_union.elements,
                    depth,
                )?,
            (left, right) => left == right,
        })
    }

    /// Return whether two type lists hold the same types in order.
    fn are_same_type_lists(
        &self,
        left_module: ModuleId,
        left: dir::TypeListId,
        right_module: ModuleId,
        right: dir::TypeListId,
        depth: u32,
    ) -> CompilerResult<bool> {
        let left = self.type_ids(left_module, left)?;
        let right = self.type_ids(right_module, right)?;
        if left.len() != right.len() {
            return Ok(false);
        }
        for (left, right) in left.iter().zip(right.iter()) {
            if !self.is_same_type_at(*left, *right, depth + 1)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one union element covers another element.
    fn union_element_covers(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.is_same_type(source, target)? {
            return Ok(true);
        }
        let decision = match (self.ty(source)?, self.ty(target)?) {
            (_, dir::Type::Never) => true,
            // a readonly view covers the literals its value covers
            (dir::Type::Form(form), dir::Type::Literal(_)) if form.form == dir::Form::Readonly => {
                return self.union_element_covers(form.value, target);
            }
            (target, dir::Type::Literal(literal)) => literal.widens_to(&target),
            (target, dir::Type::Range(range)) => range.widens_to(&target),
            // cover each variant of an enum
            (_, dir::Type::Variant(variant)) => self.is_same_type(source, variant.owner)?,
            _ => false,
        };

        Ok(decision)
    }

    /// Return one union type without nullish elements.
    pub(in crate::sema) fn split_nullish_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<NullishSplit>> {
        let reduced = self.normalize(origin, ty)?;
        let dir::Type::Union(union) = self.ty(reduced)? else {
            return Ok(None);
        };

        // partition the union into accepted and rejected elements
        let elements =
            SmallVec::<[_; 4]>::from_slice(self.type_ids(reduced.module_id, union.elements)?);
        let mut has_null = false;
        let mut has_undefined = false;
        let mut non_nullish = Vec::with_capacity(elements.len());
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null => has_null = true,
                dir::Type::Undefined => has_undefined = true,
                _ => non_nullish.push(element),
            }
        }
        if !has_null && !has_undefined {
            return Ok(None);
        }

        // collapse the accepted elements back into one type
        let value = match non_nullish.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(non_nullish)?,
        };
        let rejected = match (has_null, has_undefined) {
            (true, true) => NullishPart::NullOrUndefined,
            (true, false) => NullishPart::Null,
            (false, true) => NullishPart::Undefined,
            (false, false) => unreachable!("nullish split requires a nullish element"),
        };

        Ok(Some(NullishSplit { value, rejected }))
    }

    /// Distribute one reduction across union arms.
    pub(super) fn reduce_distributed_operation<F>(
        &mut self,
        _origin: Origin,
        elements: SmallVec<[dir::GlobalTypeId; 4]>,
        mut reduce: F,
    ) -> CompilerResult<Option<dir::GlobalTypeId>>
    where
        F: FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Option<dir::GlobalTypeId>>,
    {
        let mut reduced = Vec::with_capacity(elements.len());

        // reduce each arm independently
        for element in elements {
            let Some(element) = reduce(self, element)? else {
                return Ok(None);
            };
            reduced.push(element);
        }

        let union = self.normalized_union_type(reduced)?;

        Ok(Some(union))
    }

    /// Resolve one selected member to its canonical leaf in the union's flat view.
    pub(in crate::sema) fn canonical_union_leaf(
        &mut self,
        origin: Origin,
        union: dir::GlobalTypeId,
        member: dir::GlobalTypeId,
        site: &'static str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // resolve against the bare union head the member lists are recorded on
        let union = self.form_chain(origin, union)?.base();
        let Some(leaves) = self.union_leaves(origin, union)? else {
            return Err(CompilerError::Internal {
                message: format!("{site} canonicalizing a member outside a union"),
            });
        };

        // match the member exactly first
        if leaves.contains(&member) {
            return Ok(member);
        }

        // match the member stored beneath its enclosing forms, canonicalized like the leaves
        let base = self.form_chain(origin, member)?.base();
        let base = self.canonical_union_application(base)?;
        if leaves.contains(&base) {
            return Ok(base);
        }

        // match the member against each leaf, both normalized and resolved through their solutions
        let normal = self.normalize(origin, base)?;
        let resolved = self.fully_resolve(normal)?;
        for leaf in leaves {
            let leaf_normal = self.normalize(origin, leaf)?;
            if leaf_normal == normal || self.fully_resolve(leaf_normal)? == resolved {
                return Ok(leaf);
            }
        }

        Err(CompilerError::Internal {
            message: format!(
                "{site} selecting the member '{}' outside the canonical leaves of '{}'",
                self.format_type(member),
                self.format_type(union)
            ),
        })
    }

    /// Return one union target's members in their canonical flat order.
    pub(in crate::sema) fn canonical_union_members(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // reuse the memoized flattening for closed targets
        let flags = self.type_flags(target)?;
        let is_closed = !flags.has_variable() && !flags.has_infer() && !flags.has_error();
        if is_closed && let Some(members) = self.canonical_unions.get(&target) {
            return Ok(members.clone());
        }

        let Some(leaves) = self.union_leaves(origin, target)? else {
            if is_closed {
                self.canonical_unions.insert(target, None);
            }

            return Ok(None);
        };

        // keep the members in declaration order, each once
        let mut members = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for member in leaves {
            if !members.contains(&member) {
                members.push(member);
            }
        }
        if is_closed {
            self.canonical_unions.insert(target, Some(members.clone()));
        }

        Ok(Some(members))
    }
}
