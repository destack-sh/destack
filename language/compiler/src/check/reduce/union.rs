use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

/// Non-nullish part of one union type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct NullishSplit {
    /// The type after removing nullish elements.
    pub(in crate::check) value: dir::GlobalTypeId,
    /// The removed nullish part.
    pub(in crate::check) rejected: NullishPart,
}

/// Nullish part removed from one union type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum NullishPart {
    /// Null was removed.
    Null,
    /// Undefined was removed.
    Undefined,
    /// Null and undefined were removed.
    NullOrUndefined,
}

impl NullishPart {
    /// Return the diagnostic label for this nullish part.
    pub(in crate::check) fn label(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Undefined => "undefined",
            Self::NullOrUndefined => "null or undefined",
        }
    }
}

impl CheckState<'_> {
    /// Return a normalized union type.
    pub(in crate::check) fn normalized_union_type(
        &mut self,
        module: ModuleId,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let elements = self.union_elements(elements)?;

        match elements.as_slice() {
            [single] => Ok(*single),
            _ => {
                let elements = self.intern_type_ids(module, &elements)?;

                self.intern_type(module, dir::Type::Union(dir::UnionType { elements }))
            }
        }
    }

    /// Return flattened and deduplicated union elements.
    fn union_elements(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        let mut kept = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for element in elements {
            let element = self.settled_root(element)?;

            // flatten nested unions into one element list
            let elements = match self.ty(element)? {
                dir::Type::Union(union) => SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(element.module_id, union.elements)?,
                ),
                _ => SmallVec::from_slice(&[element]),
            };

            // keep only elements not covered by a broader element
            for element in elements {
                if self.union_contains(&kept, element)? {
                    continue;
                }
                if self.merge_borrowed_union_element(&mut kept, element)? {
                    continue;
                }

                self.remove_covered_union_elements(&mut kept, element)?;
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
        let module = element.module_id;
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
            if existing_borrow.lifetime == borrow.lifetime {
                return Ok(true);
            }

            // one borrow value valid for the join of both lifetimes
            let joined =
                self.normalized_union_type(module, [existing_borrow.lifetime, borrow.lifetime])?;
            let joined_form = self.intern_borrow(module, joined, borrow.access)?;
            *slot = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: joined_form,
                    value: existing.value,
                }),
            )?;

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
            // instances interned by different modules compare by their parts
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                self.type_ids(source.module_id, source_instance.arguments)?
                    == self.type_ids(target.module_id, target_instance.arguments)?
            }
            _ => false,
        };

        Ok(decision)
    }

    /// Return whether two union elements are the same type structurally.
    ///
    /// Instances interned by different modules carry distinct ids for one
    /// structural type, so id equality alone under-deduplicates.
    pub(in crate::check) fn union_element_duplicates(
        &self,
        existing: dir::GlobalTypeId,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if existing == element {
            return Ok(true);
        }

        match (self.ty(existing)?, self.ty(element)?) {
            (existing_head, element_head) if existing_head == element_head => Ok(true),
            (dir::Type::Instance(existing_instance), dir::Type::Instance(element_instance))
                if existing_instance.symbol == element_instance.symbol =>
            {
                Ok(
                    self.type_ids(existing.module_id, existing_instance.arguments)?
                        == self.type_ids(element.module_id, element_instance.arguments)?,
                )
            }
            _ => Ok(false),
        }
    }

    /// Return one union type without nullish elements.
    pub(in crate::check) fn split_nullish_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<NullishSplit>>> {
        let reduced = answer!(self.reduce_type_head(origin, ty)?);
        let dir::Type::Union(union) = self.ty(reduced)? else {
            return Ok(Answer::Ready(None));
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
            return Ok(Answer::Ready(None));
        }

        // collapse the accepted elements back into one type
        let value = match non_nullish.as_slice() {
            [] => self.intern_type(ty.module_id, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(ty.module_id, non_nullish)?,
        };
        let rejected = match (has_null, has_undefined) {
            (true, true) => NullishPart::NullOrUndefined,
            (true, false) => NullishPart::Null,
            (false, true) => NullishPart::Undefined,
            (false, false) => unreachable!("nullish split requires a nullish element"),
        };

        Ok(Answer::Ready(Some(NullishSplit { value, rejected })))
    }

    /// Distribute one reduction across union arms.
    pub(super) fn reduce_distributed_operation<F>(
        &mut self,
        origin: Origin,
        elements: SmallVec<[dir::GlobalTypeId; 4]>,
        mut reduce: F,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>>
    where
        F: FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>>,
    {
        let module = origin.module();
        let mut reduced = Vec::with_capacity(elements.len());

        // reduce each arm independently
        for element in elements {
            let Some(element) = answer!(reduce(self, element)?) else {
                return Ok(Answer::Ready(None));
            };
            reduced.push(element);
        }

        let union = self.normalized_union_type(module, reduced)?;

        Ok(Answer::Ready(Some(union)))
    }
}
