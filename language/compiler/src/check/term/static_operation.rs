use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckComponentState, GenericSubstitution, VariableId};

/// Static value operation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticOperationTerm {
    /// Join values in a static lattice.
    Join {
        /// The lattice being joined.
        domain: StaticJoinDomain,
        /// The joined values.
        elements: Vec<VariableId>,
    },
}

/// Static lattice with a defined join operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticJoinDomain {
    /// Borrow lifetime lattice.
    Lifetime,
}

impl StaticOperationTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Join {
                domain: _,
                elements,
            } => elements.iter().copied().collect(),
        }
    }

    /// Substitute generic arguments through one static operation.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Self> {
        let operation = match self {
            Self::Join { domain, elements } => Self::Join {
                domain: *domain,
                elements: state.substitute_static_variables(module, substitution, elements)?,
            },
        };

        Ok(operation)
    }
}

impl CheckComponentState<'_> {
    /// Reduce one static operation when its operands are solved.
    pub(in crate::check) fn reduce_static_operation(
        &mut self,
        module: ModuleId,
        operation: &StaticOperationTerm,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        match operation {
            StaticOperationTerm::Join {
                domain: StaticJoinDomain::Lifetime,
                elements,
            } => self.reduce_lifetime_join(module, elements),
        }
    }

    /// Reduce one lifetime join.
    fn reduce_lifetime_join(
        &mut self,
        module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let mut lifetimes = Vec::with_capacity(elements.len());

        // collect solved lifetime element ids
        for element in elements {
            let Some(term) = self.static_value(*element)? else {
                return Ok(None);
            };
            let dir::StaticTerm::Lifetime { .. } = term else {
                return Ok(None);
            };
            let lifetime = self.module_mut(module)?.intern_static(term);

            lifetimes.push(lifetime);
        }

        Ok(Some(dir::StaticTerm::Lifetime {
            lifetime: dir::Lifetime::Join(lifetimes),
        }))
    }
}
