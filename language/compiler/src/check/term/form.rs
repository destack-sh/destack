use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, Origin, StaticOperand, StaticRelation, SubstitutionSet,
};
use destack_source::ModuleId;

/// Check-local memory form term.
///
/// Examples:
/// ```ds
/// &value
/// ^value
/// shared ^value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum FormTerm {
    /// Managed value form.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// ```
    Managed,
    /// Owned value form.
    ///
    /// Examples:
    /// ```ds
    /// ^value
    /// ```
    Owned,
    /// Borrowed value form.
    ///
    /// Examples:
    /// ```ds
    /// &value
    /// &exclusive value
    /// ```
    Borrowed {
        /// The solved lifetime value.
        lifetime: StaticOperand,
        /// The solved access value.
        access: StaticOperand,
    },
    /// Raw pointer form.
    ///
    /// Examples:
    /// ```ds
    /// *value
    /// ```
    Raw,
    /// Placed value form.
    ///
    /// Examples:
    /// ```ds
    /// shared ^value
    /// ```
    Placed {
        /// The solved place value.
        place: StaticOperand,
    },
    /// Readonly view form.
    ///
    /// Examples:
    /// ```ds
    /// readonly value
    /// ```
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

    /// Substitute generic arguments through this memory form.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
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
            Self::Managed | Self::Owned | Self::Raw | Self::Readonly => *self,
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
    ) -> CompilerResult<Answer<bool>> {
        if left == right {
            return Ok(Answer::Ready(true));
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
                if lifetime != Answer::Ready(true) {
                    return Ok(lifetime);
                }

                self.decide_static_relation(StaticRelation::Equal, *left_access, *right_access)?
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide memory form assignability.
    pub(in crate::check) fn decide_form_assignable(
        &self,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_form_equal(source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
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
                if lifetime != Answer::Ready(true) {
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
            | (FormTerm::Readonly, FormTerm::Readonly) => Answer::Ready(true),
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one access capability can flow into another.
    pub(in crate::check) fn decide_access_assignable(
        &self,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<Answer<bool>> {
        self.decide_static_relation(StaticRelation::Assignable, source, target)
    }

    /// Constrain two memory forms by equality.
    pub(in crate::check) fn constrain_form_equal(
        &mut self,
        origin: Origin,
        left: &FormTerm,
        right: &FormTerm,
    ) -> CompilerResult<()> {
        match (left, right) {
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
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *left_lifetime,
                    *right_lifetime,
                    Condition::Always,
                );
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *left_access,
                    *right_access,
                    Condition::Always,
                );
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *left,
                    *right,
                    Condition::Always,
                );
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => (),
            _ => (),
        }

        Ok(())
    }

    /// Constrain two memory forms by assignability.
    pub(in crate::check) fn constrain_form_assignable(
        &mut self,
        origin: Origin,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<()> {
        match (source, target) {
            (FormTerm::Placed { place: source }, FormTerm::Placed { place: target }) => {
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *source,
                    *target,
                    Condition::Always,
                );
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
                self.constrain_static(
                    origin,
                    StaticRelation::Assignable,
                    *source_lifetime,
                    *target_lifetime,
                    Condition::Always,
                );
                self.constrain_static(
                    origin,
                    StaticRelation::Assignable,
                    *source_access,
                    *target_access,
                    Condition::Always,
                );
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => (),
            _ => (),
        }

        Ok(())
    }
}
