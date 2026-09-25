use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Decide whether one type is a fixed point of `readonly`.
    pub(in crate::sema) fn is_immutable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        let ty = self.shallow_resolve(ty)?;

        // accept one type already under decision, coinductively
        if active.contains(&ty) {
            return Ok(true);
        }
        active.push(ty);

        let result = self.is_immutable_head(origin, ty, active);
        active.pop();

        result
    }

    /// Decide whether one resolved type is immutable at its head.
    fn is_immutable_head(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        // decide by the storage of the value
        match self.ty(ty)? {
            // open variables fail as ambiguity until they solve
            dir::Type::Variable(_) => Ok(false),
            // accept valueless and scalar types, they grant no capability
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_) => Ok(true),
            // take variant immutability from the owning type
            dir::Type::Variant(member) => self.is_immutable(origin, member.owner, active),
            // readonly forms grant reads alone, transitively
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly => Ok(true),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    self.is_readonly_access(access)
                }
                dir::Form::Owned => self.is_immutable(origin, form.value, active),
                dir::Form::Raw => Ok(false),
            },
            // decide parameters through their declared or assumed bounds
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let mut decision = false;
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = self.is_immutable(origin, bound, active)?;
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
            // decide arrays through their element
            dir::Type::FixedArray(array) => self.is_immutable(origin, array.element, active),
            // decide tuples through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_immutable(origin, ids, active)
            }
            // shapes are immutable when every capability they grant is a read
            dir::Type::Object(shape) => {
                // reject shapes that can be called or constructed
                if !self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .is_empty()
                    || !self
                        .type_ids(ty.module_id, shape.construct_signatures)?
                        .is_empty()
                {
                    return Ok(false);
                }

                // reject shapes with a writable property
                let fields: SmallVec<[_; 4]> = self
                    .object_properties(ty.module_id, shape.properties)?
                    .into();
                if fields.iter().any(|field| field.access.is_writable()) {
                    return Ok(false);
                }

                // reject shapes with a writable index signature
                let signatures: SmallVec<[_; 4]> = self
                    .object_index_signatures(ty.module_id, shape.index_signatures)?
                    .into();
                if signatures.iter().any(|signature| !signature.is_readonly) {
                    return Ok(false);
                }

                // decide the property and index signature types of the shape
                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> = fields
                    .iter()
                    .flat_map(|field| field.access.types())
                    .collect();
                ids.extend(signatures.iter().map(|signature| signature.value_type));

                self.all_immutable(origin, ids, active)
            }
            // decide unions through every element
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_immutable(origin, ids, active)
            }
            // decide intersections through every element
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
            if !self.is_immutable(origin, id, active)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
