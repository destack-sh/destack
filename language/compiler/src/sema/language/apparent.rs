use smallvec::SmallVec;
use tspp_dir as dir;

use crate::sema::{CheckState, Origin, TypeSubstitution};
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
        // intern the applied declaration
        let arguments = check.intern_type_ids(&self.arguments)?;
        let instance = dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        };

        check.intern_type(dir::Type::Application(instance))
    }

    /// Intern the application this instance qualifies its associated projections with.
    pub(in crate::sema) fn qualifier(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.arguments.is_empty() {
            return self.intern(check);
        }
        let application = check.declaration_instance(self.symbol)?;

        check.intern_type(dir::Type::Application(application))
    }

    /// Return the generic substitution represented by this instance.
    pub(in crate::sema) fn substitution(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<TypeSubstitution> {
        // keep an unapplied bare declaration's parameters rigid
        if self.arguments.is_empty() {
            return Ok(TypeSubstitution::default());
        }
        let Some(template) = check.symbol_template(self.symbol)? else {
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
        // apply the Array declaration over the element
        let symbol = self.language_symbol(dir::LanguageItem::Array)?;
        let arguments = self.intern_type_ids(&[element])?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))
    }

    /// Return the element type of a canonical array, slice, or fixed array.
    pub(in crate::sema) fn sequence_element_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match self.ty(ty)? {
            dir::Type::Slice(slice) => Ok(Some(slice.element)),
            dir::Type::FixedArray(array) => Ok(Some(array.element)),
            dir::Type::Application(instance) => {
                let item = self.language_item(instance.symbol)?;
                if !matches!(
                    item,
                    Some(dir::LanguageItem::Array | dir::LanguageItem::ReadonlyArray)
                ) {
                    return Ok(None);
                }
                let [element] = self.type_ids(ty.module_id, instance.arguments)? else {
                    return Err(CompilerError::Internal {
                        message: "an array application without one element type".to_string(),
                    });
                };

                Ok(Some(*element))
            }
            _ => Ok(None),
        }
    }

    /// Return the element type behind one canonical Array application.
    pub(in crate::sema) fn array_element(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // require a canonical Array application
        let dir::Type::Application(instance) = self.ty_raw(ty)? else {
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

    /// Return the primitive or tuple root an undeclared structural type keys its extensions at.
    pub(in crate::sema) fn structural_root(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeRoot>> {
        // key by the structural head
        Ok(match self.ty(ty)? {
            dir::Type::Primitive(primitive) => Some(dir::TypeRoot::Primitive(primitive)),
            // key a scalar literal at the primitive it widens to
            dir::Type::Literal(literal) => match literal {
                dir::Literal::String(_) => {
                    Some(dir::TypeRoot::Primitive(dir::PrimitiveType::String))
                }
                dir::Literal::Character(_) => {
                    Some(dir::TypeRoot::Primitive(dir::PrimitiveType::Character))
                }
                dir::Literal::Boolean(_) => {
                    Some(dir::TypeRoot::Primitive(dir::PrimitiveType::Boolean))
                }
                dir::Literal::Bigint(_) => {
                    Some(dir::TypeRoot::Primitive(dir::PrimitiveType::Bigint))
                }
                _ => None,
            },
            dir::Type::Tuple(_) => Some(dir::TypeRoot::Tuple),
            _ => None,
        })
    }

    /// Return the declaration instance owning one receiver's apparent members.
    pub(in crate::sema) fn apparent_instance(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        // read the receiver through its solution
        let receiver = self.shallow_resolve(receiver)?;

        // name the owner by the receiver's head
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
            dir::Type::Function(function) => {
                let Some(instance) =
                    self.function_instance(function.signature, function.receiver)?
                else {
                    return Ok(None);
                };

                instance
            }
            dir::Type::FunctionSignature(_) => {
                let elided = self.elided_receiver()?;
                let Some(instance) = self.function_instance(receiver, elided)? else {
                    return Ok(None);
                };

                instance
            }
            // leave every other receiver without an owner
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }

    /// Return the `Function` instance one signature writes.
    fn function_instance(
        &mut self,
        signature: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        // require a signature that declares a return type
        let Some(function) = self.signature_head(signature)? else {
            return Ok(None);
        };
        let Some(return_type) = function.return_type else {
            return Ok(None);
        };

        // write the parameters as one tuple argument
        let elements = self
            .signature_parameters(signature.module_id, function.parameters)?
            .iter()
            .map(|parameter| dir::TypeElement {
                label: parameter.name,
                ty: parameter.ty,
                is_optional: parameter.is_optional,
                is_readonly: false,
                is_rest: parameter.is_rest,
            })
            .collect::<Vec<_>>();
        let elements = self.intern_elements(&elements)?;
        let parameters = self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements,
        }))?;

        Ok(Some(ApparentInstance {
            symbol: self.language_symbol(dir::LanguageItem::Function)?,
            arguments: SmallVec::from_slice(&[parameters, return_type, receiver]),
        }))
    }
}

impl CheckState<'_> {
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
        // read the members of the object beneath the target's handle
        let object = self.normalize(origin, target)?;

        self.object_members(origin, object)
    }

    /// Return the members one object type declares by its head.
    fn object_members(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<
        Option<(
            SmallVec<[dir::TypeProperty; 8]>,
            SmallVec<[dir::TypeIndexSignature; 2]>,
        )>,
    > {
        // read the members by the head of the target
        match self.ty(target)? {
            // read fields and index signatures straight off a structural target
            dir::Type::Object(shape) => {
                let fields = SmallVec::from_slice(
                    self.object_properties(target.module_id, shape.properties)?,
                );
                let indexes = SmallVec::from_slice(
                    self.object_index_signatures(target.module_id, shape.index_signatures)?,
                );

                Ok(Some((fields, indexes)))
            }

            // read fields from structural interfaces
            dir::Type::Application(instance)
                if matches!(self.definition(instance.symbol)?.as_deref(),
                    Some(dir::Definition::Interface(interface)) if !interface.is_nominal
                ) =>
            {
                let fields = self.interface_instance_fields(target, target)?;
                let fields = fields.map(|fields| (SmallVec::from_vec(fields), SmallVec::new()));

                Ok(fields)
            }

            // merge the fields accepted by every intersection arm
            dir::Type::Intersection(intersection) => {
                let elements = self.type_ids(target.module_id, intersection.elements)?;
                let mut fields = SmallVec::<[dir::TypeProperty; 8]>::new();
                let mut indexes = SmallVec::new();
                for element in elements {
                    // read the members this arm accepts
                    let element = self.structurally_normalize(origin, *element)?;
                    let Some((arm_fields, arm_indexes)) = self.object_members(origin, element)?
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
        // intersect the read and write slots of both fields
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
        // intersect two present values, else take the one present
        match (left, right) {
            (Some(left), Some(right)) if left != right => {
                let elements = self.intern_type_ids(&[left, right])?;
                let ty = dir::Type::Intersection(dir::IntersectionType { elements });

                Ok(Some(self.intern_type(ty)?))
            }
            (Some(left), _) => Ok(Some(left)),
            (None, right) => Ok(right),
        }
    }
}
