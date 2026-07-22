use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// MIR lifetime slots declared for DIR lifetime parameters.
#[derive(Clone, Default)]
pub(in crate::lower) struct LifetimeParameters {
    /// The function-local MIR slot of each lifetime parameter.
    pub(in crate::lower) slots: FxIndexMap<dir::GlobalGenericParameterId, mir::LifetimeSlot>,
}

impl LifetimeParameters {
    /// Collect the lifetime slots declared by one generic template.
    pub(in crate::lower) fn from_template(
        lowerer: &ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<Self> {
        let generics = &lowerer.state(template.module_id)?.generics;
        let template_row = generics.get_template(template.local_id);
        let mut parameters = Self::default();
        for parameter in &template_row.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime) {
                let slot = mir::LifetimeSlot(parameters.slots.len() as u32);
                parameters
                    .slots
                    .insert(parameter.into_global(template.module_id), slot);
            }
        }

        Ok(parameters)
    }

    /// Declare these lifetime slots on one MIR function header.
    pub(in crate::lower) fn declare<'a>(
        &self,
        mut header: mir::FunctionHeaderBuilder<'a>,
    ) -> mir::FunctionHeaderBuilder<'a> {
        for slot in 0..self.slots.len() {
            header = header.lifetime(&format!("L{slot}"));
        }

        header
    }

    /// Return MIR declarations for these lifetime slots.
    pub(in crate::lower) fn declarations(
        &self,
        strings: &destack_core::StringPool,
    ) -> Vec<mir::LifetimeParameter> {
        (0..self.slots.len())
            .map(|slot| {
                let name = strings.intern(&format!("L{slot}"));

                mir::LifetimeParameter::new(Some(name))
            })
            .collect()
    }
}

impl ModuleLowerer<'_> {
    /// Lower one sealed lifetime into the current MIR lifetime environment.
    pub(in crate::lower) fn lower_lifetime(
        &self,
        lifetime: dir::GlobalTypeId,
        parameters: &LifetimeParameters,
    ) -> CompilerResult<mir::Lifetime> {
        let lifetime = self.reduced_type(lifetime)?;

        match self.ty(lifetime)? {
            // lifetime parameters resolve to their declared slot
            dir::Type::Parameter(parameter) => {
                let Some(slot) = parameters.slots.get(&parameter) else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR left a lifetime parameter outside its scope"
                            .to_string(),
                    });
                };

                Ok(mir::Lifetime::slot(slot.0))
            }
            // lifetime unions retain every possible provenance term
            dir::Type::Union(union) => {
                let elements = self
                    .types(lifetime.module_id)?
                    .type_ids(union.elements)
                    .to_vec();
                let mut terms = Vec::new();
                for element in elements {
                    let lifetime = self.lower_lifetime(element, parameters)?;
                    terms.extend(lifetime.terms);
                }

                Ok(mir::Lifetime::new(terms))
            }
            // lower concrete lifetime values
            _ => match self.memory_literal(lifetime, dir::MemoryParameter::Lifetime)? {
                dir::MemoryLiteral::Lifetime(dir::Lifetime::Static) => {
                    Ok(mir::Lifetime::static_storage())
                }
                dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame) => Ok(mir::Lifetime::empty()),
                _ => Err(CompilerError::Internal {
                    message: "checked DIR decoded a lifetime in the wrong domain".to_string(),
                }),
            },
        }
    }
}
