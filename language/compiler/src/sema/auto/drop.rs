use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, Verdict};

impl CheckState<'_> {
    /// Return whether one nominal or one of its extensions declares the Drop conformance.
    pub(in crate::sema) fn declares_drop(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        for conformer in self.nominal_conformers(symbol)? {
            if self.drop_conformance(conformer)?.is_some() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the drop hook member one nominal's Drop conformance selects.
    pub(in crate::sema) fn drop_hook_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // serve the memo
        if let Some(known) = self.drop_hooks.get(&symbol) {
            return Ok(*known);
        }

        // select the member the first Drop conformance answers the requirement with
        let mut member = None;
        for conformer in self.nominal_conformers(symbol)? {
            let Some(conformance) = self.drop_conformance(conformer)? else {
                continue;
            };
            member = self.drop_requirement_member(&conformance)?;
            break;
        }
        self.drop_hooks.insert(symbol, member);

        Ok(member)
    }

    /// Return the conformers of one nominal: itself and its root extensions.
    pub(in crate::sema) fn nominal_conformers(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        let root = dir::TypeRoot::Declaration(symbol);
        let mut conformers = vec![symbol];
        if let Some(module) = self.module_maybe(symbol.module_id) {
            conformers.extend(module.root_extensions(root));
        } else if let Some(external) = self.external(symbol.module_id)? {
            conformers.extend(external.definitions().root_extensions(root));
        }

        Ok(conformers)
    }

    /// Return the conformance one declaration writes naming the Drop interface.
    fn drop_conformance(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::NominalConformance>> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(None);
        };
        for conformance in definition.implementations() {
            let Some(interface) = self.ty(conformance.interface)?.symbol() else {
                continue;
            };
            if self.language_item(interface)? == Some(dir::LanguageItem::Drop) {
                return Ok(Some(conformance.clone()));
            }
        }

        Ok(None)
    }

    /// Return the member one Drop conformance selects for the interface's requirement.
    fn drop_requirement_member(
        &mut self,
        conformance: &dir::NominalConformance,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(interface) = self.ty(conformance.interface)?.symbol() else {
            return Ok(None);
        };
        let Some(definition) = self.definition(interface)? else {
            return Ok(None);
        };
        let requirements: Vec<_> = definition
            .members()
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::Method(method) => Some(method.symbol),
                _ => None,
            })
            .collect();
        let member = self
            .conformance_members(conformance.source)?
            .into_iter()
            .find(|member| requirements.contains(&member.requirement))
            .map(|member| member.member);

        Ok(member)
    }

    /// Decide whether one type runs a drop hook when its owned storage ends.
    pub(in crate::sema) fn decide_drop(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        self.decide_recorded(
            origin,
            ty,
            dir::AutoInterface::Drop,
            active,
            |state, ty, active| state.decide_drop_type(origin, ty, active),
        )
    }

    /// Decide the drop requirement for one active type.
    fn decide_drop_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // drop through an owned form, stopping at every other form
        if let dir::Type::Form(form) = self.ty(ty)? {
            return match form.form {
                dir::Form::Owned => self.decide_drop(origin, form.value, active),
                dir::Form::Borrowed(_) | dir::Form::Raw | dir::Form::Readonly => Ok(Verdict::Fails),
            };
        }

        // managed defaults hand their storage to the runtime
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(Verdict::Fails);
        }

        // decide by the value's own storage
        match self.ty(ty)? {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // hooks declared on the nominal drop, else any stored member drops
            dir::Type::Application(instance) => {
                if self.declares_drop(instance.symbol)? {
                    return Ok(Verdict::Holds);
                }
                let fields = match self.definition(instance.symbol)?.as_deref() {
                    Some(definition) => self.stored_field_types(definition.members())?,
                    None => SmallVec::new(),
                };

                self.decide_any_applied(ty.module_id, &instance, fields, |state, id| {
                    state.decide_drop(origin, id, active)
                })
            }
            // fixed arrays drop through their element
            dir::Type::FixedArray(array) => self.decide_drop(origin, array.element, active),
            // tuples drop when any element drops
            dir::Type::Tuple(tuple) => {
                let ids: Vec<_> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_any(ids.iter().copied(), |state, id| {
                    state.decide_drop(origin, id, active)
                })
            }
            // unions drop when any alternative drops
            dir::Type::Union(union) => {
                let ids = self.type_ids(ty.module_id, union.elements)?;

                self.decide_any(ids.iter().copied(), |state, id| {
                    state.decide_drop(origin, id, active)
                })
            }
            // refinements drop through their base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_drop(origin, refined.base, active)
            }
            // variants drop through their owning enum
            dir::Type::Variant(member) => self.decide_drop(origin, member.owner, active),
            // every remaining representation stores no hook
            _ => Ok(Verdict::Fails),
        }
    }
}
