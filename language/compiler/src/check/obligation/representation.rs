use destack_core::{FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, ObligationCheck, ObligationFailure, Origin, answer};

impl CheckState<'_> {
    /// Check that one stored type does not contain itself by value.
    pub(in crate::check) fn check_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let source = self.origin_source(origin)?;
        let mut active = FxIndexSet::default();
        let mut circular = None;

        if answer!(self.decide_finite_storage(origin, ty, source, &mut active, &mut circular)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }
        let source = circular.unwrap_or(source);
        let failure = ObligationFailure::CircularType { source };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }

    /// Decide whether one type's by-value storage is finite.
    fn decide_finite_storage(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        circular: &mut Option<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // by-value cycles have no finite representation
        if !active.insert(ty) {
            circular.get_or_insert(source);

            return Ok(Answer::Ready(false));
        }
        let finite = ensure_sufficient_stack(|| {
            self.decide_finite_storage_children(origin, ty, source, active, circular)
        });
        active.swap_remove(&ty);

        finite
    }

    /// Decide every by-value child of one reduced type.
    fn decide_finite_storage_children(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        circular: &mut Option<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<Answer<bool>> {
        let owner = ty.module_id;

        match self.ty(ty)? {
            // direct forms keep their payload inline, indirect forms break cycles
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                    self.decide_finite_storage(origin, form.value, source, active, circular)
                }
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    Ok(Answer::Ready(true))
                }
            },
            dir::Type::FixedArray(array) => {
                self.decide_finite_storage(origin, array.element, source, active, circular)
            }
            dir::Type::Tuple(tuple) => {
                let elements = self
                    .tuple_elements(owner, tuple.elements)?
                    .iter()
                    .map(|element| (element.ty, source))
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_each_finite_storage(origin, &elements, active, circular)
            }
            dir::Type::Shape(shape) => {
                let fields = self
                    .shape_fields(owner, shape.fields)?
                    .iter()
                    .map(|field| (field.ty, source))
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_each_finite_storage(origin, &fields, active, circular)
            }
            dir::Type::Union(union) => {
                let elements = self
                    .type_ids(owner, union.elements)?
                    .iter()
                    .map(|element| (*element, source))
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_each_finite_storage(origin, &elements, active, circular)
            }
            dir::Type::EnumMember(member) => {
                self.decide_finite_storage(origin, member.owner, source, active, circular)
            }
            dir::Type::Instance(instance) => self
                .decide_finite_instance_storage(origin, owner, &instance, source, active, circular),
            _ => Ok(Answer::Ready(true)),
        }
    }

    /// Decide the substituted by-value storage of one applied declaration.
    fn decide_finite_instance_storage(
        &mut self,
        origin: Origin,
        owner: ModuleId,
        instance: &dir::GenericInstance,
        source: dir::GlobalNodeIdAny,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        circular: &mut Option<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<Answer<bool>> {
        // vectors store their element inline, other intrinsics store handles
        if let Some(item) = self.language_item(instance.symbol)? {
            if item == dir::LanguageItem::Vector
                && let Some(element) = self.type_ids(owner, instance.arguments)?.first().copied()
            {
                return self.decide_finite_storage(origin, element, source, active, circular);
            }

            return Ok(Answer::Ready(true));
        }

        let storage = match self.definition(instance.symbol)?.cloned() {
            // newtypes are transparent over their backing
            Some(dir::Definition::Newtype(definition)) => {
                SmallVec::<[_; 4]>::from_slice(&[(definition.value, source)])
            }
            // structs store their instance fields inline
            Some(definition @ dir::Definition::Struct(_)) => {
                let mut fields = SmallVec::new();
                for member in definition.members() {
                    let dir::DefinitionMember::Field(field) = member else {
                        continue;
                    };
                    if field.space != dir::MemberSpace::Instance {
                        continue;
                    }
                    let Some(ty) = answer!(self.definition_member_type(member)?) else {
                        continue;
                    };
                    fields.push((ty, field.source));
                }

                fields
            }
            // classes store references, enums store a scalar tag
            _ => return Ok(Answer::Ready(true)),
        };

        // substitute applied arguments through the declared storage
        let substitution = self.instance_substitution(owner, instance)?;
        let mut substituted = SmallVec::<[_; 4]>::new();
        for (ty, source) in storage {
            substituted.push((
                self.substitute_type(origin.module(), ty, &substitution)?,
                source,
            ));
        }

        self.decide_each_finite_storage(origin, &substituted, active, circular)
    }

    /// Decide finite storage for every listed slot.
    fn decide_each_finite_storage(
        &mut self,
        origin: Origin,
        slots: &[(dir::GlobalTypeId, dir::GlobalNodeIdAny)],
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        circular: &mut Option<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<Answer<bool>> {
        for (ty, source) in slots {
            if !answer!(self.decide_finite_storage(origin, *ty, *source, active, circular)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }
}
