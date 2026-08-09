use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Origin};

impl CheckState<'_> {
    /// Return the completed value carried by one async function result type.
    pub(in crate::check) fn async_completion_type(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut active = FxIndexSet::default();

        self.async_completion_type_guarded(target, &mut active)
    }

    /// Resolve one async completion type while guarding transparent recursion.
    fn async_completion_type_guarded(
        &mut self,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.shallow_resolve(target)?;
        if !active.insert(target) {
            return Ok(None);
        }

        // recognize the two compiler-owned async result carriers
        if let dir::Type::Application(instance) = self.ty(target)?
            && matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::Promise | dir::LanguageItem::Task)
            )
        {
            let completed = self.type_id_at(target.module_id, instance.arguments, 0)?;
            active.swap_remove(&target);

            return Ok(completed);
        }

        // preserve transparent aliases and nominal wrappers around an owner
        let backing = if let dir::Type::Application(instance) = self.ty(target)? {
            let definition = self.definition(instance.symbol)?.cloned();
            let declared = match definition {
                Some(dir::Definition::TypeAlias(definition)) => Some(definition.value),
                Some(dir::Definition::Newtype(definition)) => Some(definition.backing),
                _ => None,
            };
            if let Some(declared) = declared {
                let substitution = self.instance_substitution(target.module_id, &instance)?;

                Some(self.substitute_type(declared, &substitution)?)
            } else {
                None
            }
        } else {
            None
        };
        let completed = match backing {
            Some(backing) => self.async_completion_type_guarded(backing, active)?,
            None => None,
        };
        active.swap_remove(&target);

        Ok(completed)
    }

    /// Reduce one awaited type through nullish values, unions, and async carriers.
    pub(super) fn reduce_awaited(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut active = FxIndexSet::default();

        self.reduce_awaited_guarded(origin, target, &mut active)
    }

    /// Reduce one awaited type with active carrier unwrapping tracked.
    fn reduce_awaited_guarded(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.reduce_type_head(origin, target)?;
        if !active.insert(target) {
            return Ok(Some(target));
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
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // distribute awaitedness over unions
        if let dir::Type::Union(union) = self.ty(target)? {
            let union_elements: SmallVec<[dir::GlobalTypeId; 8]> =
                SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
            let mut elements = Vec::with_capacity(union_elements.len());

            for element in union_elements {
                let Some(element) = self.reduce_awaited_guarded(origin, element, active)? else {
                    return Ok(None);
                };
                elements.push(element);
            }

            let joined = match elements.as_slice() {
                [] => self.intern_type(dir::Type::Never)?,
                [single] => *single,
                _ => self.normalized_union_type(elements)?,
            };

            return Ok(Some(joined));
        }

        // preserve nullish values
        if matches!(
            self.ty(target)?,
            dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined)
        ) {
            return Ok(Some(target));
        }

        // unwrap compiler-recognized async result carriers
        let instance = match self.ty(target)? {
            dir::Type::Application(instance) => Some(instance),
            _ => None,
        };
        let inner = match instance {
            Some(instance)
                if matches!(
                    self.language_item(instance.symbol)?,
                    Some(dir::LanguageItem::Promise | dir::LanguageItem::Task)
                ) =>
            {
                self.type_id_at(target.module_id, instance.arguments, 0)?
            }
            _ => None,
        };
        if let Some(inner) = inner {
            return self.reduce_awaited_guarded(origin, inner, active);
        }

        // newtypes await through their backing
        if let Some(instance) = self.decompose_newtype(origin, target)? {
            let backing = instance.backing;
            let backing = self.reduce_type_head(origin, backing)?;
            let awaited = self.reduce_awaited_guarded(origin, backing, active)?;

            return match awaited {
                Some(awaited) if awaited != backing => Ok(Some(awaited)),
                _ => Ok(Some(target)),
            };
        }

        Ok(Some(target))
    }
}
