use destack_dir as dir;

use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, Verdict};

impl CheckState<'_> {
    /// Return the drop hook member one nominal's Drop conformance selects.
    pub(in crate::sema) fn drop_hook_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        if let Some(known) = self.drop_conformers.get(&symbol) {
            return Ok(*known);
        }

        // inspect the declaration's own conformances, then its same-module extensions
        let mut member = match self.definition(symbol)?.cloned() {
            Some(definition) => self.drop_interface_member(definition.implementations())?,
            None => None,
        };
        if member.is_none() {
            let root = dir::TypeRoot::Declaration(symbol);
            let mut extensions = Vec::new();
            let mut is_owner_loaded = true;
            if let Some(module) = self.module_maybe(symbol.module_id) {
                extensions.extend(module.root_extensions(root));
            } else if let Some(external) = self.external_modules.get(&symbol.module_id) {
                extensions.extend(external.definitions.root_extensions(root));
            } else {
                is_owner_loaded = false;
            }
            for extension in extensions {
                let Some(definition) = self.definition(extension)?.cloned() else {
                    continue;
                };
                member = self.drop_interface_member(definition.implementations())?;
                if member.is_some() {
                    break;
                }
            }

            // cache nothing while the owning module stays unloaded
            if !is_owner_loaded {
                return Ok(None);
            }
        }

        self.drop_conformers.insert(symbol, member);

        Ok(member)
    }

    /// Bind one drop hook's owner parameters to a nominal instance's arguments.
    pub(in crate::sema) fn bind_drop_hook(
        &mut self,
        member: dir::GlobalSymbolId,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<Vec<dir::GenericArgumentBinding>>> {
        let Some((owner, parameters)) = self.hook_owner_parameters(member)? else {
            return Ok(Some(Vec::new()));
        };

        // an unparameterized owner closes without arguments
        if parameters.is_empty() {
            return Ok(Some(Vec::new()));
        }
        if parameters.len() != arguments.len() {
            return Ok(None);
        }

        // an inline hook rides the nominal's own bindings
        let is_inline = parameters
            .iter()
            .zip(arguments)
            .all(|(parameter, binding)| *parameter == binding.parameter);
        if is_inline {
            return Ok(Some(arguments.to_vec()));
        }

        // an extension repeating its parameters takes the arguments positionally
        if self.target_repeats_parameters(owner, &parameters)? {
            let bindings = parameters
                .iter()
                .zip(arguments)
                .map(|(parameter, binding)| {
                    dir::GenericArgumentBinding::new(*parameter, binding.argument)
                })
                .collect();

            return Ok(Some(bindings));
        }

        Ok(None)
    }

    /// Return one hook's owner declaration and its non-lifetime parameters.
    fn hook_owner_parameters(
        &mut self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, Vec<dir::GlobalGenericParameterId>)>> {
        let Some(template) = self.symbol_template(member)? else {
            return Ok(None);
        };

        // walk to the enclosing declaration template
        let mut current = self.parent_generic_template(template)?;
        while let Some(id) = current {
            let symbol = self
                .generic_template(id)
                .and_then(|template| template.symbol);
            if let Some(symbol) = symbol
                && self.definition(symbol)?.is_some()
            {
                let parameters = self
                    .generic_template_parameters(id)?
                    .into_iter()
                    .filter(|parameter| !self.is_lifetime_parameter(*parameter))
                    .collect();

                return Ok(Some((symbol, parameters)));
            }
            current = self.parent_generic_template(id)?;
        }

        Ok(None)
    }

    /// Return whether one extension's target repeats its parameters in order.
    fn target_repeats_parameters(
        &mut self,
        owner: dir::GlobalSymbolId,
        parameters: &[dir::GlobalGenericParameterId],
    ) -> CompilerResult<bool> {
        let Some(dir::Definition::Extension(extension)) = self.definition(owner)?.cloned() else {
            return Ok(false);
        };
        let target = extension.target.r#type();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(false);
        };

        // require each target argument to spell the matching parameter
        let arguments = self
            .type_ids(target.module_id, instance.arguments)?
            .to_vec();
        if arguments.len() != parameters.len() {
            return Ok(false);
        }
        for (argument, parameter) in arguments.iter().zip(parameters) {
            if !matches!(self.ty(*argument)?, dir::Type::Parameter(spelled) if spelled == *parameter)
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return the member one conformance list selects for a Drop requirement.
    fn drop_interface_member(
        &mut self,
        implementations: &[dir::NominalConformance],
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        for conformance in implementations {
            let Some(symbol) = self.ty(conformance.interface)?.symbol() else {
                continue;
            };
            if self.language_item(symbol)? != Some(dir::LanguageItem::Drop) {
                continue;
            }

            // select the member satisfying one of the interface's own requirements
            let Some(definition) = self.definition(symbol)?.cloned() else {
                continue;
            };
            let requirements: Vec<_> = definition
                .members()
                .iter()
                .filter_map(|member| match member {
                    dir::DefinitionMember::Method(method) => Some(method.symbol),
                    _ => None,
                })
                .collect();
            let member = conformance
                .members
                .iter()
                .find(|member| requirements.contains(&member.requirement))
                .map(|member| member.member);

            return Ok(member);
        }

        Ok(None)
    }

    /// Decide whether one type runs a drop hook when its owned storage ends.
    pub(in crate::sema) fn decide_drop(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        self.decide_guarded(
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
        // owned payloads drop through their value, views and handles keep theirs
        if let dir::Type::Form(form) = self.ty(ty)? {
            return match form.form {
                dir::Form::Owned => self.decide_drop(origin, form.value, active),
                dir::Form::Managed { .. }
                | dir::Form::Borrowed(_)
                | dir::Form::Raw
                | dir::Form::Readonly => Ok(Verdict::Fails),
            };
        }

        // managed defaults hand their storage to the runtime
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(Verdict::Fails);
        }

        match self.ty(ty)? {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) | dir::Type::Hole(_) => Ok(Verdict::Ambiguous),
            // hooks declared on the nominal drop, else any stored member drops
            dir::Type::Application(instance) => {
                if self.drop_hook_member(instance.symbol)?.is_some() {
                    return Ok(Verdict::Holds);
                }
                let fields = match self.definition(instance.symbol)?.cloned() {
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

                self.decide_any(ids, |state, id| state.decide_drop(origin, id, active))
            }
            // unions drop when any alternative drops
            dir::Type::Union(union) => {
                let ids: Vec<_> = self.type_ids(ty.module_id, union.elements)?.to_vec();

                self.decide_any(ids, |state, id| state.decide_drop(origin, id, active))
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
