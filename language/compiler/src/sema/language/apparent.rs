use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{BodyState, CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// One declaration instance used for apparent member lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct ApparentInstance {
    /// The declaration that owns the apparent members.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The applied declaration arguments.
    pub(in crate::sema) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
}

impl ApparentInstance {
    /// Intern this instance into the checked module.
    pub(in crate::sema) fn intern(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = check.intern_type_ids(&self.arguments)?;
        let instance = dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        };

        check.intern_type(dir::Type::Application(instance))
    }

    /// Return the generic substitution represented by this instance.
    pub(in crate::sema) fn substitution(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = check.symbol_template(self.symbol)? else {
            if self.arguments.is_empty() {
                return Ok(TypeSubstitution::default());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric apparent owner {:?} has applied type arguments",
                    self.symbol
                ),
            });
        };

        check.applied_substitution(template, &self.arguments)
    }
}

impl CheckState<'_> {
    /// Intern the canonical Array application over one element type.
    pub(in crate::sema) fn array_type(
        &mut self,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.language_symbol(dir::LanguageItem::Array)?;
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }
        let arguments = self.intern_type_ids(&[element])?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))
    }

    /// Return the element type behind one canonical Array application.
    pub(in crate::sema) fn array_element(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let dir::Type::Application(instance) = self.ty(ty)? else {
            return Ok(None);
        };
        if self.language_item(instance.symbol)? != Some(dir::LanguageItem::Array) {
            return Ok(None);
        }
        let [element] = self.type_ids(ty.module_id, instance.arguments)? else {
            return Ok(None);
        };

        Ok(Some(*element))
    }

    /// Intern the type used for apparent member lookup into the checked module.
    pub(in crate::sema) fn intern_apparent_type(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(instance) = self.apparent_instance(receiver)? else {
            return Ok(receiver);
        };

        instance.intern(self)
    }

    /// Return the declaration instance that owns one receiver's apparent members.
    pub(in crate::sema) fn apparent_instance(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        let instance = match self.ty(receiver)? {
            dir::Type::Form(form) => return self.apparent_instance(form.value),
            dir::Type::Variant(variant) => {
                return self.apparent_instance(variant.owner);
            }
            dir::Type::Application(instance) => ApparentInstance {
                symbol: instance.symbol,
                arguments: self
                    .type_ids(receiver.module_id, instance.arguments)?
                    .iter()
                    .copied()
                    .collect(),
            },
            ref ty @ (dir::Type::Literal(_) | dir::Type::Primitive(_)) => {
                let Some(item) = ty.member_owner_item() else {
                    return Ok(None);
                };

                ApparentInstance {
                    symbol: self.language_symbol(item)?,
                    arguments: SmallVec::new(),
                }
            }
            dir::Type::Slice(slice) => ApparentInstance {
                symbol: self.language_symbol(dir::LanguageItem::Slice)?,
                arguments: SmallVec::from_slice(&[slice.element]),
            },
            dir::Type::FixedArray(array) => ApparentInstance {
                symbol: self.language_symbol(dir::LanguageItem::FixedArray)?,
                arguments: SmallVec::from_slice(&[array.element, array.count]),
            },
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }
}

impl BodyState<'_, '_> {
    /// Return the apparent object members one construction target accepts.
    pub(in crate::sema) fn apparent_object_members(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<
        Option<(
            SmallVec<[dir::TypeProperty; 8]>,
            SmallVec<[dir::TypeIndexSignature; 2]>,
        )>,
    > {
        match self.ty(target)? {
            // read fields and index signatures straight off a structural target
            dir::Type::Object(shape) => {
                let fields = SmallVec::from_slice(
                    self.shape_properties(target.module_id, shape.properties)?,
                );
                let indexes = SmallVec::from_slice(
                    self.shape_index_signatures(target.module_id, shape.index_signatures)?,
                );

                Ok(Some((fields, indexes)))
            }

            // read fields accepted by nominal struct construction
            dir::Type::Application(instance)
                if matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Struct(_))
                ) =>
            {
                let fields = self.struct_constructor_fields(origin, target)?;

                Ok(Some((fields, SmallVec::new())))
            }

            // read fields from structural interfaces
            dir::Type::Application(instance)
                if matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(interface)) if !interface.is_nominal
                ) =>
            {
                let fields = self.check.interface_instance_fields(target, target)?;
                let fields = fields.map(|fields| (SmallVec::from_vec(fields), SmallVec::new()));

                Ok(fields)
            }

            // merge the fields accepted by every intersection arm
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .check
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();
                let mut fields = SmallVec::<[dir::TypeProperty; 8]>::new();
                let mut indexes = SmallVec::new();
                for element in elements {
                    // read the members this arm accepts
                    let element = self.check.structurally_normalize(origin, element)?;
                    let Some((arm_fields, arm_indexes)) =
                        self.apparent_object_members(origin, element)?
                    else {
                        return Ok(None);
                    };

                    // merge each arm field into the collected set
                    for field in arm_fields {
                        let Some(existing) =
                            fields.iter().position(|existing| existing.key == field.key)
                        else {
                            fields.push(field);
                            continue;
                        };
                        fields[existing] = self.intersect_type_property(fields[existing], field)?;
                    }
                    indexes.extend(arm_indexes);
                }

                Ok(Some((fields, indexes)))
            }

            // reject object literal fields for every other target
            _ => Ok(None),
        }
    }

    /// Merge one duplicate field into its collected intersection field.
    fn intersect_type_property(
        &mut self,
        existing: dir::TypeProperty,
        field: dir::TypeProperty,
    ) -> CompilerResult<dir::TypeProperty> {
        let read = self.intersect_access_values(existing.access.read(), field.access.read())?;
        let write = self.intersect_access_values(existing.access.write(), field.access.write())?;
        let access = match (read, write) {
            (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
            (Some(read), None) => dir::PropertyAccess::Read(read),
            (None, Some(write)) => dir::PropertyAccess::Write(write),
            (None, None) => existing.access,
        };

        Ok(dir::TypeProperty {
            key: existing.key,
            access,
            is_optional: existing.is_optional && field.is_optional,
        })
    }

    /// Intersect two optional access value types into one.
    fn intersect_access_values(
        &mut self,
        left: Option<dir::GlobalTypeId>,
        right: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match (left, right) {
            (Some(left), Some(right)) if left != right => {
                let elements = self.check.intern_type_ids(&[left, right])?;
                let ty = dir::Type::Intersection(dir::IntersectionType { elements });

                Ok(Some(self.check.intern_type(ty)?))
            }
            (Some(left), _) => Ok(Some(left)),
            (None, right) => Ok(right),
        }
    }
}
