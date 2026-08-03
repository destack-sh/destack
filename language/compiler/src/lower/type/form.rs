use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{ModuleLowerer, TypeLowerer, TypeSubstitution};
use crate::{CompilerError, CompilerResult, LowerError};

/// One indirect layer peeled from a value type.
pub(in crate::lower) struct Indirection {
    /// The type stored behind the indirection.
    pub(in crate::lower) stored: dir::GlobalTypeId,
    /// The access exposed through the indirection.
    pub(in crate::lower) access: mir::Access,
}

impl TypeLowerer<'_, '_> {
    /// Lower one form type through the memory form algebra.
    pub(in crate::lower) fn lower_form(
        &mut self,
        id: dir::GlobalTypeId,
        access: Option<mir::Access>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let dir::Type::Form(form) = self.lowerer.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "lowering entered the form algebra outside a form type".to_string(),
            });
        };

        match form.form {
            // narrow the access of the next indirect layer inward
            dir::Form::Readonly => match self.lowerer.ty(form.value)? {
                dir::Type::Form(_) => self.lower_form(form.value, Some(mir::Access::Readonly)),
                // lower readonly over a pure value at the narrowed access
                _ => self.lower_layer_value(form.value, Some(mir::Access::Readonly)),
            },

            // reference the stored payload of managed layers on the local heap
            dir::Form::Managed => self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                access.unwrap_or(mir::Access::Mutable),
                form.value,
            ),

            // carry the declared lifetime and access of borrowed layers
            dir::Form::Borrowed(borrow) => {
                let Some(borrow) = self.lowerer.types(id.module_id)?.borrow_form_maybe(borrow)
                else {
                    return Err(CompilerError::Internal {
                        message: "missing a borrow form".to_string(),
                    });
                };
                let (lifetime, borrow_access) = (borrow.lifetime, borrow.access);
                let lifetime = self
                    .lowerer
                    .lower_lifetime(lifetime, self.lifetime_parameters)?;
                let borrow_access = self.lowerer.borrow_access(borrow_access)?;

                self.lower_reference(
                    mir::ReferenceKind::Borrowed,
                    lifetime,
                    access.unwrap_or(borrow_access),
                    form.value,
                )
            }

            // raw layers produce process-local machine pointers
            dir::Form::Raw => {
                self.lower_pointer(form.value, access.unwrap_or(mir::Access::Mutable))
            }

            // owned fat references fuse into one unique carrier
            dir::Form::Owned
                if self
                    .lowerer
                    .is_reference_carrier(form.value, self.type_substitution)? =>
            {
                self.lower_reference(
                    mir::ReferenceKind::Unique,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Exclusive),
                    form.value,
                )
            }

            // owned value families hold their storage directly
            dir::Form::Owned => self.lower_pointee(form.value),

            dir::Form::Placed { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an explicit placement".to_string(),
            })?,
        }
    }

    /// Lower one reference layer over its stored payload.
    ///
    /// Slice payloads fuse with the layer into one fat descriptor.
    pub(in crate::lower) fn lower_reference(
        &mut self,
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        access: mir::Access,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let payload = self.lowerer.reduced_type(payload)?;

        // fuse dynamic payload references into the erased descriptor
        if let dir::Type::Dynamic(dynamic) = self.lowerer.ty(payload)? {
            let constraint = self.lower(dynamic.constraint)?;

            return Ok(self.tree.intern_type(mir::Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage: mir::Storage::Heap(mir::Space::Local),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // fuse callable environment references into the closure descriptor
        if let dir::Type::Function(function) = self.lowerer.ty(payload)? {
            let signature = self.lower_callable_signature(function.signature)?;
            let multiplicity = match function.multiplicity {
                dir::Multiplicity::Repeatable => mir::Multiplicity::Repeatable,
                dir::Multiplicity::Once => mir::Multiplicity::Once,
            };

            return Ok(self.tree.intern_type(mir::Type::Function {
                signature,
                multiplicity,
                kind,
                lifetime,
                storage: mir::Storage::Heap(mir::Space::Local),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // fuse unsized pointees with the layer into one fat descriptor
        if let Some(slice) = self.lowerer.slice_pointee(payload)? {
            let element = self.lower(slice.element)?;

            return Ok(self.tree.intern_type(mir::Type::Slice {
                kind,
                lifetime,
                element,
                storage: mir::Storage::Heap(mir::Space::Local),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // otherwise reference the lowered pointee thinly
        let pointee = self.lower_pointee(payload)?;

        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            storage: mir::Storage::Heap(mir::Space::Local),
            access,
            pointee,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one raw form into a process-local machine pointer.
    fn lower_pointer(
        &mut self,
        payload: dir::GlobalTypeId,
        access: mir::Access,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let pointee = self.lower_pointee(payload)?;

        Ok(self.tree.intern_type(mir::Type::Pointer {
            pointee,
            access,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one type as the pointee behind a reference or owner.
    ///
    /// The pointee is the declared storage for nominal families and the
    /// value form for value families.
    pub(in crate::lower) fn lower_pointee(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let ty = self.lowerer.ty(id)?;

        // store reference primitives as their representation classes
        if let Some((item, arguments)) = ModuleLowerer::representation_item(&ty) {
            let symbol = self.lowerer.language_item_symbol(item)?;

            return Ok(self.lower_nominal(symbol, &arguments)?.storage);
        }

        match ty {
            // store contextual this as the receiver's storage representation
            dir::Type::This => self.lower_receiver_storage(),
            // store nominals as their declared type
            dir::Type::Application(instance) => {
                let arguments = self
                    .lowerer
                    .types(id.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();
                let nominal = self.lower_nominal(instance.symbol, &arguments)?;

                Ok(nominal.storage)
            }
            // store anonymous object rows as their concrete struct
            dir::Type::Object(shape) => self.lower_object_struct(&shape, id.module_id),
            // hold value families as their value form
            _ => self.lower(id),
        }
    }

    /// Lower one non-form type at a narrowed access.
    fn lower_layer_value(
        &mut self,
        id: dir::GlobalTypeId,
        access: Option<mir::Access>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // reference families receive the access on their implicit managed layer
        if self
            .lowerer
            .has_indirect_representation(id, self.type_substitution)?
        {
            return self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                access.unwrap_or(mir::Access::Mutable),
                id,
            );
        }

        self.lower(id)
    }
}

impl ModuleLowerer<'_> {
    /// Return the outermost indirect layer of one value type, when one exists.
    pub(in crate::lower) fn peel_indirection(
        &self,
        id: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<Indirection>> {
        let id = type_substitution.resolve(self, id)?;

        match self.ty(id)? {
            // form layers select their indirection by constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Raw => Ok(Some(Indirection {
                    stored: type_substitution.resolve(self, form.value)?,
                    access: mir::Access::Mutable,
                })),
                dir::Form::Borrowed(borrow) => {
                    let Some(borrow) = self.types(id.module_id)?.borrow_form_maybe(borrow) else {
                        return Err(CompilerError::Internal {
                            message: "missing a borrow form".to_string(),
                        });
                    };
                    let access = self.borrow_access(borrow.access)?;

                    Ok(Some(Indirection {
                        stored: type_substitution.resolve(self, form.value)?,
                        access,
                    }))
                }
                // narrow the layer beneath views
                dir::Form::Readonly => {
                    let layer = self.peel_indirection(form.value, type_substitution)?;

                    Ok(layer.map(|layer| Indirection {
                        access: mir::Access::Readonly,
                        ..layer
                    }))
                }
                // owned fat references carry one exclusive unique layer
                dir::Form::Owned if self.is_reference_carrier(form.value, type_substitution)? => {
                    Ok(Some(Indirection {
                        stored: type_substitution.resolve(self, form.value)?,
                        access: mir::Access::Exclusive,
                    }))
                }
                // owned values and explicit placements hold storage directly
                dir::Form::Owned | dir::Form::Placed { .. } => Ok(None),
            },

            // nullable unions reference through their carrier
            dir::Type::Union(union) => {
                match self.decompose_nullish_union(id.module_id, &union, type_substitution)? {
                    Some((_, carrier)) => self.peel_indirection(carrier, type_substitution),
                    None => Ok(None),
                }
            }

            // bare reference families carry an implicit managed layer
            _ => match self.has_indirect_representation(id, type_substitution)? {
                true => Ok(Some(Indirection {
                    stored: id,
                    access: mir::Access::Mutable,
                })),
                false => Ok(None),
            },
        }
    }

    /// Return the storage base beneath one value's owner and view layers.
    pub(in crate::lower) fn peel_owned(
        &self,
        id: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = type_substitution.resolve(self, id)?;

        match self.ty(id)? {
            // store the payload of owners and views directly
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Readonly => {
                    self.peel_owned(form.value, type_substitution)
                }
                _ => Ok(id),
            },
            _ => Ok(id),
        }
    }

    /// Return whether one type has an indirect runtime representation.
    pub(in crate::lower) fn has_indirect_representation(
        &self,
        id: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        let id = type_substitution.resolve(self, id)?;

        match self.ty(id)? {
            // answer form layers by their outermost constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => Ok(true),
                // intrinsic fat owners retain a unique reference carrier
                dir::Form::Owned => self.is_reference_carrier(form.value, type_substitution),
                // views and placement answer for the layer beneath
                dir::Form::Readonly | dir::Form::Placed { .. } => {
                    self.has_indirect_representation(form.value, type_substitution)
                }
            },

            // nullable unions ride their reference carrier; tagged unions are values
            dir::Type::Union(union) => {
                match self.decompose_nullish_union(id.module_id, &union, type_substitution)? {
                    Some((_, carrier)) => {
                        self.has_indirect_representation(carrier, type_substitution)
                    }
                    None => Ok(false),
                }
            }

            // answer bare bases by their family default
            other => Ok(self.base_default_ownership(&other)? == dir::Ownership::Managed),
        }
    }

    /// Return whether one value has an intrinsic reference carrier.
    fn is_reference_carrier(
        &self,
        id: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        let id = type_substitution.resolve(self, id)?;
        let id = self.reduced_type(id)?;

        Ok(match self.ty(id)? {
            dir::Type::Dynamic(_) | dir::Type::Function(_) | dir::Type::Slice(_) => true,
            dir::Type::Application(instance) => matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::Tensor | dir::LanguageItem::TensorView)
            ),
            _ => false,
        })
    }

    /// Return the default ownership of one base type family.
    fn base_default_ownership(&self, base: &dir::Type) -> CompilerResult<dir::Ownership> {
        Ok(match base {
            // default reference families to managed
            dir::Type::Shape(_)
            | dir::Type::Array(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Slice(_)
            | dir::Type::Object(_) => dir::Ownership::Managed,

            // follow the declaration family for nominal defaults
            dir::Type::Application(instance) => match self.definition(instance.symbol)? {
                Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                    dir::Ownership::Managed
                }
                _ => dir::Ownership::Owned,
            },

            // hold string and bigint primitives through their representation classes
            dir::Type::Primitive(dir::PrimitiveType::String | dir::PrimitiveType::Bigint) => {
                dir::Ownership::Managed
            }

            // hold value families directly
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => dir::Ownership::Owned,

            // share the owner's representation for precise variants
            dir::Type::Variant(variant) => {
                let owner = self.ty(variant.owner)?;

                return self.base_default_ownership(&owner);
            }

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a default form for the '{}' type", other.variant_name()),
            })?,
        })
    }

    /// Split one union into its nullish bits and single reference carrier.
    ///
    /// Returns None when the union does not reduce to one nullable reference.
    pub(in crate::lower) fn decompose_nullish_union(
        &self,
        module: ModuleId,
        union: &dir::UnionType,
        type_substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<(mir::Nullability, dir::GlobalTypeId)>> {
        let mut nullability = (false, false);
        let mut carriers = Vec::new();
        for id in self.types(module)?.type_ids(union.elements) {
            match self.ty(*id)? {
                // ride the carrier's spare values for nullish members
                dir::Type::Null => nullability.0 = true,
                dir::Type::Undefined => nullability.1 = true,
                _ => carriers.push(*id),
            }
        }

        // require one reference member to carry the union
        let [carrier] = carriers.as_slice() else {
            return Ok(None);
        };
        let carrier = type_substitution.resolve(self, *carrier)?;
        if !self.has_indirect_representation(carrier, type_substitution)? {
            return Ok(None);
        }
        let nullability = match nullability {
            (true, true) => mir::Nullability::NullOrUndefined,
            (true, false) => mir::Nullability::Null,
            (false, true) => mir::Nullability::Undefined,
            (false, false) => {
                return Err(CompilerError::Internal {
                    message: "a single-member union".to_string(),
                });
            }
        };

        Ok(Some((nullability, carrier)))
    }
}

impl TypeLowerer<'_, '_> {
    /// Widen one lowered reference-like type with the nullish values it admits.
    pub(in crate::lower) fn insert_nullability(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        nullability: mir::Nullability,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let mut ty = self.tree.get(ty).clone();
        if !ty.set_nullability(nullability) {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a nullable union without a reference-like carrier".to_string(),
            }
            .into());
        }

        Ok(self.tree.intern_type(ty))
    }

    /// Insert one reference type over a pointee.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::Heap(mir::Space::Local),
            access,
            pointee,
            nullability: mir::Nullability::None,
        })
    }
}

impl ModuleLowerer<'_> {
    /// Return the access of one borrow access singleton.
    pub(in crate::lower) fn borrow_access(
        &self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Access> {
        let dir::MemoryLiteral::Access(access) =
            self.memory_literal(access, dir::MemoryParameter::Access)?
        else {
            return Err(CompilerError::Internal {
                message: "a borrow access in the wrong domain".to_string(),
            });
        };

        Ok(match access {
            dir::Access::Readonly => mir::Access::Readonly,
            dir::Access::Mutable => mir::Access::Mutable,
            dir::Access::Exclusive => mir::Access::Exclusive,
        })
    }
}
