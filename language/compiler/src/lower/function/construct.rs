use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, NominalField, constructor_receiver_type};
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
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        // super(...) initializes the current receiver through the base constructor
        if let dir::Expression::Call { left, .. } = *self.source().tree().get(expression)
            && matches!(*self.source().tree().get(left), dir::Expression::Super)
        {
            return self.lower_super_construct(resolution);
        }

        match &resolution.target {
            // Meters(5)
            dir::ConstructTarget::Newtype { .. } => self.lower_newtype_construct(resolution),
            // new User("ada")
            dir::ConstructTarget::Class {
                selection,
                constructor,
            } => self.lower_class_construct(resolution, selection, constructor),
            // new factory(1)
            dir::ConstructTarget::Dynamic { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a dynamically dispatched construction".to_string(),
            }
            .into()),
        }
    }

    /// Lower one super call initializing the current receiver.
    fn lower_super_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        let dir::ConstructTarget::Class {
            selection,
            constructor,
        } = &resolution.target
        else {
            return Err(CompilerError::Internal {
                message: "a super call outside a class constructor target".to_string(),
            });
        };
        let Some(this) = self.this else {
            return Err(CompilerError::Internal {
                message: "a super call without a receiver".to_string(),
            });
        };
        let this = self.read_binding(this);

        // resolve the base constructor behind the selection
        let symbol = match constructor {
            dir::ClassConstructor::Declared { symbol } => Some(*symbol),
            dir::ClassConstructor::Default => None,
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a {other:?} super constructor"),
                }
                .into());
            }
        };

        // a defaulted base runs its synthesized constructor when it stores initializers
        let target = match symbol {
            Some(symbol) => Some(symbol),
            None if self
                .lowerer
                .class_has_field_initializers(selection.symbol)? =>
            {
                Some(selection.symbol)
            }
            None => None,
        };
        let function = match target {
            Some(symbol) => {
                let key = self.selection_key(symbol, selection)?;

                Some(self.function(&key)?)
            }
            None => None,
        };

        // call the base constructor over the narrowed receiver
        if let Some(function) = function {
            let parameters = self.function_parameters(function);
            let Some((receiver, parameters)) = parameters.split_first() else {
                return Err(CompilerError::Internal {
                    message: "a base constructor without a declared receiver slot".to_string(),
                });
            };
            let receiver = self
                .builder
                .cast(mir::CastOperator::Bitcast, this, *receiver);
            let mut values = vec![receiver];
            values.extend(self.lower_call_arguments(&resolution.arguments, parameters, None)?);
            self.builder.call_function(function, values);
        }

        // store the derived field initializers once the base storage settles
        if let Some(owner) = self.constructs {
            self.lower_field_initializers(owner)?;
        }

        // a super call carries no value
        let void = self.builder.tree_mut().intern_type(mir::Type::Void);

        Ok(self.builder.constant(mir::Constant::Undefined, void))
    }

    /// Lower one class construction to an allocation and its constructor call.
    fn lower_class_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
        selection: &dir::Selection,
        constructor: &dir::ClassConstructor,
    ) -> CompilerResult<mir::Value> {
        // adapt the arguments against the declared constructor header
        let values = match constructor {
            dir::ClassConstructor::Declared { symbol } => {
                let key = self.selection_key(*symbol, selection)?;
                let function = self.function(&key)?;
                let parameters = self.function_parameters(function);
                let Some((_, parameters)) = parameters.split_first() else {
                    return Err(CompilerError::Internal {
                        message: "a constructor without a declared receiver slot".to_string(),
                    });
                };

                self.lower_call_arguments(&resolution.arguments, parameters, None)?
            }
            dir::ClassConstructor::Default => Vec::new(),
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a {other:?} constructor"),
                }
                .into());
            }
        };

        self.lower_class_instance(
            resolution.return_type,
            constructor,
            &selection.arguments,
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
        // allocate zeroed heap storage for managed destinations
        let (slot, storage) = if is_reference {
            (None, self.builder.new_zeroed(pointee, carrier))
        }
        // otherwise construct owned destinations in place inside a local slot
        else {
            let slot = self.builder.local(pointee, mir::Mutability::Mutable);
            let address = self.insert_reference(
                mir::ReferenceKind::Borrowed,
                mir::Access::Exclusive,
                pointee,
            );
            let address = self.builder.local_addr(slot, address);

            (Some(slot), address)
        };

        // initialize the storage through an exclusive borrow
        match constructor {
            dir::ClassConstructor::Declared { symbol } => {
                // select the declared instance from the substituted class arguments
                let bindings = self
                    .lowerer
                    .instance_bindings(generic_arguments, self.instance)?;
                let instance: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
                let key = self.generic_instance_key(*symbol, &instance)?;
                let function = self.function(&key)?;
                let receiver = self.constructed_receiver(storage, pointee);

                // bind the constructor arguments after the receiver
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(arguments);
                self.builder.call_function(function, values);
            }
            dir::ClassConstructor::Default => {
                // call the synthesized constructor of initializer-bearing classes
                let stored = match self.lowerer.peel_indirection(return_type)? {
                    Some(reference) => reference.stored,
                    None => self.lowerer.peel_owned(return_type)?,
                };
                let dir::Type::Application(application) = self.lowerer.ty(stored)? else {
                    return Err(CompilerError::Internal {
                        message: "a class construction outside an application type".to_string(),
                    });
                };
                let class = application.symbol;
                if self.lowerer.class_has_field_initializers(class)? {
                    let bindings = self
                        .lowerer
                        .instance_bindings(generic_arguments, self.instance)?;
                    let instance: Vec<_> =
                        bindings.iter().map(|binding| binding.argument).collect();
                    let key = self.generic_instance_key(class, &instance)?;
                    let function = self.function(&key)?;
                    let receiver = self.constructed_receiver(storage, pointee);
                    self.builder.call_function(function, vec![receiver]);
                }
            }
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

    /// Bitcast one constructed storage to its exclusive uninitialized receiver.
    fn constructed_receiver(
        &mut self,
        storage: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        let receiver = constructor_receiver_type(self.builder.tree_mut(), pointee);

        self.builder
            .cast(mir::CastOperator::Bitcast, storage, receiver)
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
        let ty = self.lowerer.peel_owned(ty)?;
        let dir::Type::Application(_) = self.lowerer.ty(ty)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural object construction".to_string(),
            }
            .into());
        };
        let application = ty;
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
                // evaluate the declared initializer for omitted fields
                if field.initializer.is_some() {
                    if let Some(value) =
                        self.lower_omitted_field(application, storage, index, &field)?
                    {
                        ordered.push(value);
                    }

                    continue;
                }

                // store the undefined case for absent optional fields
                if field.is_optional {
                    ordered.push(self.lower_absent_property(storage, index)?);

                    continue;
                }

                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an omitted field without an initializer".to_string(),
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
            let carrier = self.property_carrier(storage, index)?;
            ordered.push(self.adapt_to_carrier(value, carrier)?);
        }

        Ok(self.builder.aggregate(ty, ordered))
    }

    /// Store the declared field initializers through one constructor receiver.
    pub(in crate::lower) fn lower_field_initializers(
        &mut self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(definition) = self.lowerer.definition(owner)? else {
            return Err(CompilerError::Internal {
                message: "a constructor body without its class definition".to_string(),
            });
        };
        // offset own initializers past the base's chained fields
        let total = self.lowerer.nominal_fields(owner)?.len();
        let own = self.lowerer.instance_fields(definition.members());
        let inherited = total
            .checked_sub(own.len())
            .ok_or_else(|| CompilerError::Internal {
                message: "a class chaining fewer fields than it declares".to_string(),
            })?;
        if own.iter().all(|field| field.initializer.is_none()) {
            return Ok(());
        }

        // address the constructed storage behind the receiver
        let Some(this) = self.this else {
            return Err(CompilerError::Internal {
                message: "a constructor body without a receiver".to_string(),
            });
        };
        let this = self.read_binding(this);
        let reference = self.value_carrier(this)?;
        let mir::Type::Reference { pointee, .. } =
            self.builder.tree().get(mir::TypeId::from(reference))
        else {
            return Err(CompilerError::Internal {
                message: "a constructor receiver outside a reference".to_string(),
            });
        };
        let (concrete, _) = self.builder.tree().split_lifetime_application(*pointee);

        for (index, field) in own.iter().enumerate() {
            let index = inherited + index;
            let Some(initializer) = field.initializer else {
                continue;
            };
            if initializer.module_id != self.source {
                return Err(CompilerError::Internal {
                    message: "a field initializer declared outside its class module".to_string(),
                });
            }
            let expression = initializer
                .local_id
                .try_into_typed::<dir::Expression>()
                .map_err(|message| CompilerError::Internal { message })?;

            // skip singleton literal initializers storing no runtime value
            let carrier = self.property_carrier(concrete, index)?;
            let is_void = matches!(self.builder.tree().get(carrier), mir::Type::Void);
            if is_void && self.is_literal_initializer(self.source, expression)? {
                continue;
            }

            // evaluate the initializer outside the constructor's bindings, keeping this live
            let values = std::mem::take(&mut self.values);
            let frames = std::mem::take(&mut self.frames);
            let value = self.lower_expression(expression);
            self.values = values;
            self.frames = frames;
            let value = value?;
            if is_void {
                continue;
            }

            // store the initialized value at its declared carrier
            let value = self.adapt_to_carrier(value, carrier)?;
            let address =
                self.emit_field_address(this, index as u32, carrier, mir::Access::Exclusive);
            self.builder.store(address, value);
        }

        Ok(())
    }

    /// Return whether one initializer is a scalar literal computing nothing.
    fn is_literal_initializer(
        &self,
        module: destack_source::ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let Some(declaring) = self.lowerer.modules.get(&module) else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside its loaded module".to_string(),
            });
        };

        Ok(matches!(
            declaring.tree().get(expression),
            dir::Expression::ScalarLiteral(_)
        ))
    }

    /// Lower one omitted field through its declared initializer.
    fn lower_omitted_field(
        &mut self,
        application: dir::GlobalTypeId,
        storage: mir::LocalNodeId<mir::Type>,
        index: usize,
        field: &NominalField,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(initializer) = field.initializer else {
            return Err(CompilerError::Internal {
                message: "an omitted field lowered without a declared initializer".to_string(),
            });
        };
        let expression = initializer
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;

        // skip singleton literal initializers storing no runtime value
        let carrier = self.property_carrier(storage, index)?;
        let is_void = matches!(self.builder.tree().get(carrier), mir::Type::Void);
        if is_void && self.is_literal_initializer(initializer.module_id, expression)? {
            return Ok(None);
        }

        // evaluate the initializer inside the constructed instance
        let dir::Type::Application(instance) = self.lowerer.ty(application)? else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside an application type".to_string(),
            });
        };
        let arguments = self
            .lowerer
            .types(application.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        let specialization = self
            .lowerer
            .specialization_of(instance.symbol, &arguments)?;
        let value =
            self.lower_foreign_expression(initializer.module_id, specialization, expression)?;

        Ok(Some(self.adapt_to_carrier(value, carrier)?))
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
        let (base, _) = self.builder.tree().split_lifetime_application(carrier);
        let (concrete, reference) = match self.builder.tree().get(base) {
            mir::Type::Reference { pointee, .. } => (*pointee, Some(carrier)),
            mir::Type::Struct { .. } => (base, None),
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
                    let carrier = self.property_carrier(concrete, index)?;
                    values.push(self.adapt_to_carrier(value, carrier)?);
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

    /// Adapt one lowered value into its declared carrier.
    pub(in crate::lower) fn adapt_to_carrier(
        &mut self,
        value: mir::Value,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // store a value already at its carrier type directly
        let value_type = self.value_carrier(value)?;
        if value_type == carrier {
            return Ok(value);
        }

        // classify through proof-only lifetime applications
        let (value_base, _) = self.builder.tree().split_lifetime_application(value_type);
        let (carrier_base, _) = self.builder.tree().split_lifetime_application(carrier);
        if value_base == carrier_base {
            return Ok(value);
        }

        // pass distinct identities sharing one structure unchanged
        if self.builder.tree().types_equal(
            mir::TypeId::from(value_base),
            mir::TypeId::from(carrier_base),
        ) {
            return Ok(value);
        }

        // store no value in erased zero-sized carriers such as variant tags
        if matches!(self.builder.tree().get(carrier_base), mir::Type::Void) {
            return Ok(self
                .builder
                .constant(mir::Constant::Undefined, carrier_base));
        }

        // reinterpret reference-family kind changes over the shared fat carrier
        if self.builder.tree().get(carrier_base).is_reference_carrier() {
            // pass heads equal under erased lifetimes unchanged
            let source = self.builder.tree().get(value_base).erased_lifetime();
            let target = self.builder.tree().get(carrier_base).erased_lifetime();
            if source == target {
                return Ok(value);
            }

            return Ok(self
                .builder
                .cast(mir::CastOperator::Bitcast, value, carrier));
        }

        // select the carrier case the value type declares
        let mir::Type::Variant { cases, .. } = self.builder.tree().get(carrier_base) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "an adapted value at '{:?}' misses its declared '{:?}' carrier",
                    self.builder.tree().get(value_type),
                    self.builder.tree().get(carrier)
                ),
            });
        };
        let Some(case) = cases.iter().position(|case| {
            case.ty == mir::TypeId::from(value_base)
                || self
                    .builder
                    .tree()
                    .types_equal(case.ty, mir::TypeId::from(value_type))
        }) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "an adapted value {value_type:?} selects no case of carrier {carrier:?}"
                ),
            });
        };

        Ok(self
            .builder
            .variant_new(carrier_base, case as u32, Some(value)))
    }

    /// Materialize the undefined case of one absent optional property.
    fn lower_absent_property(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let carrier = self.property_carrier(concrete, index)?;

        self.absent_carrier_value(carrier)
    }

    /// Materialize the undefined case of one optional carrier.
    pub(in crate::lower) fn absent_carrier_value(
        &mut self,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        match self.builder.tree().undefined_case(carrier) {
            // select the void case of a variant carrier
            Some(mir::NullishCase::Case(case)) => Ok(self.builder.variant_new(carrier, case, None)),
            // store undefined directly in niched nullable carriers
            Some(mir::NullishCase::Niche) => {
                Ok(self.builder.constant(mir::Constant::Undefined, carrier))
            }
            None => Err(CompilerError::Internal {
                message: "an optional value lowered without its undefined case".to_string(),
            }),
        }
    }

    /// Return the value one omitted argument passes at its carrier.
    pub(in crate::lower) fn absent_argument_value(
        &mut self,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        match self.builder.tree().undefined_case(carrier) {
            // select the void case of a variant carrier
            Some(mir::NullishCase::Case(case)) => self.builder.variant_new(carrier, case, None),
            // niched and headerless carriers take the bare placeholder
            Some(mir::NullishCase::Niche) | None => {
                self.builder.constant(mir::Constant::Undefined, carrier)
            }
        }
    }

    /// Return the declared carrier type of one concrete class property.
    fn property_carrier(
        &self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let tree = self.builder.tree();

        // unwrap uninitialized storage to the layout its value holds
        let mut concrete = concrete;
        while let mir::Type::Uninit { value } = tree.get(concrete) {
            concrete = *value;
        }

        // read the field carrier out of the struct layout
        let mir::Type::Struct { fields, .. } = tree.get(concrete) else {
            return Err(CompilerError::Internal {
                message: "dynamic constraint lowered outside a struct".to_string(),
            });
        };

        Ok(tree.get(fields[index]).ty)
    }
}
