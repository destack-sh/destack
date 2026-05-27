use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericSubstitution, Progress, StaticRelation, StaticTerm, VariableId,
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
        lifetime: VariableId,
        /// The solved access value.
        access: VariableId,
    },
    /// Raw pointer form.
    Raw,
    /// Placed value form.
    Placed {
        /// The solved place value.
        place: VariableId,
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
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Borrowed { lifetime, access } => {
                variables.push(*lifetime);
                variables.push(*access);
            }
            Self::Placed { place } => variables.push(*place),
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
                lifetime: state.substitute_static_variable(module, substitution, *lifetime)?,
                access: state.substitute_static_variable(module, substitution, *access)?,
            },
            Self::Placed { place } => Self::Placed {
                place: state.substitute_static_variable(module, substitution, *place)?,
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
                FormTerm::Borrowed { access: source, .. },
                FormTerm::Borrowed { access: target, .. },
            ) => self.decide_access_assignable(*source, *target)?,
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
        source: VariableId,
        target: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(source) = self.access_value(source)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(target) = self.access_value(target)? else {
            return Ok(Decision::Undecidable);
        };

        if Self::access_rank(source) >= Self::access_rank(target) {
            Ok(Decision::Yes)
        } else {
            Ok(Decision::No)
        }
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
            (FormTerm::Borrowed { .. }, FormTerm::Borrowed { .. })
            | (FormTerm::Managed, FormTerm::Managed)
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

    /// Return one solved access value.
    fn access_value(&self, variable: VariableId) -> CompilerResult<Option<dir::Access>> {
        let Some(term) = self.solved_static_term(variable)? else {
            return Ok(None);
        };
        let access = match term {
            StaticTerm::Literal(dir::StaticTerm::Access { access }) => Some(access),
            _ => None,
        };

        Ok(access)
    }

    /// Return one access capability rank.
    fn access_rank(access: dir::Access) -> u8 {
        match access {
            dir::Access::Readonly => 0,
            dir::Access::Mutable => 1,
            dir::Access::Exclusive => 2,
        }
    }
}
