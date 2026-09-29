use tspp_core::StringId;
use tspp_mir as mir;

use crate::instantiate::function::Specialization;
use crate::instantiate::function::union::CaseMap;
use crate::{CompilerError, CompilerResult};

impl Specialization<'_, '_> {
    /// Rewrite one copied place at its instance's shapes.
    pub(super) fn reshape_place(
        &mut self,
        template: &mir::Place,
        place: &mut mir::Place,
    ) -> CompilerResult<()> {
        if !template.path.projections.iter().any(|projection| {
            matches!(
                projection,
                mir::Projection::Variant { .. } | mir::Projection::Member { .. }
            )
        }) {
            return Ok(());
        }
        let Some(root) = template.root_type(self.template, self.source) else {
            return Err(self.untyped_place("place root"));
        };

        // walk the template projections
        let mut ty = mir::PlaceType::Value(root);
        let mut projections = Vec::with_capacity(place.path.projections.len());
        for (template_projection, projection) in template
            .path
            .projections
            .iter()
            .zip(&place.path.projections)
        {
            let next = ty
                .project(template_projection, self.source)
                .ok_or_else(|| self.untyped_place("place projection"))?;
            match (template_projection, ty) {
                // select the kept case
                (mir::Projection::Variant { case }, mir::PlaceType::Value(variant)) => {
                    match self.case_map(variant) {
                        CaseMap::Kept => projections.push(projection.clone()),
                        CaseMap::Merged(indices) => projections.push(mir::Projection::Variant {
                            case: indices[*case as usize],
                        }),
                        CaseMap::Collapsed => {}
                    }
                }
                // select the field by name
                (mir::Projection::Member { name, .. }, mir::PlaceType::Value(parameter)) => {
                    let closed = self.ty(parameter);
                    projections.extend(self.member_path(closed, *name)?);
                }
                _ => projections.push(projection.clone()),
            }
            ty = next;
        }
        place.path.projections = projections;

        Ok(())
    }

    /// Return the projections selecting one named field of a closed type.
    fn member_path(
        &self,
        closed: mir::TypeId,
        name: StringId,
    ) -> CompilerResult<Vec<mir::Projection>> {
        let tree = &self.state.tree;
        let (mut path, storage) = match tree.type_definition(closed) {
            mir::Type::Reference { pointee, .. } => (vec![mir::Projection::Deref], *pointee),
            _ => (Vec::new(), closed),
        };
        let (mir::Type::Struct { fields } | mir::Type::Class { fields, .. }) =
            tree.type_definition(storage)
        else {
            return Err(self.absent_member(closed));
        };
        let index = fields
            .iter()
            .position(|field| tree.get(*field).name == Some(name))
            .ok_or_else(|| self.absent_member(closed))?;
        path.push(mir::Projection::Field {
            index: index as u32,
        });

        Ok(path)
    }

    /// Build the error for an untyped template place.
    fn untyped_place(&self, operation: &str) -> CompilerError {
        CompilerError::Internal {
            message: format!("a template {operation} without its type"),
        }
    }

    /// Build the error for a member the instance storage lacks.
    fn absent_member(&self, closed: mir::TypeId) -> CompilerError {
        CompilerError::Internal {
            message: format!("a parameter member absent from the instance storage {closed:?}"),
        }
    }
}
