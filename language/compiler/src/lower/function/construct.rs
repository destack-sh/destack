use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, GenericInstanceKey};

use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one construct call through its checked construct resolution.
    pub(in crate::lower) fn lower_construct(
        &mut self,
        resolution: &dir::ConstructResolution,
    ) -> CompilerResult<mir::Value> {
        match &resolution.target {
            // Meters(5)
            dir::ConstructTarget::Newtype(_) => self.lower_newtype_construct(resolution),
            // new User("ada")
            dir::ConstructTarget::Class(candidate) => {
                self.lower_class_construct(resolution, candidate)
            }
            // Shape.Circle(2.0)
            dir::ConstructTarget::Variant(_) => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an enum variant construction".to_string(),
            }
            .into()),
        }
    }

    /// Lower one class construction to an allocation and its constructor call.
    fn lower_class_construct(
        &mut self,
        resolution: &dir::ConstructResolution,
        candidate: &dir::ClassConstructCandidate,
    ) -> CompilerResult<mir::Value> {
        // lower the return form and its class representation
        let carrier = self.lower_type(resolution.return_type)?;
        let nominal = self.lower_nominal(resolution.return_type)?;
        let pointee = nominal.storage;

        // the checked return form decides where the instance stores
        let is_reference = matches!(
            self.builder.tree().get(carrier),
            mir::Type::Reference { .. }
        );
        let (slot, storage) = match is_reference {
            // managed destinations allocate zeroed heap storage
            true => (None, self.builder.new_zeroed(pointee, carrier)),
            // owned destinations construct in place inside a local slot
            false => {
                let slot = self.builder.local(pointee, mir::Mutability::Mutable);
                let address = self.type_lowerer().insert_reference(
                    mir::ReferenceKind::Borrowed,
                    mir::Access::Exclusive,
                    pointee,
                );
                let address = self.builder.local_addr(slot, address);

                (Some(slot), address)
            }
        };

        // constructors initialize the storage through an exclusive borrow
        match &candidate.constructor {
            dir::ClassConstructor::Declared { symbol } => {
                let key = GenericInstanceKey::non_generic(*symbol);
                let Some(function) = self.lowerer.functions.get(&key).copied() else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR is missing a declared constructor function"
                            .to_string(),
                    });
                };
                // heap references borrow exclusively; local addresses already do
                let exclusive = self.type_lowerer().insert_reference(
                    mir::ReferenceKind::Borrowed,
                    mir::Access::Exclusive,
                    pointee,
                );
                let receiver = match is_reference {
                    true => self
                        .builder
                        .cast(mir::CastOperator::Bitcast, storage, exclusive),
                    false => storage,
                };

                // bind the constructor arguments after the receiver
                let mut values = Vec::with_capacity(resolution.arguments.len() + 1);
                values.push(receiver);
                for binding in &resolution.arguments {
                    let dir::ArgumentSource::Provided(source) = binding.argument else {
                        return Err(LowerError::Unsupported {
                            anchor: self.lowerer.module.into(),
                            construct: "a defaulted or spread argument".to_string(),
                        }
                        .into());
                    };
                    values.push(self.lower_argument(source)?);
                }
                self.builder.call_function(function, values);
            }
            dir::ClassConstructor::Default => {}
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a {other:?} constructor"),
                }
                .into());
            }
        }

        // owned constructions read the initialized slot back out
        let object = match slot {
            Some(slot) => self.builder.local_get(slot),
            None => storage,
        };

        Ok(object)
    }

    /// Lower one newtype construction to a single-value aggregate.
    fn lower_newtype_construct(
        &mut self,
        resolution: &dir::ConstructResolution,
    ) -> CompilerResult<mir::Value> {
        // the construct resolution names the newtype before contextual coercion
        let ty = self.lower_type(resolution.return_type)?;

        // wrap the single bound raw value
        let [binding] = resolution.arguments.as_slice() else {
            return Err(CompilerError::Internal {
                message: "checked DIR bound multiple arguments to one newtype construction"
                    .to_string(),
            });
        };
        let dir::ArgumentSource::Provided(source) = binding.argument else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a defaulted newtype argument".to_string(),
            }
            .into());
        };

        // singleton backings carry no runtime argument payload
        let inner_is_void = match self.builder.tree().get(ty) {
            mir::Type::Newtype { inner, .. } => {
                matches!(self.builder.tree().get(*inner), mir::Type::Void)
            }
            _ => false,
        };
        if inner_is_void {
            self.lower_argument(source)?;

            return Ok(self.builder.aggregate(ty, Vec::new()));
        }
        let value = self.lower_argument(source)?;

        Ok(self.builder.aggregate(ty, vec![value]))
    }

    /// Lower one struct expression to an aggregate value.
    pub(in crate::lower) fn lower_struct_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<mir::Value> {
        // the checked node type names the constructed nominal
        let ty = self.coerced_type_id(expression)?;
        let dir::Type::Application(_) = self.lowerer.ty(ty)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural object construction".to_string(),
            }
            .into());
        };
        let nominal = self.lower_nominal(ty)?;
        let ty = self.lower_type(ty)?;
        let nominal = self.lowerer.nominal(&nominal.key)?;
        let fields = nominal
            .fields
            .iter()
            .map(|field| field.key)
            .collect::<Vec<_>>();

        // gather each property value under its field key
        let mut values = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { key, value, .. } = self.source().tree().get(*property)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a method or spread property".to_string(),
                }
                .into());
            };

            let dir::Key::Name(name) = key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a computed property key".to_string(),
                }
                .into());
            };

            values.push((name.static_key(), *value));
        }

        // lower the field values in declaration order
        let mut ordered = Vec::with_capacity(fields.len());
        for field in fields {
            let Some((_, value)) = values.iter().find(|(key, _)| *key == field) else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted struct field".to_string(),
                }
                .into());
            };

            ordered.push(self.lower_expression(*value)?);
        }

        Ok(self.builder.aggregate(ty, ordered))
    }

    /// Lower one tuple expression to an aggregate value.
    pub(in crate::lower) fn lower_tuple_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<mir::Value> {
        // the checked node type names the tuple carrier
        let ty = self.coerced_type_id(expression)?;
        let ty = self.lower_type(ty)?;

        // lower the element values in order
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            let dir::Argument::Positional { value } = self.source().tree().get(*element) else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a spread tuple element".to_string(),
                }
                .into());
            };
            values.push(self.lower_expression(*value)?);
        }

        Ok(self.builder.aggregate(ty, values))
    }
}
