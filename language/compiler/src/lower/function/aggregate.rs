use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::call::Callee;
use crate::lower::function::argument::Argument;
use crate::lower::{FunctionLowerer, GenericScope, NominalField};
use crate::{CompilerError, CompilerResult};

/// One declared field an object literal constructs.
struct ConstructionField {
    /// The field key.
    key: dir::StaticKey,
    /// Whether the field may be absent from the literal.
    is_optional: bool,
}

/// The arguments one class construction passes its constructor.
pub(in crate::lower) enum ConstructArguments<'a> {
    /// The written arguments.
    Written {
        /// The argument bindings.
        arguments: &'a [dir::ArgumentBinding],
        /// The supplied values.
        supplied: &'a [Argument],
    },
    /// The lowered values.
    Values(Vec<mir::Value>),
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

        // evaluate a constructor operand before its arguments
        if let dir::Expression::New { left, .. } = *self.source().tree().get(expression) {
            self.lower_operand(left)?;
        }

        self.lower_construction(resolution, &[])
    }

    /// Execute one construction over its authored and supplied arguments.
    pub(in crate::lower) fn lower_construction(
        &mut self,
        resolution: &dir::ConstructDecision,
        supplied: &[Argument],
    ) -> CompilerResult<mir::Value> {
        // allocate and initialize the selected construction
        let value = match &resolution.target {
            // wrap a raw value in its newtype
            dir::ConstructTarget::Newtype { .. } => self.lower_newtype_construct(resolution),
            // construct a declared class
            dir::ConstructTarget::Class { .. } => self.lower_class_construct(resolution, supplied),
        }?;

        // execute the result conversion recorded by checking
        self.convert_value(value, resolution.coercion.as_deref())
    }

    /// Lower one super call initializing the current receiver.
    fn lower_super_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        // read the receiver the base constructor initializes
        let Some(this) = self.this else {
            return Err(CompilerError::Internal {
                message: "a super call without a receiver".to_string(),
            });
        };
        let place = self.binding_home(this)?;
        let this = self.place_reborrow(&place)?;

        // call the base constructor
        let (target, regions) = (&resolution.target, resolution.regions.as_slice());
        if let Some(mut function) = self.class_constructor(target, regions)? {
            self.bind_class_constructor(target, regions, &mut function, this)?;
            let parameters = function.parameters[1..].to_vec();
            let mut values = vec![self.constructed_receiver(this, &function)?];
            values.extend(self.lower_call_arguments(&resolution.arguments, &parameters, &[])?);
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

    /// Lower one implicit constructor body.
    pub(in crate::lower) fn lower_implicit_constructor(
        &mut self,
        class: dir::GlobalSymbolId,
        forwards: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        // read the receiver and base construction
        let Some(this) = self.this else {
            return Err(self.internal("an implicit constructor without a receiver"));
        };
        let candidate = self.lower.implicit_constructor(class, forwards)?;

        // run the base constructor
        if let Some(base) = &candidate.base {
            let place = self.binding_home(this)?;
            let this = self.place_reborrow(&place)?;
            if let Some(mut function) = self.class_constructor(base, &[])? {
                self.bind_class_constructor(base, &[], &mut function, this)?;
                let count = self.function_parameters(self.builder.function_id()).len();
                let mut values = vec![self.constructed_receiver(this, &function)?];
                values.extend((1..count).map(|index| self.builder.function_parameter(index)));
                self.call(&function, values);
            }
        }

        self.lower_field_initializers(class)
    }

    /// Resolve the constructor one class construction runs.
    fn class_constructor(
        &mut self,
        target: &dir::ConstructTarget,
        regions: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<Callee>> {
        let dir::ConstructTarget::Class {
            key,
            constructor,
            arguments,
        } = target
        else {
            return Err(self.internal("a class construction without its selected class"));
        };

        // resolve the written or implicit constructor
        match *constructor {
            dir::ClassConstructor::Declared { symbol } => {
                let selection = self.written_selection(symbol, arguments, regions)?;

                Ok(Some(self.resolve_callee_of(symbol, &selection)?))
            }
            dir::ClassConstructor::Implicit { forwards } => {
                if !self.lower.implicit_constructor_runs(key.symbol, forwards)? {
                    return Ok(None);
                }
                let bindings = self.lower.instance_bindings(arguments)?;
                let instance = self.lower.declare_implicit_constructor(
                    self.builder.tree_mut(),
                    key.symbol,
                    forwards,
                    &bindings,
                    &self.scope,
                )?;

                Ok(Some(self.callee_of(instance)?))
            }
        }
    }

    /// Bind the regions of one class constructor.
    fn bind_class_constructor(
        &mut self,
        target: &dir::ConstructTarget,
        regions: &[dir::GenericArgumentBinding],
        function: &mut Callee,
        storage: mir::Value,
    ) -> CompilerResult<()> {
        let dir::ConstructTarget::Class {
            key,
            constructor,
            arguments,
        } = target
        else {
            return Err(self.internal("a class construction without its selected class"));
        };

        // read the scope and bindings
        let (scope, positions, bindings) = match *constructor {
            dir::ClassConstructor::Declared { symbol } => {
                let selection = self.written_selection(symbol, arguments, regions)?;
                let positions = self.erased_region_positions(&selection)?;

                (
                    self.lower.symbol_scope(symbol)?,
                    positions,
                    regions.to_vec(),
                )
            }
            dir::ClassConstructor::Implicit { forwards } => {
                let scope = self
                    .lower
                    .implicit_constructor_scope(key.symbol, forwards)?;
                let bindings = arguments.iter().chain(regions).copied().collect();

                (scope, Vec::new(), bindings)
            }
        };

        let region = self.reborrow_lifetime(storage);

        self.instantiate_signature(
            &mut function.parameters,
            &mut function.result,
            &scope,
            &bindings,
            &positions,
            Some(region),
        )
    }

    /// Return the instance one written constructor runs at.
    fn written_selection(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GenericArgumentBinding],
        regions: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<dir::InstanceKey> {
        let bindings = self.lower.instance_bindings(arguments)?;
        let selection = dir::InstanceKey::new(symbol, bindings);

        self.with_type_lifetimes(symbol, &selection, regions)
    }

    /// Lower one class construction to an allocation and its constructor call.
    fn lower_class_construct(
        &mut self,
        resolution: &dir::ConstructDecision,
        supplied: &[Argument],
    ) -> CompilerResult<mir::Value> {
        // allocate and construct the instance
        self.lower_class_instance(
            resolution.return_type,
            &resolution.target,
            &resolution.regions,
            ConstructArguments::Written {
                arguments: &resolution.arguments,
                supplied,
            },
        )
    }

    /// Lower one class construction.
    pub(in crate::lower) fn lower_class_instance(
        &mut self,
        return_type: dir::GlobalTypeId,
        target: &dir::ConstructTarget,
        regions: &[dir::GenericArgumentBinding],
        arguments: ConstructArguments<'_>,
    ) -> CompilerResult<mir::Value> {
        // evaluate the arguments
        let constructor = self.class_constructor(target, regions)?;
        let parameters = match &constructor {
            Some(function) => self.signature_parameters(function.signature)?[1..].to_vec(),
            None => Vec::new(),
        };
        let values = match arguments {
            ConstructArguments::Written {
                arguments,
                supplied,
            } => self.lower_call_arguments(arguments, &parameters, supplied)?,
            ConstructArguments::Values(values) => values,
        };

        // lower the return form and its class representation
        let representation = self.lower_type(return_type)?;
        let nominal = self.lower_nominal(return_type)?;
        let pointee = nominal.storage;

        // decide where the instance stores from the return form
        let is_reference = matches!(
            self.builder.tree().type_definition(representation),
            mir::Type::Reference { .. }
        );

        // allocate zeroed heap storage for managed destinations
        let (storage, local) = if is_reference {
            let space = self.allocation_space(representation)?;

            (
                self.builder.new_zeroed(pointee, representation, space),
                None,
            )
        }
        // otherwise construct owned destinations in place inside a local slot
        else {
            let slot = self.builder.local(pointee, mir::Mutability::Mutable);
            let address = self.insert_reference(
                mir::Reference::Borrowed,
                mir::Lifetime::frame(),
                mir::Access::Exclusive,
                pointee,
            );

            let address = self.builder.address(mir::Place::local(slot), address);

            (address, Some(slot))
        };

        // run the constructor over the storage
        if let Some(mut function) = constructor {
            self.bind_class_constructor(target, regions, &mut function, storage)?;
            let receiver = self.constructed_receiver(storage, &function)?;
            let mut arguments = Vec::with_capacity(values.len() + 1);
            arguments.push(receiver);
            arguments.extend(values);
            self.call(&function, arguments);
        }

        // move completed local storage directly, retaining allocated reference results
        let object = match local {
            Some(local) => self.builder.local_get(local),
            None => storage,
        };

        Ok(object)
    }

    /// Borrow the constructed storage as the uninitialized receiver the constructor declares.
    fn constructed_receiver(
        &mut self,
        storage: mir::Value,
        constructor: &Callee,
    ) -> CompilerResult<mir::Value> {
        let Some(receiver) = constructor.parameters.first().copied() else {
            return Err(self.internal("a constructor without its receiver parameter"));
        };

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

    /// Read the object a constructor receiver fills as its this type.
    pub(in crate::lower) fn constructed_this_at(
        &mut self,
        ty: dir::GlobalTypeId,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // keep a value that borrows no uninitialized storage
        let received = self.value_representation(value)?;
        let tree = self.builder.tree();
        let mir::Type::Reference { pointee, .. } = tree.get(received) else {
            return Ok(value);
        };
        if !matches!(tree.get(*pointee), mir::Type::Uninit { .. }) {
            return Ok(value);
        }

        // read the filled object at this type
        let target = self.lower_type(ty)?;

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

        // reinterpret the handle an object newtype wraps
        if matches!(
            self.builder.tree().type_definition(ty),
            mir::Type::Reference { .. }
        ) {
            let value = self.lower_argument(source)?;

            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, ty));
        }

        // detect singleton newtypes storing no runtime value
        let is_payload_void = match self.builder.tree().type_definition(ty) {
            mir::Type::Newtype { value, .. } => {
                matches!(self.builder.tree().get(*value), mir::Type::Void)
            }
            _ => false,
        };
        if is_payload_void {
            self.lower_argument(source)?;

            return Ok(self.builder.aggregate(ty, Vec::new()));
        }

        // enter the bound value into the backing arm the construction selected, then wrap it
        let value = self.lower_argument_at(source, binding.argument_type)?;
        let value = match (&resolution.target, self.builder.tree().type_definition(ty)) {
            (
                dir::ConstructTarget::Newtype { arm: Some(arm), .. },
                mir::Type::Newtype { value: variant, .. },
            ) => {
                let variant = *variant;
                let payload = self
                    .case_has_payload(binding.argument_type)?
                    .then_some(value);

                self.builder.variant_new(variant, *arm, payload)
            }
            (dir::ConstructTarget::Newtype { arm: Some(_), .. }, _) => {
                return Err(self.internal("a newtype arm outside a newtype representation"));
            }
            _ => value,
        };

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
        let storage = instance.storage;
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
        let storage = mir::Substitution::resolve(storage, self.builder.tree_mut());
        let storage_fields = match self.builder.tree().get(storage) {
            mir::Type::Struct { fields, .. } | mir::Type::Class { fields, .. } => fields.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a nominal lowered without struct or class storage".to_string(),
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
            let is_void = field_storage.is_some_and(|ty| {
                matches!(self.builder.tree().type_definition(ty), mir::Type::Void)
            });
            let is_literal = matches!(
                self.source().tree().get(*value),
                dir::Expression::Literal(_)
            );
            if is_void && is_literal {
                continue;
            }

            // store the value at the field's storage
            let value = self.lower_value(*value)?;
            ordered.push((index, value));
        }

        // drop the field indices, keeping the values in storage order
        let values = ordered.into_iter().map(|(_, value)| value).collect();

        Ok(self.builder.aggregate(ty, values))
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

        // offset declared initializers past the base's chained fields
        let total = self.lower.nominal_fields(owner)?.len();
        let declared = self.lower.instance_fields(definition.members())?;
        let inherited =
            total
                .checked_sub(declared.len())
                .ok_or_else(|| CompilerError::Internal {
                    message: "a class chaining fewer fields than it declares".to_string(),
                })?;

        // continue for a class with unassigned fields declaring an initializer or an absence
        let is_defaulted = |field: &NominalField| {
            !self.assigned_fields.contains(&field.symbol)
                && (field.initializer.is_some() || field.is_optional)
        };
        if !declared.iter().any(is_defaulted) {
            return Ok(());
        }

        // address the constructed storage behind the receiver
        let Some(this) = self.this else {
            return Err(CompilerError::Internal {
                message: "a constructor body without a receiver".to_string(),
            });
        };
        let place = self.binding_home(this)?;
        let reference = self.place_type(&place)?;
        let reference = self.resolved_type(reference);
        let mir::Type::Reference { pointee, .. } = self.builder.tree().get(reference) else {
            return Err(CompilerError::Internal {
                message: "a constructor receiver outside a reference".to_string(),
            });
        };
        let concrete = mir::Substitution::resolve(*pointee, self.builder.tree_mut());

        // store each declared default at the field it belongs to, skipping assigned fields
        for (index, field) in declared.iter().enumerate() {
            let index = inherited + index;
            if self.assigned_fields.contains(&field.symbol) {
                continue;
            }

            // store the absence an optional field holds until a constructor assigns it
            let Some(initializer) = field.initializer else {
                if field.is_optional {
                    let this = self.place_reborrow(&place)?;
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
            let is_void = matches!(
                self.builder.tree().type_definition(representation),
                mir::Type::Void
            );
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

            // materialize a singleton initializer
            let value = match self.node_type(expression)? {
                dir::Type::Literal(literal) if self.coercion(expression).is_none() => {
                    self.lower_constant(literal, representation)?
                }
                _ => value,
            };

            // store the settled value at its declared representation through the receiver
            let this = self.place_reborrow(&place)?;
            self.lower_anchored(expression, |function| {
                let place = mir::Place::value(this)
                    .with_projection(mir::Projection::Deref)
                    .with_projection(mir::Projection::Field {
                        index: index as u32,
                    });
                function.builder.store(place, value);

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Store undefined at one optional field a constructor may leave unassigned.
    fn store_field_absence(
        &mut self,
        this: mir::Value,
        owner: mir::TypeId,
        index: u32,
    ) -> CompilerResult<()> {
        // skip a void representation
        let representation = self.property_representation(owner, index as usize)?;
        if matches!(
            self.builder.tree().type_definition(representation),
            mir::Type::Void
        ) {
            return Ok(());
        }

        // store the undefined sentinel at the field's representation
        let value = self.lower_constant(dir::Literal::Undefined, representation)?;
        let place = mir::Place::value(this)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Field { index });
        self.builder.store(place, value);

        Ok(())
    }

    /// Return whether one initializer is a scalar literal computing nothing.
    fn is_literal_initializer(
        &mut self,
        module: tspp_source::ModuleId,
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
        storage: mir::TypeId,
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
        let is_void = matches!(
            self.builder.tree().type_definition(representation),
            mir::Type::Void
        );
        if is_void && self.is_literal_initializer(initializer.module_id, expression)? {
            return Ok(None);
        }

        // evaluate the initializer at the application's arguments
        let grounded = self.grounded_scope(application)?;
        let scope = std::mem::replace(&mut self.scope, grounded);
        let value = self.lower_foreign_expression(initializer.module_id, expression);
        self.scope = scope;

        Ok(Some(value?))
    }

    /// Return one application's declaration scope after this scope, grounded at its arguments.
    fn grounded_scope(&mut self, application: dir::GlobalTypeId) -> CompilerResult<GenericScope> {
        let dir::Type::Application(instance) = self.lower.ty(application)? else {
            return Err(CompilerError::Internal {
                message: "a field initializer read outside an application type".to_string(),
            });
        };
        let module = instance.symbol.module_id;
        let Some(template) = self
            .lower
            .definition(instance.symbol)?
            .and_then(|definition| definition.template())
        else {
            return Ok(self.scope.clone());
        };
        let template = template.into_global(module);
        let parameters = self
            .lower
            .state(module)?
            .generics
            .get_template(template.local_id)
            .parameters
            .clone();
        let arguments = self
            .lower
            .types(application.module_id)?
            .type_ids(instance.arguments)
            .to_vec();

        // index the declaration's parameters after the enclosing ones, every index at itself
        let tree = self.builder.tree_mut();
        let mut scope = GenericScope::from_templates(self.lower, Some(template), None)?
            .with_parameters_of(&self.scope, self.lower, tree)?;
        let grounded = scope.grounding.len() as u32;
        if grounded < scope.count() {
            let identity = scope.identity_arguments(self.lower, tree, grounded)?;
            scope.grounding.extend(identity);
        }

        // ground the declaration's parameters at the application's arguments
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            let parameter = parameter.into_global(module);
            let binding = dir::GenericArgumentBinding {
                parameter,
                argument,
            };
            let argument = self
                .lower
                .type_lowerer(tree, &self.scope)
                .lower_bound_argument(binding)?;
            let index =
                scope
                    .parameter_index(parameter)
                    .ok_or_else(|| CompilerError::Internal {
                        message: "a declaration parameter outside its grounded scope".to_string(),
                    })?;
            scope.grounding[index as usize] = argument;
        }

        Ok(scope)
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
                    values.push(dir::ArgumentBinding {
                        parameter_type: slice.element,
                        argument_type: slice.element,
                        source: dir::ArgumentSource::Provided(
                            value.into_global_any(self.lower.module),
                        ),
                        coercion: None,
                    });
                }

                self.lower_rest_pack(&values, slice.element, None, ty, &[])
            }
            // construct the array in its selected storage form
            _ => {
                let value =
                    self.lower_call(expression)?
                        .ok_or_else(|| CompilerError::Internal {
                            message: "an array construction without a value".to_string(),
                        })?;
                let target = self.lower_type(ty)?;

                self.adopt(value, target)
            }
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

        // evaluate the properties in source order
        let mut written = Vec::with_capacity(properties.len());
        for property in properties {
            match self.source().tree().get(*property).clone() {
                dir::Property::Field { name, value, .. } => {
                    let value = self.lower_value(value)?;
                    written.push((dir::StaticKey::from(name), value));
                }
                dir::Property::Spread { value } => {
                    let source = self.lower_value(value)?;
                    for (key, field) in self.spread_fields(*property)? {
                        let value = self.read_field(source, &field)?;
                        written.push((key, value));
                    }
                }
                _ => return Err(self.unsupported("a method object property")),
            }
        }

        // lower the declared representation and the struct storage construction fills
        let representation = self.lower_type(committed)?;
        let base = mir::Substitution::resolve(representation, self.builder.tree_mut());
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

        // build the instance in declaration order
        let mut values = Vec::with_capacity(declared.len());
        for (index, field) in declared.iter().enumerate() {
            match written.iter().rev().find(|(key, _)| *key == field.key) {
                // store a supplied value
                Some((_, value)) => values.push(*value),
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
            Some(reference) => self.box_value(aggregate, reference)?,
            // hold owned destinations in place
            None => aggregate,
        })
    }

    /// Return the fields one spread property supplies.
    fn spread_fields(
        &mut self,
        property: dir::LocalNodeId<dir::Property>,
    ) -> CompilerResult<Vec<(dir::StaticKey, dir::FieldResolution)>> {
        // read the spread subject
        let site = dir::MemberSite::Node(property.into_global_any(self.source));
        let Some((subject, _)) = self.source().members.subject(site) else {
            return Err(self.internal("a spread property without its member subject"));
        };
        let dir::Type::Object(shape) = self.lower.ty(subject.key_source)? else {
            return Err(self.internal("a spread subject keyed outside an object type"));
        };
        let properties = self
            .lower
            .types(subject.key_source.module_id)?
            .properties(shape.properties)
            .to_vec();

        // read each key structurally
        let mut fields = Vec::with_capacity(properties.len());
        for property in properties {
            let Some(ty) = property.access.read() else {
                continue;
            };
            let field = dir::FieldResolution {
                receiver: dir::MemberReceiver::direct(subject.receiver),
                target: dir::FieldTarget::Structural {
                    owner: subject.receiver,
                    key: property.key,
                },
                ty,
            };
            fields.push((property.key, field));
        }

        Ok(fields)
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
        concrete: mir::TypeId,
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
        concrete: mir::TypeId,
        index: usize,
    ) -> CompilerResult<mir::TypeId> {
        let tree = self.builder.tree_mut();

        // unwrap uninitialized storage to the layout its value holds
        let mut concrete = concrete;
        while let mir::Type::Uninit { value } = tree.get(concrete) {
            concrete = *value;
        }

        // read the field representation off the struct layout, an application through its own
        let concrete = mir::Substitution::resolve(concrete, tree);
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
        let storage = representation.storage(self.builder.tree_mut());

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
        let mir::Type::FixedArray { length, .. } =
            *self.builder.tree().type_definition(representation)
        else {
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
