use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    FunctionLowerer, NominalField, constructor_receiver_type, nominal_receiver_storage,
};
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
        // initialize the current receiver through a super call
        if let dir::Expression::Call { left, .. } = *self.source().tree().get(expression)
            && matches!(*self.source().tree().get(left), dir::Expression::Super)
        {
            return self.lower_super_construct(resolution);
        }

        // lower by the construct target the resolution names
        match &resolution.target {
            // wrap a raw value in its newtype
            dir::ConstructTarget::Newtype { .. } => self.lower_newtype_construct(resolution),
            // construct a declared class
            dir::ConstructTarget::Class { key, constructor } => {
                self.lower_class_construct(resolution, key, constructor)
            }
            // reject construction through a class value
            dir::ConstructTarget::Dynamic { .. } => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
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
        // require a class construct target
        let dir::ConstructTarget::Class {
            key: selection,
            constructor,
        } = &resolution.target
        else {
            return Err(CompilerError::Internal {
                message: "a super call outside a class constructor target".to_string(),
            });
        };

        // read the receiver the base constructor initializes
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
                    anchor: self.lower.module.into(),
                    construct: format!("a {} super constructor", other.name()),
                }
                .into());
            }
        };

        // run the synthesized constructor of a defaulted base that stores initializers
        let target = match symbol {
            Some(symbol) => Some(symbol),
            None if self.lower.class_has_field_initializers(selection.symbol)? => {
                Some(selection.symbol)
            }
            None => None,
        };

        // resolve the target constructor at the selected instance
        let function = match target {
            Some(symbol) => {
                // declare the synthesized constructor a defaulted base stands in for
                if symbol == selection.symbol {
                    self.ensure_default_constructor(selection.symbol, &selection.arguments)?;
                }
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

        // yield the void value a super call produces
        let void = self.builder.tree_mut().intern_type(mir::Type::Void);

        Ok(self.builder.constant(mir::Constant::Undefined, void))
    }

    /// Lower one class construction to an allocation and its constructor call.
    fn lower_class_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
        selection: &dir::InstanceKey,
        constructor: &dir::ClassConstructor,
    ) -> CompilerResult<mir::Value> {
        // adapt the arguments against the declared constructor header
        let values = match constructor {
            // bind the written arguments to the declared parameters after the receiver
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
            // pass nothing to a defaulted constructor
            dir::ClassConstructor::Default => Vec::new(),
            // reject every remaining constructor kind
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
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
        let representation = self.lower_type(return_type)?;
        let nominal = self.lower_nominal(return_type)?;
        let pointee = nominal.storage;

        // decide where the instance stores from the return form
        let is_reference = matches!(
            self.builder.tree().get(representation),
            mir::Type::Reference { .. }
        );

        // allocate zeroed heap storage for managed destinations
        let (slot, storage) = if is_reference {
            (None, self.builder.new_zeroed(pointee, representation))
        }
        // otherwise construct owned destinations in place inside a local slot
        else {
            let slot = self.builder.local(pointee, mir::Mutability::Mutable);
            let address = self.insert_reference(
                mir::ReferenceKind::Borrowed,
                mir::Access::Exclusive,
                mir::Storage::Frame,
                pointee,
            );
            let address = self.builder.local_addr(slot, address);

            (Some(slot), address)
        };

        // initialize the storage through an exclusive borrow
        match constructor {
            // call the constructor the class declares
            dir::ClassConstructor::Declared { symbol } => {
                // select the declared instance from the substituted class arguments
                let bindings = self
                    .lower
                    .instance_bindings(generic_arguments, self.instance)?;
                let instance: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
                let key = self.generic_instance_key(*symbol, None, &instance)?;
                let function = self.function(&key)?;
                let receiver = self.constructed_receiver(storage, pointee)?;

                // bind the constructor arguments after the receiver
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(arguments);
                self.builder.call_function(function, values);
            }
            // call the synthesized constructor a defaulted class stands in for
            dir::ClassConstructor::Default => {
                // peel the return form down to the constructed class
                let stored = match self.lower.peel_indirection(return_type)? {
                    Some(reference) => reference.stored,
                    None => self.lower.peel_owned(return_type)?,
                };
                let dir::Type::Application(application) = self.lower.ty(stored)? else {
                    return Err(CompilerError::Internal {
                        message: "a class construction outside an application type".to_string(),
                    });
                };

                // synthesize the call only where the class stores initializers
                let class = application.symbol;
                if self.lower.class_has_field_initializers(class)? {
                    let bindings = self
                        .lower
                        .instance_bindings(generic_arguments, self.instance)?;
                    self.lower.declare_default_constructor(
                        self.builder.tree_mut(),
                        class,
                        &bindings,
                    )?;
                    let instance: Vec<_> =
                        bindings.iter().map(|binding| binding.argument).collect();
                    let key = self.generic_instance_key(class, None, &instance)?;
                    let function = self.function(&key)?;
                    let receiver = self.constructed_receiver(storage, pointee)?;
                    self.builder.call_function(function, vec![receiver]);
                }
            }
            // reject every remaining constructor kind
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
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
    ) -> CompilerResult<mir::Value> {
        // read the receiver type the constructor declares
        let representation = self.value_representation(storage)?;
        let receiver_storage = nominal_receiver_storage(self.builder.tree(), representation);
        let receiver =
            constructor_receiver_type(self.builder.tree_mut(), pointee, receiver_storage);

        Ok(self
            .builder
            .cast(mir::CastOperator::Bitcast, storage, receiver))
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
                anchor: self.lower.module.into(),
                construct: "a defaulted newtype argument".to_string(),
            }
            .into());
        };

        // detect singleton newtypes storing no runtime value
        let is_payload_void = match self.builder.tree().get(ty) {
            mir::Type::Newtype { inner, .. } => {
                matches!(self.builder.tree().get(*inner), mir::Type::Void)
            }
            _ => false,
        };
        if is_payload_void {
            self.lower_argument(source)?;

            return Ok(self.builder.aggregate(ty, Vec::new()));
        }

        // store the bound value as the newtype's single element
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
        let ty = self.lower.peel_owned(ty)?;
        let dir::Type::Application(_) = self.lower.ty(ty)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a structural object construction".to_string(),
            }
            .into());
        };
        let application = ty;
        let nominal = self.lower_nominal(ty)?;
        let ty = self.lower_type(ty)?;
        let nominal = self.lower.nominal(&nominal.key)?;
        let fields = nominal.fields.clone();

        // gather each property value under its field name
        let mut values = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { name, value, .. } = self.source().tree().get(*property)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a method or spread property".to_string(),
                }
                .into());
            };
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

                // reject an omitted field without an initializer
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
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
                dir::Expression::Literal(_)
            );
            if is_void && is_literal {
                continue;
            }

            // store the written value at the field's own case
            let value = self.lower_expression(*value)?;
            let representation = self.property_representation(storage, index)?;
            ordered.push(self.adapt_to_representation(value, representation)?);
        }

        Ok(self.builder.aggregate(ty, ordered))
    }

    /// Declare the synthesized default constructor one construction reaches.
    fn ensure_default_constructor(
        &mut self,
        class: dir::GlobalSymbolId,
        generic_arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<()> {
        let bindings = self
            .lower
            .instance_bindings(generic_arguments, self.instance)?;

        self.lower
            .declare_default_constructor(self.builder.tree_mut(), class, &bindings)
    }

    /// Store the declared field initializers through one constructor receiver.
    pub(in crate::lower) fn lower_field_initializers(
        &mut self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // require the class definition behind the owner
        let Some(definition) = self.lower.definition(owner)? else {
            return Err(CompilerError::Internal {
                message: "a constructor body without its class definition".to_string(),
            });
        };

        // offset own initializers past the base's chained fields
        let total = self.lower.nominal_fields(owner)?.len();
        let own = self.lower.instance_fields(definition.members());
        let inherited = total
            .checked_sub(own.len())
            .ok_or_else(|| CompilerError::Internal {
                message: "a class chaining fewer fields than it declares".to_string(),
            })?;

        // skip a class whose own fields declare no initializer
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
        let reference = self.value_representation(this)?;
        let mir::Type::Reference { pointee, .. } =
            self.builder.tree().get(mir::TypeId::from(reference))
        else {
            return Err(CompilerError::Internal {
                message: "a constructor receiver outside a reference".to_string(),
            });
        };
        let (concrete, _) = self.builder.tree().split_lifetime_application(*pointee);

        // store each declared initializer at the field it belongs to
        for (index, field) in own.iter().enumerate() {
            let index = inherited + index;
            let Some(initializer) = field.initializer else {
                continue;
            };

            // require the initializer to come from the class module
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
            let representation = self.property_representation(concrete, index)?;
            let is_void = matches!(self.builder.tree().get(representation), mir::Type::Void);
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

            // drop the evaluated value at a void representation
            if is_void {
                continue;
            }

            // store the initialized value at its declared representation
            let value = self.adapt_to_representation(value, representation)?;
            let address =
                self.emit_field_address(this, index as u32, representation, mir::Access::Exclusive);
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
        // read the initializer out of its declaring module
        let Some(declaring) = self.lower.modules.get(&module) else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside its loaded module".to_string(),
            });
        };

        Ok(matches!(
            declaring.tree().get(expression),
            dir::Expression::Literal(_)
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
        // require the field's declared initializer
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
        let representation = self.property_representation(storage, index)?;
        let is_void = matches!(self.builder.tree().get(representation), mir::Type::Void);
        if is_void && self.is_literal_initializer(initializer.module_id, expression)? {
            return Ok(None);
        }

        // evaluate the initializer inside the constructed instance
        let dir::Type::Application(applied) = self.lower.ty(application)? else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside an application type".to_string(),
            });
        };
        let specialization = self
            .lower
            .application_specialization(application, &applied)?;
        let value =
            self.lower_foreign_expression(initializer.module_id, specialization, expression)?;

        Ok(Some(self.adapt_to_representation(value, representation)?))
    }

    /// Lower one tuple expression to an aggregate value.
    pub(in crate::lower) fn lower_tuple_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<mir::Value> {
        // lower the tuple representation from the node type
        let ty = self.representation_type_id(expression)?;
        let ty = self.lower_type(ty)?;

        // lower the element values in order
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            let dir::Argument::Positional { value } = self.source().tree().get(*element) else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a spread tuple element".to_string(),
                }
                .into());
            };
            values.push(self.lower_expression(*value)?);
        }

        Ok(self.builder.aggregate(ty, values))
    }

    /// Lower one array literal at its representation.
    pub(in crate::lower) fn lower_array_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<mir::Value> {
        // lower by the representation the literal takes
        let ty = self.representation_type_id(expression)?;
        match self.lower.ty(ty)? {
            // build fixed storage as a tuple aggregate
            dir::Type::FixedArray(_) | dir::Type::Tuple(_) => {
                self.lower_tuple_expression(expression, elements)
            }
            // pack slice elements into a borrowed view
            dir::Type::Slice(slice) => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let dir::Argument::Positional { value } = self.source().tree().get(*element)
                    else {
                        return Err(LowerError::Unsupported {
                            anchor: self.lower.module.into(),
                            construct: "a spread slice element".to_string(),
                        }
                        .into());
                    };
                    values.push(value.into_global_any(self.lower.module));
                }

                self.lower_rest_pack(&values, slice.element, None)
            }
            // build every other array through its pack constructor
            _ => self
                .lower_call(expression)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "an array construction without a value".to_string(),
                }),
        }
    }

    /// Lower one object literal to its concrete class instance.
    pub(in crate::lower) fn lower_object_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<mir::Value> {
        // read the class from the literal's committed type
        let committed = self.representation_type_id(expression)?;
        let declared = self.construction_fields(committed)?;
        let Some(declared) = declared else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
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
                    anchor: self.lower.module.into(),
                    construct: "a method or spread object property".to_string(),
                }
                .into());
            };
            written.push((dir::StaticKey::from(*name), *value));
        }

        // lower the declared representation and the struct storage construction fills
        let representation = self.lower_type(committed)?;
        let (base, _) = self
            .builder
            .tree()
            .split_lifetime_application(representation);
        let (concrete, reference) = match self.builder.tree().get(base) {
            mir::Type::Reference { pointee, .. } => (*pointee, Some(representation)),
            mir::Type::Struct { .. } => (base, None),
            _ => {
                return Err(CompilerError::Internal {
                    message: "an object class outside its struct or reference representation"
                        .to_string(),
                });
            }
        };

        // build the concrete instance in declaration order
        let mut values = Vec::with_capacity(declared.len());
        for (index, field) in declared.iter().enumerate() {
            match written.iter().find(|(key, _)| *key == field.key) {
                Some((_, value)) => {
                    // skip singleton literal properties storing no runtime value
                    let representation = self.property_representation(concrete, index)?;
                    let is_void =
                        matches!(self.builder.tree().get(representation), mir::Type::Void);
                    let is_literal = matches!(
                        self.source().tree().get(*value),
                        dir::Expression::Literal(_)
                    );
                    if is_void && is_literal {
                        values.push(
                            self.builder
                                .constant(mir::Constant::Undefined, representation),
                        );

                        continue;
                    }

                    // store the written value at the property's declared representation
                    let value = self.lower_expression(*value)?;
                    values.push(self.adapt_to_representation(value, representation)?);
                }
                // store the undefined case for absent optional properties
                None if field.is_optional => {
                    values.push(self.lower_absent_property(concrete, index)?);
                }
                // reject a literal omitting a required property
                None => {
                    return Err(CompilerError::Internal {
                        message: "an absent required property".to_string(),
                    });
                }
            }
        }

        // hand the instance out at its declared representation
        let aggregate = self.builder.aggregate(concrete, values);

        Ok(match reference {
            // allocate managed destinations on the heap
            Some(reference) => self.builder.new_complete(aggregate, reference),
            // hold owned destinations in place
            None => aggregate,
        })
    }

    /// Return the construction fields one committed object type declares.
    fn construction_fields(
        &mut self,
        committed: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<ConstructionField>>> {
        // walk to the nominal or shape the type constructs
        let mut class = committed;
        loop {
            match self.lower.ty(class)? {
                // step through the memory forms around the value
                dir::Type::Form(form) => class = form.value,
                // read a nominal's fields, following alias applications on the way
                dir::Type::Application(instance) => {
                    let aliased = match self.lower.definition(instance.symbol)? {
                        Some(dir::Definition::TypeAlias(alias)) => alias.value,
                        Some(dir::Definition::Class(_) | dir::Definition::Struct(_)) => {
                            let fields = self.lower.nominal_fields(instance.symbol)?;

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
                        .lower
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

    /// Adapt one lowered value into its declared representation.
    pub(in crate::lower) fn adapt_to_representation(
        &mut self,
        value: mir::Value,
        representation: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // store a value already at its representation type directly
        let value_type = self.value_representation(value)?;
        if value_type == representation {
            return Ok(value);
        }

        // look through lifetime applications
        let (value_base, _) = self.builder.tree().split_lifetime_application(value_type);
        let (representation_base, _) = self
            .builder
            .tree()
            .split_lifetime_application(representation);
        if value_base == representation_base {
            return Ok(value);
        }

        // pass distinct identities sharing one structure unchanged
        if self.builder.tree().types_equal(
            mir::TypeId::from(value_base),
            mir::TypeId::from(representation_base),
        ) {
            return Ok(value);
        }

        // store no value in erased zero-sized representations such as variant tags
        if matches!(
            self.builder.tree().get(representation_base),
            mir::Type::Void
        ) {
            return Ok(self
                .builder
                .constant(mir::Constant::Undefined, representation_base));
        }

        // reinterpret reference kind changes over the shared fat representation
        if self
            .builder
            .tree()
            .get(representation_base)
            .is_reference_representation()
        {
            // pass values whose heads match under erased lifetimes
            let source = self.builder.tree().get(value_base).erased_lifetime();
            let target = self
                .builder
                .tree()
                .get(representation_base)
                .erased_lifetime();
            if source == target {
                return Ok(value);
            }

            return Ok(self
                .builder
                .cast(mir::CastOperator::Bitcast, value, representation));
        }

        // pass values whose contents match the representation
        if self.builder.tree().types_equal(
            mir::TypeId::from(value_base),
            mir::TypeId::from(representation_base),
        ) {
            return Ok(self
                .builder
                .cast(mir::CastOperator::Bitcast, value, representation));
        }

        // select the representation case the value type declares
        let mir::Type::Variant { cases, .. } = self.builder.tree().get(representation_base) else {
            return Err(CompilerError::Internal {
                message: "an adapted value outside its declared representation".to_string(),
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
                    "an adapted value {value_type:?} outside every case of {representation:?}"
                ),
            });
        };

        Ok(self
            .builder
            .variant_new(representation_base, case as u32, Some(value)))
    }

    /// Materialize the undefined case of one absent optional property.
    fn lower_absent_property(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let representation = self.property_representation(concrete, index)?;

        self.absent_representation_value(representation)
    }

    /// Materialize the undefined case of one optional representation.
    pub(in crate::lower) fn absent_representation_value(
        &mut self,
        representation: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        match self.builder.tree().undefined_case(representation) {
            // select the void case of a variant representation
            Some(mir::NullishCase::Case(case)) => {
                Ok(self.builder.variant_new(representation, case, None))
            }
            // store undefined directly in niched nullable representations
            Some(mir::NullishCase::Niche) => Ok(self
                .builder
                .constant(mir::Constant::Undefined, representation)),
            // reject an optional representation lowered without an undefined case
            None => Err(CompilerError::Internal {
                message: "an optional value lowered without its undefined case".to_string(),
            }),
        }
    }

    /// Return the value one omitted argument passes at its representation.
    pub(in crate::lower) fn absent_argument_value(
        &mut self,
        representation: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        match self.builder.tree().undefined_case(representation) {
            // select the void case of a variant representation
            Some(mir::NullishCase::Case(case)) => {
                self.builder.variant_new(representation, case, None)
            }
            // niched and headerless representations take the bare placeholder
            Some(mir::NullishCase::Niche) | None => self
                .builder
                .constant(mir::Constant::Undefined, representation),
        }
    }

    /// Return the declared representation type of one concrete class property.
    fn property_representation(
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

        // read the field representation out of the struct layout
        let mir::Type::Struct { fields, .. } = tree.get(concrete) else {
            return Err(CompilerError::Internal {
                message: "a property representation outside a struct layout".to_string(),
            });
        };

        Ok(tree.get(fields[index]).ty)
    }
}
