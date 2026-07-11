use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Reduce one awaited type through nullish values, unions, and promises.
    pub(super) fn reduce_awaited(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut active = FxIndexSet::default();

        self.reduce_awaited_guarded(origin, target, &mut active)
    }

    /// Reduce one awaited type with active promise unwrapping tracked.
    fn reduce_awaited_guarded(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        if !active.insert(target) {
            return Ok(Answer::Ready(Some(target)));
        }

        let reduced = self.reduce_awaited_active(origin, target, active);
        active.swap_remove(&target);

        reduced
    }

    /// Reduce one active awaited target with its reduced head.
    fn reduce_awaited_active(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // distribute awaitedness over unions
        if let dir::Type::Union(union) = self.ty(target)? {
            let module = origin.module();
            let union_elements: SmallVec<[dir::GlobalTypeId; 8]> =
                SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
            let mut elements = Vec::with_capacity(union_elements.len());

            for element in union_elements {
                let Some(element) = answer!(self.reduce_awaited_guarded(origin, element, active)?)
                else {
                    return Ok(Answer::Ready(None));
                };
                elements.push(element);
            }

            let joined = match elements.as_slice() {
                [] => self.intern_type(module, dir::Type::Never)?,
                [single] => *single,
                _ => self.normalized_union_type(module, elements)?,
            };

            return Ok(Answer::Ready(Some(joined)));
        }

        // preserve nullish values
        if matches!(
            self.ty(target)?,
            dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined)
        ) {
            return Ok(Answer::Ready(Some(target)));
        }

        // unwrap compiler-recognized promises
        let instance = match self.ty(target)? {
            dir::Type::Instance(instance) => Some(instance),
            _ => None,
        };
        let inner = match instance {
            Some(instance)
                if self.language_item(instance.symbol)? == Some(dir::LanguageItem::Promise) =>
            {
                self.type_id_at(target.module_id, instance.arguments, 0)?
            }
            _ => None,
        };
        if let Some(inner) = inner {
            return self.reduce_awaited_guarded(origin, inner, active);
        }

        // newtypes await through their backing
        if let Some(backing) = answer!(self.body(origin.module()).newtype_backing(origin, target)?)
        {
            let backing = answer!(self.reduce_type_head(origin, backing)?);
            let awaited = answer!(self.reduce_awaited_guarded(origin, backing, active)?);

            return match awaited {
                Some(awaited) if awaited != backing => Ok(Answer::Ready(Some(awaited))),
                _ => Ok(Answer::Ready(Some(target))),
            };
        }

        Ok(Answer::Ready(Some(target)))
    }
}
