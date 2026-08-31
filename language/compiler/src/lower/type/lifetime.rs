use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::LowerState;
use crate::{CompilerError, CompilerResult};

/// The lifetime slots declared for generic lifetime parameters.
#[derive(Clone, Default)]
pub(in crate::lower) struct LifetimeParameters {
    /// The function-local slot of each lifetime parameter.
    pub(in crate::lower) slots: FxIndexMap<dir::GlobalGenericParameterId, mir::LifetimeSlot>,
    /// The declared name of each slot, without the tick.
    pub(in crate::lower) names: Vec<String>,
    /// The declared outlives pairs between slots, left outliving right.
    pub(in crate::lower) outlives: Vec<(mir::LifetimeSlot, mir::LifetimeSlot)>,
}

impl LifetimeParameters {
    /// Collect the lifetime slots declared by one generic template.
    pub(in crate::lower) fn from_template(
        lower: &LowerState<'_>,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<Self> {
        // give every declared lifetime parameter its own slot and printed name
        let generics = &lower.state(template.module_id)?.generics;
        let declared = generics.get_template(template.local_id);
        let mut parameters = Self::default();
        for parameter in &declared.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Region) {
                let slot = mir::LifetimeSlot(parameters.slots.len() as u32);
                let name = match binding.key {
                    dir::GenericParameterKey::Symbol(symbol) => lower.symbol_name(symbol)?,
                    dir::GenericParameterKey::Generated(name) => Some(name),
                };

                let name = match name {
                    // keep tick names verbatim and add a tick to bare names
                    Some(name) => {
                        let name = lower.strings.get(name);
                        match name.starts_with('\'') {
                            true => name.to_string(),
                            false => format!("'{name}"),
                        }
                    }
                    None => format!("'l{}", slot.0),
                };
                parameters.names.push(name);
                parameters
                    .slots
                    .insert(parameter.into_global(template.module_id), slot);
            }
        }

        // carry declared outlives predicates between the collected slots
        for predicate in &declared.predicates {
            if predicate.relation != dir::WhereRelation::Satisfies {
                continue;
            }

            let left = parameters.parameter_slot(lower, predicate.left)?;
            let right = parameters.parameter_slot(lower, predicate.right)?;
            if let (Some(left), Some(right)) = (left, right) {
                parameters.outlives.push((left, right));
            }
        }

        Ok(parameters)
    }

    /// Return the slot of one type when it names a collected lifetime parameter.
    fn parameter_slot(
        &self,
        lower: &LowerState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::LifetimeSlot>> {
        let dir::Type::Parameter(parameter) = lower.ty(ty)? else {
            return Ok(None);
        };

        Ok(self.slots.get(&parameter).copied())
    }

    /// Return the outlives slots declared for one slot.
    fn slot_outlives(&self, slot: mir::LifetimeSlot) -> Vec<mir::LifetimeSlot> {
        self.outlives
            .iter()
            .filter(|(left, _)| *left == slot)
            .map(|(_, right)| *right)
            .collect()
    }

    /// Declare these lifetime slots on one function header.
    pub(in crate::lower) fn declare<'a>(
        &self,
        mut header: mir::FunctionHeaderBuilder<'a>,
    ) -> mir::FunctionHeaderBuilder<'a> {
        for (slot, name) in self.names.iter().enumerate() {
            let outlives = self.slot_outlives(mir::LifetimeSlot(slot as u32));
            header = header.lifetime_outlives(name, outlives);
        }

        header
    }

    /// Return declarations for these lifetime slots.
    pub(in crate::lower) fn declarations(
        &self,
        strings: &destack_core::StringPool,
    ) -> Vec<mir::LifetimeParameter> {
        self.names
            .iter()
            .enumerate()
            .map(|(slot, name)| {
                let name = strings.intern(name);
                let outlives = self.slot_outlives(mir::LifetimeSlot(slot as u32));

                mir::LifetimeParameter::with_outlives(Some(name), outlives)
            })
            .collect()
    }
}

impl LowerState<'_> {
    /// Lower one lifetime into the current lifetime environment.
    pub(in crate::lower) fn lower_lifetime(
        &self,
        lifetime: dir::GlobalTypeId,
        parameters: &LifetimeParameters,
    ) -> CompilerResult<mir::Lifetime> {
        match self.ty(lifetime)? {
            // read the extent of a region pair
            dir::Type::Region(region) => self.lower_lifetime(region.extent, parameters),
            // resolve declared slots and erase parameters outside the scope
            dir::Type::Parameter(parameter) => Ok(parameters
                .slots
                .get(&parameter)
                .map(|slot| mir::Lifetime::slot(slot.0))
                .unwrap_or_default()),
            // retain every provenance term of a lifetime union
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
            _ => match self
                .memory_text(lifetime)?
                .and_then(dir::Lifetime::from_text)
            {
                Some(dir::Lifetime::Static) => Ok(mir::Lifetime::static_storage()),
                Some(dir::Lifetime::Frame) => Ok(mir::Lifetime::frame()),
                None => Err(CompilerError::Internal {
                    message: "an unknown lifetime value".to_string(),
                }),
            },
        }
    }
}
