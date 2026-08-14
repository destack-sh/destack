use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;

use crate::{CompilerError, CompilerResult, LowerError};

/// One declared field an object literal constructs.
struct ConstructionField {
    /// The field key.
    key: dir::StaticKey,
    /// Whether the field may be absent from the literal.
    is_optional: bool,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one construct call through its construct resolution.
    pub(in crate::lower) fn lower_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        match &resolution.target {
            // Meters(5)
            dir::ConstructTarget::Newtype(_) => self.lower_newtype_construct(resolution),
            // new User("ada")
            dir::ConstructTarget::Class(candidate) => {
                self.lower_class_construct(resolution, candidate)
            }
            // new factory(1)
            dir::ConstructTarget::Dynamic { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a dynamically dispatched construction".to_string(),
            }
            .into()),
        }
    }

    /// Lower one class construction to an allocation and its constructor call.
    fn lower_class_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
        candidate: &dir::ClassConstructCandidate,
    ) -> CompilerResult<mir::Value> {
        // lower the constructor arguments in declaration order
        let mut values = Vec::with_capacity(resolution.arguments.len());
        for binding in &resolution.arguments {
            let dir::ArgumentSource::Provided(source) = binding.source else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted or spread argument".to_string(),
                }
                .into());
            };
            values.push(self.lower_argument(source)?);
        }

        self.lower_class_instance(
            resolution.return_type,
            &candidate.constructor,
            &candidate.generic_arguments,
            values,
        )
    }

    /// Lower one class construction over its evaluated argument values.
    pub(in crate::lower) fn lower_class_instance(
        &mut self,
        return_type: dir::GlobalTypeId,
        constructor: &dir::ClassConstructor,
        generic_arguments: &[dir::GenericArgumentBinding],
        arguments: Vec<mir::Value>,
    ) -> CompilerResult<mir::Value> {
        // lower the return form and its class representation
        let carrier = self.lower_type(return_type)?;
        let nominal = self.lower_nominal(return_type)?;
        let pointee = nominal.storage;

        // decide where the instance stores from the return form
        let is_reference = matches!(
            self.builder.tree().get(carrier),
            mir::Type::Reference { .. }
        );
        let (slot, storage) = match is_reference {
            // allocate zeroed heap storage for managed destinations
            true => (None, self.builder.new_zeroed(pointee, carrier)),
            // construct owned destinations in place inside a local slot
            false => {
                let slot = self.builder.local(pointee, mir::Mutability::Mutable);
                let address = self.insert_reference(
                    mir::ReferenceKind::Borrowed,
                    mir::Access::Exclusive,
                    pointee,
                );
                let address = self.builder.local_addr(slot, address);

                (Some(slot), address)
            }
        };

        // initialize the storage through an exclusive borrow
        match constructor {
            dir::ClassConstructor::Declared { symbol } => {
                // select the declared instance from the substituted class arguments
                let bindings = self
                    .lowerer
                    .instance_bindings(generic_arguments, &self.type_substitution)?;
                let instance: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
                let key = self.generic_instance_key(*symbol, &instance)?;
                let function = self.function(&key)?;

                // borrow heap references exclusively
                let exclusive = self.insert_reference(
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
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(arguments);
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

        // read the initialized slot back out for owned constructions
        let object = match slot {
            Some(slot) => self.builder.local_get(slot),
            None => storage,
        };

        Ok(object)
    }

    /// Lower one newtype construction to a single-value aggregate.
    fn lower_newtype_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        // lower the newtype named by the construct resolution
        let ty = self.lower_type(resolution.return_type)?;

        // wrap the single bound raw value
        let [binding] = resolution.arguments.as_slice() else {
            return Err(CompilerError::Internal {
                message: "multiple arguments bound to one newtype construction".to_string(),
            });
        };
        let dir::ArgumentSource::Provided(source) = binding.source else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a defaulted newtype argument".to_string(),
            }
            .into());
        };

        // detect singleton backings carrying no runtime payload
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
        // lower the constructed nominal beneath its owner and view forms
        let ty = self.node_type_id(expression)?;
        let ty = self.lowerer.peel_owned(ty, &self.type_substitution)?;
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
        let fields = nominal.fields.clone();

        // gather each property value under its field name
        let mut values = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { name, value, .. } = self.source().tree().get(*property)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a method or spread property".to_string(),
                }
                .into());
            };

            // key each written value by its property name
            values.push((dir::StaticKey::from(*name), *value));
        }

        // collect the field storage types behind the nominal, peeling lifetime applications
        let (storage, _) = self.builder.tree().split_lifetime_application(ty);
        let storage_fields = match self.builder.tree().get(storage) {
            mir::Type::Struct { fields, .. } => fields.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a nominal lowered without struct storage".to_string(),
                });
            }
        };

        // lower the field values in declaration order, eliding void storage
        let mut ordered = Vec::with_capacity(fields.len());
        for (index, field) in fields.into_iter().enumerate() {
            let Some((_, value)) = values.iter().find(|(key, _)| *key == field.key) else {
                // store the undefined case for absent optional fields
                if field.is_optional {
                    ordered.push(self.lower_absent_property(storage, index)?);

                    continue;
                }

                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted struct field".to_string(),
                }
                .into());
            };

            // skip singleton literal fields storing no runtime value
            let field_storage = storage_fields
                .get(index)
                .map(|field| self.builder.tree().get(*field).ty);
            let is_void = field_storage
                .is_some_and(|ty| matches!(self.builder.tree().get(ty), mir::Type::Void));
            let is_literal = matches!(
                self.source().tree().get(*value),
                dir::Expression::ScalarLiteral(_)
            );
            if is_void && is_literal {
                continue;
            }

            // store the written value at the field's own case
            let value = self.lower_expression(*value)?;
            ordered.push(self.lower_property_case(storage, index, value)?);
        }

        Ok(self.builder.aggregate(ty, ordered))
    }

    /// Lower one tuple expression to an aggregate value.
    pub(in crate::lower) fn lower_tuple_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<mir::Value> {
        // lower the tuple carrier from the node type
        let ty = self.node_type_id(expression)?;
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

    /// Lower one object literal to its concrete class instance.
    pub(in crate::lower) fn lower_object_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<mir::Value> {
        // read the class from the written expectation, falling back to the literal's own type
        let own = self.node_type_id(expression)?;
        let mut committed = self.expected_type_id(expression).unwrap_or(own);
        let mut declared = self.construction_fields(committed)?;
        if declared.is_none() && committed != own {
            committed = own;
            declared = self.construction_fields(own)?;
        }
        let Some(declared) = declared else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an object literal outside its concrete class".to_string(),
            }
            .into());
        };

        // gather each written value under its property name
        let mut written = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { name, value, .. } = self.source().tree().get(*property)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a method or spread object property".to_string(),
                }
                .into());
            };
            written.push((dir::StaticKey::from(*name), *value));
        }

        // lower the declared carrier, construction fills its struct storage
        let carrier = self.lower_type(committed)?;
        let (concrete, reference) = match self.builder.tree().get(carrier) {
            mir::Type::Reference { pointee, .. } => (*pointee, Some(carrier)),
            mir::Type::Struct { .. } => (carrier, None),
            _ => {
                return Err(CompilerError::Internal {
                    message: "an object class outside its struct or reference carrier".to_string(),
                });
            }
        };

        // build the concrete instance in declaration order
        let mut values = Vec::with_capacity(declared.len());
        for (index, field) in declared.iter().enumerate() {
            match written.iter().find(|(key, _)| *key == field.key) {
                Some((_, value)) => {
                    // skip singleton literal properties storing no runtime value
                    let carrier = self.property_carrier(concrete, index)?;
                    let is_void = matches!(self.builder.tree().get(carrier), mir::Type::Void);
                    let is_literal = matches!(
                        self.source().tree().get(*value),
                        dir::Expression::ScalarLiteral(_)
                    );
                    if is_void && is_literal {
                        values.push(self.builder.constant(mir::Constant::Undefined, carrier));

                        continue;
                    }

                    let value = self.lower_expression(*value)?;
                    values.push(self.lower_property_case(concrete, index, value)?);
                }
                // store the undefined case for absent optional properties
                None if field.is_optional => {
                    values.push(self.lower_absent_property(concrete, index)?);
                }
                None => {
                    return Err(CompilerError::Internal {
                        message: "an absent required property".to_string(),
                    });
                }
            }
        }

        // hand the instance out at its declared carrier
        let aggregate = self.builder.aggregate(concrete, values);

        Ok(match reference {
            // allocate managed destinations on the heap
            Some(reference) => self.builder.new_complete(aggregate, reference),
            // owned destinations hold the struct in place
            None => aggregate,
        })
    }

    /// Return the construction fields one committed object type declares.
    fn construction_fields(
        &mut self,
        committed: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<ConstructionField>>> {
        let mut class = committed;
        loop {
            match self.lowerer.ty(class)? {
                // step through the memory forms around the value
                dir::Type::Form(form) => class = form.value,
                // read a nominal's fields, following alias applications on the way
                dir::Type::Application(instance) => {
                    let aliased = match self.lowerer.definition(instance.symbol)? {
                        Some(dir::Definition::TypeAlias(alias)) => alias.value,
                        Some(dir::Definition::Class(_) | dir::Definition::Struct(_)) => {
                            let fields = self.lowerer.nominal_fields(instance.symbol)?;

                            return Ok(Some(
                                fields
                                    .iter()
                                    .map(|field| ConstructionField {
                                        key: field.key,
                                        is_optional: field.is_optional,
                                    })
                                    .collect(),
                            ));
                        }
                        _ => return Ok(None),
                    };
                    class = aliased;
                }
                // read the properties an anonymous class declares
                dir::Type::Object(shape) => {
                    let properties = self
                        .lowerer
                        .types(class.module_id)?
                        .properties(shape.properties)
                        .to_vec();

                    return Ok(Some(
                        properties
                            .iter()
                            .map(|property| ConstructionField {
                                key: property.key,
                                is_optional: property.is_optional,
                            })
                            .collect(),
                    ));
                }
                // stop at every other type
                _ => return Ok(None),
            }
        }
    }

    /// Inject one written value into its declared property carrier.
    fn lower_property_case(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // store a value already at its carrier type directly
        let carrier = self.property_carrier(concrete, index)?;
        let Some(value_type) = self.builder.value_type(value) else {
            return Err(CompilerError::Internal {
                message: "the lowered property value has no type".to_string(),
            });
        };
        if value_type == carrier {
            return Ok(value);
        }

        // store no value in erased zero-sized carriers such as variant tags
        if matches!(self.builder.tree().get(carrier), mir::Type::Void) {
            return Ok(self.builder.constant(mir::Constant::Undefined, carrier));
        }

        // widen values into niched nullable carriers as a kind change
        if let mir::Type::Reference { .. } | mir::Type::Slice { .. } | mir::Type::Dynamic { .. } =
            self.builder.tree().get(carrier)
        {
            return Ok(self
                .builder
                .cast(mir::CastOperator::Bitcast, value, carrier));
        }

        // select the carrier case the value type declares
        let mir::Type::Variant { cases, .. } = self.builder.tree().get(carrier) else {
            return Err(CompilerError::Internal {
                message: "lowered property value misses its declared carrier".to_string(),
            });
        };
        let Some(case) = cases.iter().position(|case| {
            case.ty == mir::TypeId::from(value_type)
                || self
                    .builder
                    .tree()
                    .types_equal(case.ty, mir::TypeId::from(value_type))
        }) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "lowered property value {value_type:?} selects no case of carrier {carrier:?}"
                ),
            });
        };

        Ok(self.builder.variant_new(carrier, case as u32, Some(value)))
    }

    /// Materialize the undefined case of one absent optional property.
    fn lower_absent_property(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        // store undefined directly in niched nullable carriers
        let carrier = self.property_carrier(concrete, index)?;
        if let mir::Type::Reference { .. }
        | mir::Type::Slice { .. }
        | mir::Type::Dynamic { .. }
        | mir::Type::Function { .. } = self.builder.tree().get(carrier)
        {
            return Ok(self.builder.constant(mir::Constant::Undefined, carrier));
        }

        // select the void case of the optional property's carrier
        let tree = self.builder.tree();
        let mir::Type::Variant { cases, .. } = tree.get(carrier) else {
            return Err(CompilerError::Internal {
                message: "an optional property lowered without its undefined case".to_string(),
            });
        };
        let Some(undefined) = cases
            .iter()
            .position(|case| matches!(tree.get(case.ty), mir::Type::Void))
        else {
            return Err(CompilerError::Internal {
                message: "an optional property lowered without its undefined case".to_string(),
            });
        };

        Ok(self.builder.variant_new(carrier, undefined as u32, None))
    }

    /// Return the declared carrier type of one concrete class property.
    fn property_carrier(
        &self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let tree = self.builder.tree();
        let mir::Type::Struct { fields, .. } = tree.get(concrete) else {
            return Err(CompilerError::Internal {
                message: "dynamic constraint lowered outside a struct".to_string(),
            });
        };

        Ok(tree.get(fields[index]).ty)
    }
}
