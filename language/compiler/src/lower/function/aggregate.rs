use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    FunctionLowerer, NominalField, constructor_receiver_type, nominal_receiver_storage,
};
use crate::{CompilerError, CompilerResult};

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
            dir::ConstructTarget::Dynamic { .. } => {
                Err(self.unsupported("a dynamically dispatched construction"))
            }
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
                return Err(self.internal(format!("a {} super constructor", other.name())));
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
                Some(self.resolve_callee_of(symbol, selection)?)
            }
            None => None,
        };

        // call the base constructor over the narrowed receiver, a synthesized one at the base's
        // instance bindings
        if let (Some(mut function), Some(symbol)) = (function, target) {
            let lifetime = self.reborrow_lifetime(this);
            let (scope, regions) = match symbol == selection.symbol {
                true => (
                    self.lower.class_constructor_scope(symbol)?,
                    selection.arguments.as_slice(),
                ),
                false => (
                    self.lower.symbol_scope(symbol)?,
                    resolution.regions.as_slice(),
                ),
            };
            self.instantiate_signature(
                &mut function.parameters,
                &mut function.result,
                &scope,
                regions,
                Some(lifetime),
            )?;
            let parameters = function.parameters.clone();
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
            self.call(&function, values);
        }

        // store the derived field initializers once the base storage settles
        if let Some(owner) = self.constructs {
            self.lower_field_initializers(owner)?;
        }

        // yield the void value a super call produces
        let void = self.builder.tree_mut().intern_type(mir::Type::Void);

        Ok(self.builder.constant(mir::Constant::Zeroed, void))
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
                let function = self.resolve_callee_of(*symbol, selection)?;
                let parameters = self.signature_parameters(function.signature)?;
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
                return Err(self.internal(format!("a {other:?} constructor")));
            }
        };

        self.lower_class_instance(
            resolution.return_type,
            constructor,
            &selection.arguments,
            &resolution.regions,
            values,
        )
    }

    /// Lower one class construction over its evaluated argument values, a declared constructor
    /// instantiated at the regions the construction bound.
    pub(in crate::lower) fn lower_class_instance(
        &mut self,
        return_type: dir::GlobalTypeId,
        constructor: &dir::ClassConstructor,
        generic_arguments: &[dir::GenericArgumentBinding],
        regions: &[dir::GenericArgumentBinding],
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
                mir::Lifetime::frame(),
                mir::Access::Mutable,
                mir::Storage::Frame,
                pointee,
            );
            let address = self
                .builder
                .local_addr(slot, address, mir::AddressKind::Borrow);

            (Some(slot), address)
        };

        // initialize the storage through an exclusive borrow
        match constructor {
            // call the constructor the class declares
            dir::ClassConstructor::Declared { symbol } => {
                // select the declared instance from the substituted class arguments
                let bindings = self.lower.instance_bindings(generic_arguments)?;
                let selection = dir::InstanceKey::new(*symbol, bindings);
                let mut function = self.resolve_callee_of(*symbol, &selection)?;
                let lifetime = self.reborrow_lifetime(storage);
                let scope = self.lower.symbol_scope(*symbol)?;
                self.instantiate_signature(
                    &mut function.parameters,
                    &mut function.result,
                    &scope,
                    regions,
                    Some(lifetime),
                )?;
                let receiver = self.constructed_receiver(storage, pointee)?;

                // bind the constructor arguments after the receiver
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(arguments);
                self.call(&function, values);
            }
            // call the synthesized constructor a defaulted class stands in for
            dir::ClassConstructor::Default => {
                // peel the return form down to the constructed class
                let stored = match self.lower.indirection(return_type, &self.scope)? {
                    Some(reference) => reference.stored,
                    None => self.lower.stored(return_type)?,
                };
                let dir::Type::Application(application) = self.lower.ty(stored)? else {
                    return Err(CompilerError::Internal {
                        message: "a class construction outside an application type".to_string(),
                    });
                };

                // synthesize the call only where the class stores initializers
                let class = application.symbol;
                if self.lower.class_has_field_initializers(class)? {
                    let bindings = self.lower.instance_bindings(generic_arguments)?;
                    let constructor = self.lower.declare_default_constructor(
                        self.builder.tree_mut(),
                        class,
                        &bindings,
                        &self.scope,
                    )?;
                    let mut selected = self.callee_of(constructor);
                    let lifetime = self.reborrow_lifetime(storage);
                    let scope = self.lower.class_constructor_scope(class)?;
                    self.instantiate_signature(
                        &mut selected.parameters,
                        &mut selected.result,
                        &scope,
                        generic_arguments,
                        Some(lifetime),
                    )?;
                    let receiver = self.constructed_receiver(storage, pointee)?;
                    self.call(&selected, vec![receiver]);
                }
            }
            // reject every remaining constructor kind
            other => {
                return Err(self.internal(format!("a {other:?} constructor")));
            }
        }

        // read the initialized slot back out for owned constructions
        let object = match slot {
            Some(slot) => self.builder.local_get(slot),
            None => storage,
        };

        Ok(object)
    }

    /// Bitcast one constructed storage to the exclusive uninitialized receiver the instantiated
    /// constructor takes.
    fn constructed_receiver(
        &mut self,
        storage: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // reborrow the storage as the uninitialized receiver the constructor fills
        let lifetime = self.reborrow_lifetime(storage);
        let representation = self.value_representation(storage)?;
        let receiver_storage = nominal_receiver_storage(self.builder.tree(), representation);
        let receiver =
            constructor_receiver_type(self.builder.tree_mut(), pointee, receiver_storage, lifetime);

        Ok(self
            .builder
            .cast(mir::CastOperator::Bitcast, storage, receiver))
    }

    /// Read the object a constructor's receiver fills once the body reads this as a value.
    pub(in crate::lower) fn constructed_this(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let ty = self.node_type_id(expression)?;

        self.constructed_this_at(ty, value)
    }

    /// Read the object a constructor's receiver fills as one this type, the managed reference
    /// of a heap class.
    pub(in crate::lower) fn constructed_this_at(
        &mut self,
        ty: dir::GlobalTypeId,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let received = self.value_representation(value)?;
        let tree = self.builder.tree();
        let mir::Type::Reference { pointee, .. } = tree.get(mir::TypeId::from(received)) else {
            return Ok(value);
        };
        if !matches!(tree.get(*pointee), mir::Type::Uninit { .. }) {
            return Ok(value);
        }
        let target = self.lower_type(ty)?;
        let is_managed = matches!(
            self.builder.tree().get(target),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                ..
            }
        );
        if !is_managed {
            return Ok(value);
        }

        Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target))
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
            return Err(self.unsupported("a defaulted newtype argument"));
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
        let ty = self.lower.stored(ty)?;
        let dir::Type::Application(_) = self.lower.ty(ty)? else {
            return Err(self.unsupported("a structural object construction"));
        };
        let application = ty;
        let instance = self.lower_nominal(ty)?;
        let storage = mir::TypeId::from(instance.storage);
        let is_class = matches!(
            self.lower.definition(instance.key.symbol)?,
            Some(dir::Definition::Class(_))
        );
        let nominal = self.lower.nominal(instance.key.symbol)?;
        let fields = nominal.fields.clone();

        // build a class literal at its object storage, a value family at its representation
        let ty = match is_class {
            true => instance.storage,
            false => self.lower_type(ty)?,
        };

        // gather each property value under its field name
        let mut values = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { name, value, .. } = self.source().tree().get(*property)
            else {
                return Err(self.unsupported("a method or spread property"));
            };
            values.push((dir::StaticKey::from(*name), *value));
        }

        // collect the field storage types behind the nominal's object
        let storage = self.builder.tree().represented(storage);
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
                        ordered.push((index, value));
                    }

                    continue;
                }

                // store the undefined case for absent optional fields
                if field.is_optional {
                    ordered.push((index, self.lower_absent_property(storage, index)?));

                    continue;
                }

                // reject an omitted field without an initializer
                return Err(self.internal("an omitted field without an initializer"));
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

            // store the written value at the field's storage
            let value = self.lower_value(*value)?;
            ordered.push((index, value));
        }

        // drop the field indices, keeping the values in storage order
        let values = ordered.into_iter().map(|(_, value)| value).collect();

        Ok(self.builder.aggregate(ty, values))
    }

    /// Declare the synthesized default constructor one construction calls.
    fn ensure_default_constructor(
        &mut self,
        class: dir::GlobalSymbolId,
        generic_arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<()> {
        let bindings = self.lower.instance_bindings(generic_arguments)?;

        self.lower.declare_default_constructor(
            self.builder.tree_mut(),
            class,
            &bindings,
            &self.scope,
        )?;

        Ok(())
    }

    /// Store the declared field initializers through one constructor receiver.
    pub(in crate::lower) fn lower_field_initializers(
        &mut self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // require the class definition behind the owner
        let Some(definition) = self.lower.definition(owner)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a constructor body without its class definition".to_string(),
            });
        };

        // offset own initializers past the base's chained fields
        let total = self.lower.nominal_fields(owner)?.len();
        let own = self.lower.instance_fields(definition.members())?;
        let inherited = total
            .checked_sub(own.len())
            .ok_or_else(|| CompilerError::Internal {
                message: "a class chaining fewer fields than it declares".to_string(),
            })?;

        // continue for a class with unassigned fields declaring an initializer or an absence
        let is_defaulted = |field: &NominalField| {
            !self.assigned_fields.contains(&field.symbol)
                && (field.initializer.is_some() || field.is_optional)
        };
        if !own.iter().any(is_defaulted) {
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
        let concrete = self.builder.tree().represented(*pointee);

        // store each declared default at the field it belongs to, skipping assigned fields
        for (index, field) in own.iter().enumerate() {
            let index = inherited + index;
            if self.assigned_fields.contains(&field.symbol) {
                continue;
            }

            // store the absence an optional field holds until a constructor assigns it
            let Some(initializer) = field.initializer else {
                if field.is_optional {
                    self.store_field_absence(this, concrete, index as u32)?;
                }

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
            let value = self.lower_value(expression);
            self.values = values;
            self.frames = frames;
            let value = value?;

            // drop the evaluated value at a void representation
            if is_void {
                continue;
            }

            // store the initialized value at its declared representation
            self.lower_anchored(expression, |function| {
                let address = function.field_address(
                    this,
                    index as u32,
                    representation,
                    mir::Access::Mutable,
                );
                function.builder.store(address, value);

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Store undefined at one optional field a constructor may leave unassigned.
    fn store_field_absence(
        &mut self,
        this: mir::Value,
        owner: mir::LocalNodeId<mir::Type>,
        index: u32,
    ) -> CompilerResult<()> {
        // skip a void representation
        let representation = self.property_representation(owner, index as usize)?;
        if matches!(self.builder.tree().get(representation), mir::Type::Void) {
            return Ok(());
        }

        // store the undefined sentinel at the field's representation
        let value = self.lower_constant(dir::Literal::Undefined, representation)?;
        let address = self.field_address(this, index, representation, mir::Access::Mutable);
        self.builder.store(address, value);

        Ok(())
    }

    /// Return whether one initializer is a scalar literal computing nothing.
    fn is_literal_initializer(
        &mut self,
        module: destack_source::ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        // read the initializer out of its declaring module
        let declaring = self.lower.state(module)?;

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
        let dir::Type::Application(_) = self.lower.ty(application)? else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside an application type".to_string(),
            });
        };
        let value = self.lower_foreign_expression(initializer.module_id, expression)?;

        Ok(Some(value))
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
                return Err(self.unsupported("a spread tuple element"));
            };
            values.push(self.lower_value(*value)?);
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
                        return Err(self.unsupported("a spread slice element"));
                    };
                    values.push(value.into_global_any(self.lower.module));
                }

                self.lower_rest_pack(&values, slice.element, None, false)
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
            return Err(self.unsupported("an object literal outside its concrete class"));
        };

        // gather each written value under its property name
        let mut written = Vec::with_capacity(properties.len());
        for property in properties {
            let dir::Property::Field { name, value, .. } = self.source().tree().get(*property)
            else {
                return Err(self.unsupported("a method or spread object property"));
            };
            written.push((dir::StaticKey::from(*name), *value));
        }

        // lower the declared representation and the struct storage construction fills
        let representation = self.lower_type(committed)?;
        let base = self.builder.tree().represented(representation);
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
                        values.push(self.builder.constant(mir::Constant::Zeroed, representation));

                        continue;
                    }

                    // store the written value at the property's storage
                    let value = self.lower_value(*value)?;
                    values.push(value);
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
                        Some(dir::Definition::TypeAlias(_)) => {
                            self.lower.symbol_type(instance.symbol)?
                        }
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

    /// Materialize the undefined case of one absent optional property.
    fn lower_absent_property(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let representation = self.property_representation(concrete, index)?;
        let Some(absent) = self.absent_value(representation) else {
            return Err(CompilerError::Internal {
                message: "an absent optional property without an undefined case".to_string(),
            });
        };

        Ok(absent)
    }

    /// Return the declared representation type of one concrete class property.
    pub(in crate::lower) fn property_representation(
        &mut self,
        concrete: mir::LocalNodeId<mir::Type>,
        index: usize,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let tree = self.builder.tree_mut();

        // unwrap uninitialized storage to the layout its value holds
        let mut concrete = concrete;
        while let mir::Type::Uninit { value } = tree.get(concrete) {
            concrete = *value;
        }

        // read the field representation off the struct layout, an application through its own
        let concrete = tree.represented(mir::TypeId::from(concrete));
        let Some(field) = tree.get(concrete).field_type(index as u32, tree) else {
            return Err(CompilerError::Internal {
                message: "a property representation outside a struct layout".to_string(),
            });
        };

        Ok(field)
    }

    /// Lower one range expression to the range family its bounds name.
    pub(in crate::lower) fn lower_range_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<mir::Value> {
        // read the range family the node's type names
        let ty = self.node_type_id(expression)?;
        let ty = self.lower.stored(ty)?;
        let representation = self.lower_type(ty)?;
        let storage = self
            .builder
            .tree()
            .storage_type(mir::TypeId::from(representation));

        // store each written bound under the member it names
        let mir::Type::Struct { fields, .. } = self.builder.tree().get(storage).clone() else {
            return Err(CompilerError::Internal {
                message: "a range family outside struct storage".to_string(),
            });
        };
        let mut values = vec![None; fields.len()];
        for (bound, member) in [(start, "start"), (end, "end")] {
            let Some(bound) = bound else {
                continue;
            };
            let index = self.language_member_field(ty, member)?;
            values[index as usize] = Some(self.lower_value(bound)?);
        }

        // require every declared bound of the selected family
        let mut ordered = Vec::with_capacity(values.len());
        for value in values {
            let Some(value) = value else {
                return Err(CompilerError::Internal {
                    message: "a range family with an unwritten bound".to_string(),
                });
            };
            ordered.push(value);
        }

        Ok(self.builder.aggregate(representation, ordered))
    }

    /// Lower one fixed array expression by repeating its value.
    pub(in crate::lower) fn lower_fixed_array_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // read the repeated length off the lowered representation
        let representation = self.lower_type(self.node_type_id(expression)?)?;
        let mir::Type::FixedArray { length, .. } = *self.builder.tree().get(representation) else {
            return Err(CompilerError::Internal {
                message: "a fixed array expression outside fixed array storage".to_string(),
            });
        };

        // repeat the value across the declared length
        let Some(length) = self.builder.tree().static_value(length).length() else {
            return Err(CompilerError::Internal {
                message: "a fixed array expression at an open length".to_string(),
            });
        };
        let value = self.lower_value(value)?;
        let values = vec![value; length as usize];

        Ok(self.builder.aggregate(representation, values))
    }
}
