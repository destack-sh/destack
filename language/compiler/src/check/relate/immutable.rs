use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

impl CheckState<'_> {
    /// Decide whether one type is a fixed point of `readonly`.
    pub(in crate::check) fn type_is_immutable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.settled_root(ty)?;
        if active.contains(&ty) {
            return Ok(Answer::Ready(true));
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
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        match self.ty(ty)? {
            dir::Type::Variable(variable) => Ok(Answer::pending([Dependency::Variable(variable)])),
            // valueless and scalar types carry no capability at all
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
            | dir::Type::Primitive(_) => Ok(Answer::Ready(true)),
            dir::Type::EnumMember(member) => self.type_is_immutable(origin, member.owner, active),
            // readonly forms grant reads alone, transitively
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly => Ok(Answer::Ready(true)),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    self.body().access_is_readonly(origin, access)
                }
                dir::Form::Owned => self.type_is_immutable(origin, form.value, active),
                dir::Form::Raw | dir::Form::Managed | dir::Form::Placed { .. } => {
                    Ok(Answer::Ready(false))
                }
            },
            // parameters prove through declared or assumed bounds
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let mut decision = Answer::Ready(false);
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = decision.or(self.type_is_immutable(origin, bound, active)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }
            // nominal immutability is seeded by language semantics
            dir::Type::Application(instance) => Ok(Answer::Ready(matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::String | dir::LanguageItem::BigInt)
            ))),
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
            dir::Type::Shape(shape) => {
                if !self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .is_empty()
                    || !self
                        .type_ids(ty.module_id, shape.construct_signatures)?
                        .is_empty()
                {
                    return Ok(Answer::Ready(false));
                }
                let fields = self.shape_fields(ty.module_id, shape.fields)?.to_vec();
                if fields.iter().any(|field| !field.is_readonly) {
                    return Ok(Answer::Ready(false));
                }
                let signatures = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .to_vec();
                if signatures.iter().any(|signature| !signature.is_readonly) {
                    return Ok(Answer::Ready(false));
                }

                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    fields.iter().map(|field| field.ty).collect();
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
            _ => Ok(Answer::Ready(false)),
        }
    }

    /// Decide immutability across one element list.
    fn all_immutable(
        &mut self,
        origin: Origin,
        ids: SmallVec<[dir::GlobalTypeId; 8]>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.type_is_immutable(origin, id, active)?);
            if decision.is_ready_false() {
                break;
            }
        }

        Ok(decision)
    }
}
