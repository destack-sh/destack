use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericSubstitution, Progress, StaticOperand, StaticRelation, VariableId,
};

/// Check-local memory form term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum FormTerm {
    /// Managed value form.
    Managed,
    /// Owned value form.
    Owned,
    /// Borrowed value form.
    Borrowed {
        /// The solved lifetime value.
        lifetime: StaticOperand,
        /// The solved access value.
        access: StaticOperand,
    },
    /// Raw pointer form.
    Raw,
    /// Placed value form.
    Placed {
        /// The solved place value.
        place: StaticOperand,
    },
    /// Readonly view form.
    Readonly,
}

impl FormTerm {
    /// Return whether two forms share the same constructor.
    pub(in crate::check) fn same_constructor(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Owned, Self::Owned)
                | (Self::Managed, Self::Managed)
                | (Self::Borrowed { .. }, Self::Borrowed { .. },)
                | (Self::Raw, Self::Raw)
                | (Self::Placed { .. }, Self::Placed { .. })
                | (Self::Readonly, Self::Readonly)
        )
    }

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Borrowed { lifetime, access } => {
                variables.extend(lifetime.referenced_variables(state));
                variables.extend(access.referenced_variables(state));
            }
            Self::Placed { place } => variables.extend(place.referenced_variables(state)),
            Self::Managed | Self::Owned | Self::Raw | Self::Readonly => {}
        }

        variables
    }

    /// Substitute generic arguments through this memory form.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let form = match self {
            Self::Borrowed { lifetime, access } => Self::Borrowed {
                lifetime: state.substitute_static_operand(module, substitution, *lifetime)?,
                access: state.substitute_static_operand(module, substitution, *access)?,
            },
            Self::Placed { place } => Self::Placed {
                place: state.substitute_static_operand(module, substitution, *place)?,
            },
            Self::Managed | Self::Owned | Self::Raw | Self::Readonly => self.clone(),
        };

        Ok(form)
    }
}

impl CheckState<'_> {
    /// Decide exact memory form equality.
    pub(in crate::check) fn decide_form_equal(
        &self,
        left: &FormTerm,
        right: &FormTerm,
    ) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (
                FormTerm::Borrowed {
                    lifetime: left_lifetime,
                    access: left_access,
                },
                FormTerm::Borrowed {
                    lifetime: right_lifetime,
                    access: right_access,
                },
            ) => {
                let lifetime = self.decide_static_relation(
                    StaticRelation::Equal,
                    *left_lifetime,
                    *right_lifetime,
                )?;
                if lifetime != Decision::Yes {
                    return Ok(lifetime);
                }

                self.decide_static_relation(StaticRelation::Equal, *left_access, *right_access)?
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide memory form assignability.
    pub(in crate::check) fn decide_form_assignable(
        &self,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_form_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (
                FormTerm::Borrowed {
                    lifetime: source_lifetime,
                    access: source_access,
                },
                FormTerm::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                let lifetime = self.decide_static_relation(
                    StaticRelation::Assignable,
                    *source_lifetime,
                    *target_lifetime,
                )?;
                if lifetime != Decision::Yes {
                    return Ok(lifetime);
                }

                self.decide_access_assignable(*source_access, *target_access)?
            }
            (FormTerm::Placed { place: source }, FormTerm::Placed { place: target }) => {
                self.decide_static_relation(StaticRelation::Equal, *source, *target)?
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Decision::Yes,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide whether one access capability can flow into another.
    pub(in crate::check) fn decide_access_assignable(
        &self,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<Decision> {
        self.decide_static_relation(StaticRelation::Assignable, source, target)
    }

    /// Constrain two memory forms by equality.
    pub(in crate::check) fn constrain_form_equal(
        &mut self,
        left: &FormTerm,
        right: &FormTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (left, right) {
            (
                FormTerm::Borrowed {
                    lifetime: left_lifetime,
                    access: left_access,
                },
                FormTerm::Borrowed {
                    lifetime: right_lifetime,
                    access: right_access,
                },
            ) => {
                let lifetime = self.solve_static_equality(*left_lifetime, *right_lifetime)?;
                let access = self.solve_static_equality(*left_access, *right_access)?;

                lifetime.merge(access)
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.solve_static_equality(*left, *right)?
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Progress::Unchanged,
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Constrain two memory forms by assignability.
    pub(in crate::check) fn constrain_form_assignable(
        &mut self,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, target) {
            (FormTerm::Placed { place: source }, FormTerm::Placed { place: target }) => {
                self.solve_static_equality(*source, *target)?
            }
            (
                FormTerm::Borrowed {
                    lifetime: source_lifetime,
                    access: source_access,
                },
                FormTerm::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                let lifetime =
                    self.solve_static_assignability(*source_lifetime, *target_lifetime)?;
                let access = self.solve_static_assignability(*source_access, *target_access)?;

                lifetime.merge(access)
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Progress::Unchanged,
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect one form term to satisfy expected form fields.
    pub(in crate::check) fn expect_form_term(
        &mut self,
        form: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (form, target) {
            (
                FormTerm::Borrowed { lifetime, access },
                FormTerm::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                let lifetime = self.solve_static_equality(*lifetime, *target_lifetime)?;
                let access = self.solve_static_equality(*access, *target_access)?;

                lifetime.merge(access)
            }
            (
                FormTerm::Placed { place },
                FormTerm::Placed {
                    place: target_place,
                },
            ) => self.solve_static_equality(*place, *target_place)?,
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Progress::Unchanged,
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }
}
