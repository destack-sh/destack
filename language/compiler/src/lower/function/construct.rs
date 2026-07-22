use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;

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
        // the checked return form decides where the instance stores
        let carrier = self
            .lowerer
            .lower_type_id(self.builder.tree_mut(), resolution.return_type)?;
        let symbol = self.lowerer.resolve_symbol_alias(candidate.symbol)?;
        let Some(nominal) = self.lowerer.nominals.get(&symbol) else {
            return Err(CompilerError::Internal {
                message: "class return type lowered without its nominal declaration".to_string(),
            });
        };
        let pointee = nominal.ty;
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
                let address = self.exclusive_borrow(pointee);
                let address = self.builder.local_addr(slot, address);

                (Some(slot), address)
            }
        };

        // constructors initialize the storage through an exclusive borrow
        match &candidate.constructor {
            dir::ClassConstructor::Declared { symbol } => {
                let Some(function) = self.lowerer.functions.get(&(*symbol, Vec::new())).copied()
                else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR is missing a declared constructor function"
                            .to_string(),
                    });
                };
                // heap references borrow exclusively; local addresses already do
                let exclusive = self.exclusive_borrow(pointee);
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

    /// Insert one exclusive borrow over construction storage.
    fn exclusive_borrow(
        &mut self,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.lowerer.insert_reference(
            self.builder.tree_mut(),
            mir::ReferenceKind::Borrowed,
            mir::Access::Exclusive,
            pointee,
        )
    }

    /// Lower one newtype construction to a single-value aggregate.
    fn lower_newtype_construct(
        &mut self,
        resolution: &dir::ConstructResolution,
    ) -> CompilerResult<mir::Value> {
        // the construct resolution names the newtype before contextual coercion
        let ty = self
            .lowerer
            .lower_type_id(self.builder.tree_mut(), resolution.return_type)?;

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
        let dir::Type::Instance(instance) = self.lowerer.coerced_type(expression)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural object construction".to_string(),
            }
            .into());
        };
        let nominal = self
            .lowerer
            .lower_nominal(self.builder.tree_mut(), &instance)?;
        let (ty, fields) = (
            nominal.ty,
            nominal
                .fields
                .iter()
                .map(|field| field.key)
                .collect::<Vec<_>>(),
        );

        // gather each property value under its field key
        let mut values = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { key, value, .. } =
                self.lowerer.source().tree().get(*property)
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
        let ty = self.lowerer.coerced_type_id(expression)?;
        let ty = self.lowerer.lower_type_id(self.builder.tree_mut(), ty)?;

        // lower the element values in order
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            let dir::Argument::Positional { value } = self.lowerer.source().tree().get(*element)
            else {
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
