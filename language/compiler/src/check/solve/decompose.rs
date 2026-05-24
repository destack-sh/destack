use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, FormTerm, ShapeMemberTerm, TupleElementTerm, TypeTerm, VariableId,
};

use super::queue::Progress;

impl CheckComponentState<'_> {
    /// Decompose equality between solved type terms into smaller relations.
    pub(super) fn relate_solved_type_equal(
        &mut self,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (left, right) {
            (
                TypeTerm::Form {
                    form: left_form,
                    payload: left_value,
                },
                TypeTerm::Form {
                    form: right_form,
                    payload: right_value,
                },
            ) => {
                let form = self.relate_form_equal(left_form, right_form)?;
                let value = self.relate_type_equal(*left_value, *right_value)?;

                form.merge(value)
            }
            (
                TypeTerm::Array {
                    element: left_element,
                },
                TypeTerm::Array {
                    element: right_element,
                },
            )
            | (
                TypeTerm::Slice {
                    element: left_element,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: right_element,
                    is_readonly: _,
                },
            ) => self.relate_type_equal(*left_element, *right_element)?,
            (
                TypeTerm::FixedArray {
                    element: left_element,
                    length: left_length,
                    is_readonly: _,
                },
                TypeTerm::FixedArray {
                    element: right_element,
                    length: right_length,
                    is_readonly: _,
                },
            ) => {
                let element = self.relate_type_equal(*left_element, *right_element)?;
                let length = self.relate_static_equal(*left_length, *right_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left,
                    is_readonly: _,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right,
                    is_readonly: _,
                },
            ) => {
                if left_form == right_form {
                    self.relate_tuple_elements_equal(left, right)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: left }, TypeTerm::Shape { members: right }) => {
                self.relate_shape_members_equal(left, right)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decompose assignability between solved type terms into smaller relations.
    pub(super) fn relate_solved_type_assignable(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, target) {
            (
                TypeTerm::Form {
                    form: source_form,
                    payload: source_value,
                },
                TypeTerm::Form {
                    form: target_form,
                    payload: target_value,
                },
            ) => {
                let form = self.relate_form_assignable(source_form, target_form)?;
                let value = self.relate_type_assignable(*source_value, *target_value)?;

                form.merge(value)
            }
            (
                TypeTerm::Array {
                    element: source_element,
                },
                TypeTerm::Array {
                    element: target_element,
                },
            )
            | (
                TypeTerm::Array {
                    element: source_element,
                },
                TypeTerm::Slice {
                    element: target_element,
                    is_readonly: _,
                },
            )
            | (
                TypeTerm::Slice {
                    element: source_element,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: target_element,
                    is_readonly: _,
                },
            ) => self.relate_type_assignable(*source_element, *target_element)?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: _,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: target_element,
                    is_readonly: _,
                },
            ) => self.relate_type_assignable(*source_element, *target_element)?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                    is_readonly: _,
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                    is_readonly: _,
                },
            ) => {
                let element = self.relate_type_assignable(*source_element, *target_element)?;
                let length = self.relate_static_equal(*source_length, *target_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source,
                    is_readonly: _,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target,
                    is_readonly: _,
                },
            ) => {
                if source_form == target_form {
                    self.relate_tuple_elements_assignable(source, target)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.relate_shape_members_assignable(source, target)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Relate matching tuple elements by equality.
    fn relate_tuple_elements_equal(
        &mut self,
        left: &[TupleElementTerm],
        right: &[TupleElementTerm],
    ) -> CompilerResult<Progress> {
        if left.len() != right.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // propagate each matching element
        for (left, right) in left.iter().zip(right) {
            progress = progress.merge(self.relate_type_equal(left.ty, right.ty)?);
        }

        Ok(progress)
    }

    /// Relate matching tuple elements by assignability.
    fn relate_tuple_elements_assignable(
        &mut self,
        source: &[TupleElementTerm],
        target: &[TupleElementTerm],
    ) -> CompilerResult<Progress> {
        if source.len() != target.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // propagate each matching element
        for (source, target) in source.iter().zip(target) {
            progress = progress.merge(self.relate_type_assignable(source.ty, target.ty)?);
        }

        Ok(progress)
    }

    /// Relate matching shape fields by equality.
    fn relate_shape_members_equal(
        &mut self,
        left: &[ShapeMemberTerm],
        right: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // propagate common fields in both directions
        for left in left {
            let Some((left_key, left_ty)) = shape_field(left) else {
                continue;
            };
            let Some(right_ty) = shape_field_type(right, left_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_equal(left_ty, right_ty)?);
        }

        Ok(progress)
    }

    /// Relate matching shape fields by assignability.
    fn relate_shape_members_assignable(
        &mut self,
        source: &[ShapeMemberTerm],
        target: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push target field types into source fields
        for target in target {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(source_ty) = shape_field_type(source, target_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_assignable(source_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Relate two memory forms by equality.
    fn relate_form_equal(&mut self, left: &FormTerm, right: &FormTerm) -> CompilerResult<Progress> {
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
                let lifetime = self.relate_static_equal(*left_lifetime, *right_lifetime)?;
                let access = self.relate_static_equal(*left_access, *right_access)?;

                lifetime.merge(access)
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.relate_static_equal(*left, *right)?
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Progress::Unchanged,
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Relate two memory forms by assignability.
    fn relate_form_assignable(
        &mut self,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, target) {
            (FormTerm::Placed { place: source }, FormTerm::Placed { place: target }) => {
                self.relate_static_equal(*source, *target)?
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
}

/// Return one shape field key and type.
pub(super) fn shape_field(member: &ShapeMemberTerm) -> Option<(dir::StaticKey, VariableId)> {
    match member {
        ShapeMemberTerm::Field {
            key,
            ty,
            is_optional: _,
            is_readonly: _,
        } => Some((key.clone(), *ty)),
        ShapeMemberTerm::CallSignature { ty: _ }
        | ShapeMemberTerm::ConstructSignature { ty: _ }
        | ShapeMemberTerm::IndexSignature {
            name: _,
            key_type: _,
            value_type: _,
            is_optional: _,
            is_readonly: _,
        } => None,
    }
}

/// Return one shape field type by key.
pub(super) fn shape_field_type(
    members: &[ShapeMemberTerm],
    key: dir::StaticKey,
) -> Option<VariableId> {
    members.iter().find_map(|member| {
        let (member_key, ty) = shape_field(member)?;

        member_key.matches(&key).then_some(ty)
    })
}
