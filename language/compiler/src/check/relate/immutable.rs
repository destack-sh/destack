use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Origin};

impl CheckState<'_> {
    /// Decide whether one type is a fixed point of `readonly`.
    pub(in crate::check) fn type_is_immutable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        let ty = self.shallow_resolve(ty)?;
        if active.contains(&ty) {
            return Ok(true);
        }
        active.push(ty);

        let result = self.decide_immutable_type(origin, ty, active);
        active.pop();

        result
    }

    /// Decide immutability for one active type.
    fn decide_immutable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        let ty = self.reduce_type_head(origin, ty)?;

        match self.ty(ty)? {
            // open variables are ambiguity: the ambiguity witness retries
            //  the judgment once they solve
            dir::Type::Variable(_) => {
                self.infer.ambiguity = true;

                Ok(false)
            }
            // reject valueless and scalar types, they have no capability
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_) => Ok(true),
            dir::Type::Variant(member) => self.type_is_immutable(origin, member.owner, active),
            // readonly forms grant reads alone, transitively
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly => Ok(true),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    self.body().access_is_readonly(origin, access)
                }
                dir::Form::Owned => self.type_is_immutable(origin, form.value, active),
                dir::Form::Raw | dir::Form::Managed | dir::Form::Placed { .. } => Ok(false),
            },
            // parameters prove through declared or assumed bounds
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let mut decision = false;
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = self.type_is_immutable(origin, bound, active)?;
                    if decision {
                        break;
                    }
                }

                Ok(decision)
            }
            // read nominal immutability from the compiler known language items
            dir::Type::Application(instance) => Ok(matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::String | dir::LanguageItem::BigInt)
            )),
            dir::Type::FixedArray(array) => self.type_is_immutable(origin, array.element, active),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_immutable(origin, ids, active)
            }
            // shapes are immutable when every capability they grant is a read
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                if !self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .is_empty()
                    || !self
                        .type_ids(ty.module_id, shape.construct_signatures)?
                        .is_empty()
                {
                    return Ok(false);
                }
                let fields = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .to_vec();
                if fields.iter().any(|field| field.access.is_writable()) {
                    return Ok(false);
                }
                let signatures = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .to_vec();
                if signatures.iter().any(|signature| !signature.is_readonly) {
                    return Ok(false);
                }

                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> = fields
                    .iter()
                    .flat_map(|field| field.access.types())
                    .collect();
                ids.extend(signatures.iter().map(|signature| signature.value_type));

                self.all_immutable(origin, ids, active)
            }
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_immutable(origin, ids, active)
            }
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_immutable(origin, ids, active)
            }
            // everything else may reach mutable state through its values
            _ => Ok(false),
        }
    }

    /// Decide immutability across one element list.
    fn all_immutable(
        &mut self,
        origin: Origin,
        ids: SmallVec<[dir::GlobalTypeId; 8]>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        for id in ids {
            if !self.type_is_immutable(origin, id, active)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
