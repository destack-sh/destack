use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

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
        match self.ty(resolved)? {
            dir::Type::Union(union) => {
                let elements = self.type_ids(resolved.module_id, union.elements)?.to_vec();
                for element in elements {
                    if !rejects(&self.ty(element)?) {
                        kept.push(element);
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
        let elements = self.union_elements(elements)?;

        match elements.as_slice() {
            [single] => Ok(*single),
            _ => {
                let elements = self.intern_type_ids(&elements)?;

                self.intern_type(dir::Type::Union(dir::UnionType { elements }))
            }
        }
    }

    /// Canonicalize one union element through import binders and aliases to one form.
    fn canonical_union_element(
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

        // import the module the resolved symbol lives in
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
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
        let mut kept = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut keys = FxIndexSet::default();
        let mut key_domains = SmallVec::<[dir::PrimitiveType; 2]>::new();
        for element in elements {
            let element = self.shallow_resolve(element)?;
            let element = self.canonical_union_element(element)?;

            // flatten nested unions into one element list
            let elements = match self.ty(element)? {
                dir::Type::Union(union) => SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(element.module_id, union.elements)?,
                ),
                _ => SmallVec::from_slice(&[element]),
            };

            // drop the elements a broader element already covers
            for element in elements {
                // singleton keys deduplicate by identity and their primitive domains
                if let Some(key) = self.static_key_from_type(element)? {
                    let is_covered = !keys.insert(key)
                        || key_domains
                            .iter()
                            .any(|primitive| key.widens_to_primitive(*primitive));
                    if !is_covered {
                        kept.push(element);
                    }

                    continue;
                }

                // skip the elements a kept element already covers or absorbs
                if self.union_contains(&kept, element)? {
                    continue;
                }
                if self.merge_borrowed_union_element(&mut kept, element)? {
                    continue;
                }
                if self.merge_placed_union_element(&mut kept, element)? {
                    continue;
                }

                // keep this element and drop the narrower ones it covers
                self.remove_covered_union_elements(&mut kept, element)?;
                if let dir::Type::Primitive(primitive) = self.ty(element)? {
                    key_domains.push(primitive);
                }
                kept.push(element);
            }
        }

        Ok(kept)
    }

    /// Merge borrows of one payload and access by joining their lifetimes.
    fn merge_borrowed_union_element(
        &mut self,
        kept: &mut SmallVec<[dir::GlobalTypeId; 4]>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Form(form) = self.ty(element)? else {
            return Ok(false);
        };
        let dir::Form::Borrowed(borrow) = form.form else {
            return Ok(false);
        };
        let borrow = self.type_borrow(element.module_id, borrow)?;

        for slot in kept.iter_mut() {
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
            //  distinct spaces keep the union discriminant
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

    /// Merge one placed element into a kept element sharing its place.
    ///
    /// Place representations apply to the whole union, so same-place elements factor
    /// into one representation over the joined values.
    fn merge_placed_union_element(
        &mut self,
        kept: &mut SmallVec<[dir::GlobalTypeId; 4]>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Form(form) = self.ty(element)? else {
            return Ok(false);
        };
        let dir::Form::Managed { place } = form.form else {
            return Ok(false);
        };

        for slot in kept.iter_mut() {
            let dir::Type::Form(existing) = self.ty(*slot)? else {
                continue;
            };
            let dir::Form::Managed {
                place: existing_place,
            } = existing.form
            else {
                continue;
            };
            if existing_place != place {
                continue;
            }
            if existing.value == form.value {
                return Ok(true);
            }

            // factor both values under the shared place
            let joined = self.normalized_union_type([existing.value, form.value])?;
            *slot = self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Managed { place },
                value: joined,
            }))?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Return whether a union element list already covers one type.
    fn union_contains(
        &self,
        kept: &[dir::GlobalTypeId],
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for candidate in kept {
            if self.union_element_covers(*candidate, element)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Remove elements covered by one broader union element.
    fn remove_covered_union_elements(
        &self,
        kept: &mut SmallVec<[dir::GlobalTypeId; 4]>,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let mut index = 0;
        while index < kept.len() {
            if self.union_element_covers(element, kept[index])? {
                kept.remove(index);
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    /// Return whether one union element covers another element.
    fn union_element_covers(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let decision = match (self.ty(source)?, self.ty(target)?) {
            (source, target) if source == target => true,
            (_, dir::Type::Never) => true,
            (target, dir::Type::Literal(literal)) => literal.widens_to(&target),
            (target, dir::Type::Range(range)) => range.widens_to(&target),
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
}
