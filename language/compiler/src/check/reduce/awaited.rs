use destack_dir as dir;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Reduce one awaited type through nullish values, unions, and promises.
    pub(super) fn reduce_awaited(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut active = IndexSet::new();

        self.reduce_awaited_guarded(origin, target, &mut active)
    }

    /// Reduce one awaited type with active promise unwrapping tracked.
    fn reduce_awaited_guarded(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        if !active.insert(target) {
            return Ok(Answer::Ready(Some(target)));
        }

        // distribute awaitedness over unions
        if let dir::Type::Union(union) = self.ty(target)?.clone() {
            let module = origin.module();
            let source = self.origin_source_node(origin)?;
            let mut elements = Vec::with_capacity(union.elements.len());

            for element in union.elements {
                let Some(element) = answer!(self.reduce_awaited_guarded(origin, element, active)?)
                else {
                    active.swap_remove(&target);

                    return Ok(Answer::Ready(None));
                };
                elements.push(element);
            }

            let joined = match elements.as_slice() {
                [] => self.push_type(module, dir::Type::Never, source)?,
                [single] => *single,
                _ => self.normalized_union_type(module, elements, source)?,
            };
            active.swap_remove(&target);

            return Ok(Answer::Ready(Some(joined)));
        }

        // preserve nullish values
        match self.ty(target)? {
            dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined) => {
                active.swap_remove(&target);

                return Ok(Answer::Ready(Some(target)));
            }
            _ => {}
        }

        // unwrap compiler-recognized promises
        let instance = match self.ty(target)? {
            dir::Type::Instance(instance) => Some(instance.clone()),
            _ => None,
        };
        let inner = match instance {
            Some(instance)
                if self.language_item(instance.symbol)? == Some(dir::LanguageItem::Promise) =>
            {
                instance.arguments.first().copied()
            }
            _ => None,
        };
        let Some(inner) = inner else {
            active.swap_remove(&target);

            return Ok(Answer::Ready(Some(target)));
        };

        let awaited = self.reduce_awaited_guarded(origin, inner, active)?;

        active.swap_remove(&target);

        Ok(awaited)
    }
}
