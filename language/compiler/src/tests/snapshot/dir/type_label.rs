use destack_dir as dir;
use destack_source::ModuleId;

use super::DirSnapshotBuilder;

impl DirSnapshotBuilder<'_> {
    /// Return one type label from a type table.
    pub(super) fn type_table_label(
        &self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
    ) -> String {
        // resolve the canonical type slot
        let ty = types.get_type(type_id);

        self.type_label_from_value(types, &ty)
    }

    /// Return one type text label.
    fn type_label_from_value(&self, types: &dir::TypeTable<'_>, ty: &dir::Type) -> String {
        match ty {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Region(region) => format!(
                "{} & {}",
                self.type_id_label(types, region.extent),
                self.type_id_label(types, region.space)
            ),
            dir::Type::Never => "never".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Primitive(primitive) => Self::primitive_type_label(*primitive),
            dir::Type::Literal(literal) => self.scalar_literal_label(literal),
            dir::Type::Key(key) => self.key_type_label(*key),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::Erased(_) => "*".to_string(),
            dir::Type::Parameter(parameter) => self.parameter_type_label(parameter),
            dir::Type::Reference(reference) => {
                let name = self.reference_symbol_label(reference.symbol);
                let name = if reference.arguments.is_empty() {
                    name
                } else {
                    let arguments =
                        self.type_id_list_label(types, types.type_ids(reference.arguments), ", ");

                    format!("{name}<{arguments}>")
                };
                if matches!(
                    self.definition(reference.symbol),
                    Some(dir::Definition::Class(_))
                ) {
                    format!("typeof {name}")
                } else {
                    name
                }
            }
            dir::Type::Application(instance) => self.instance_type_label(types, instance),
            dir::Type::This => "this".to_string(),
            dir::Type::Member(member) => self.member_type_label(types, types.member(*member)),
            dir::Type::Refined(refined) => {
                let refined = types.refined(*refined);
                let base = self.type_id_label(types, refined.base);
                let key = self.static_key(refined.key);
                let value = self.type_id_label(types, refined.value);

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::Variant(variant) => self.variant_type_label(variant),
            dir::Type::Form(form) => self.form_type_label(types, form),
            dir::Type::Dynamic(any) => {
                let constraint = self.type_id_label(types, any.constraint);

                format!("Dynamic<{constraint}>")
            }
            dir::Type::Operation(operation) => {
                self.operation_type_label(types, types.operation(*operation))
            }
            dir::Type::FixedArray(array) => self.fixed_array_type_label(types, array),
            dir::Type::Range(range) => self.range_type_label(range),
            dir::Type::Slice(slice) => self.slice_type_label(types, slice),
            dir::Type::Tuple(tuple) => self.tuple_type_label(types, tuple),
            dir::Type::Object(shape) => self.object_type_label(types, shape),
            dir::Type::FunctionSignature(function) => {
                self.function_type_label(types, types.signature(*function))
            }
            dir::Type::Function(function) => self.function_value_type_label(types, function),
            dir::Type::FunctionPointer(function) => {
                self.function_pointer_type_label(types, function)
            }
            dir::Type::Union(union) => {
                self.type_id_list_label(types, types.type_ids(union.elements), " | ")
            }
            dir::Type::Variable(variable) => format!("?{}", variable.0),
            dir::Type::Static(static_id) => self.global_static_label(*static_id),
            dir::Type::Intersection(intersection) => {
                self.type_id_list_label(types, types.type_ids(intersection.elements), " & ")
            }
        }
    }

    /// Return one precise variant type label.
    fn variant_type_label(&self, variant: &dir::VariantType) -> String {
        let types = if variant.owner.module_id == self.tree.module_id {
            self.types
                .as_ref()
                .unwrap_or_else(|| panic!("dir snapshot is missing its type table"))
        } else {
            self.foreign_types
                .get(&variant.owner.module_id)
                .unwrap_or_else(|| {
                    panic!(
                        "variant owner module {:?} is not loaded for snapshots",
                        variant.owner.module_id
                    )
                })
        };
        let dir::Type::Application(owner) = types.get_type(variant.owner.local_id) else {
            panic!("variant type must point to an application owner");
        };
        let definition = self
            .definition(owner.symbol)
            .unwrap_or_else(|| panic!("variant owner {:?} has no definition", owner.symbol));
        let key = match definition {
            dir::Definition::Enum(definition) => definition
                .variants()
                .find(|member| member.symbol == variant.variant)
                .map(|member| member.key),
            _ => None,
        }
        .unwrap_or_else(|| {
            panic!(
                "variant {:?} is missing from owner {:?}",
                variant.variant, owner.symbol
            )
        });
        let owner_label = self.reference_symbol_label(owner.symbol);
        let key = self.static_key(key);
        let symbol = format!("{owner_label}.{key}");
        if owner.arguments.is_empty() {
            return symbol;
        }

        let arguments = self.type_id_list_label(types, types.type_ids(owner.arguments), ", ");

        format!("{symbol}<{arguments}>")
    }

    /// Return one exact property key type label.
    fn key_type_label(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => format!("\"{}\"", self.strings.get(name)),
            dir::StaticKey::Index(index) => index.to_string(),
        }
    }

    /// Return one function value type label.
    fn function_value_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionType,
    ) -> String {
        // print the signature under the elided receiver, the application under every other receiver
        let mode = self.receiver_mode_at(types, function.receiver);
        let receiver = self.type_id_label(types, function.receiver);
        let (types, signature) = self.function_signature_type(types, function.signature);
        if mode.is_some_and(dir::ReceiverMode::is_elided) {
            return self.function_type_label(types, &signature);
        }
        let parameters = self.function_parameter_tuple_label(types, &signature);
        let return_type = self.function_return_type_label(types, &signature);

        format!("Function<{parameters}, {return_type}, {receiver}>")
    }

    /// Return the settled receiver mode one function value names, none for an open receiver.
    fn receiver_mode_at(
        &self,
        types: &dir::TypeTable<'_>,
        receiver: dir::GlobalTypeId,
    ) -> Option<dir::ReceiverMode> {
        let dir::Type::Literal(dir::Literal::String(text)) = self.type_at(types, receiver)? else {
            return None;
        };

        dir::ReceiverMode::from_text(self.strings.get(text))
    }

    /// Return one function pointer type label.
    fn function_pointer_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionPointerType,
    ) -> String {
        let (types, signature) = self.function_signature_type(types, function.signature);
        let parameters = self.function_parameter_tuple_label(types, &signature);
        let return_type = self.function_return_type_label(types, &signature);

        format!("FunctionPointer<{parameters}, {return_type}>")
    }

    /// Return the function signature type referenced by one callable representation.
    fn function_signature_type<'t>(
        &'t self,
        types: &'t dir::TypeTable<'t>,
        signature_id: dir::GlobalTypeId,
    ) -> (&'t dir::TypeTable<'t>, dir::FunctionSignatureType) {
        // foreign signatures resolve through their owning module's table
        let types = if signature_id.module_id == types.module_id {
            types
        } else {
            self.foreign_types
                .get(&signature_id.module_id)
                .unwrap_or_else(|| {
                    panic!(
                        "callable signature module {:?} is not loaded for snapshots",
                        signature_id.module_id
                    )
                })
        };
        let signature = types.get_type(signature_id.local_id);
        let dir::Type::FunctionSignature(signature) = signature else {
            panic!("callable signature type must point to a function signature");
        };
        let signature = *types.signature(signature);

        (types, signature)
    }

    /// Return one function signature's parameter tuple label.
    fn function_parameter_tuple_label(
        &self,
        types: &dir::TypeTable<'_>,
        signature: &dir::FunctionSignatureType,
    ) -> String {
        let mut parameters =
            self.function_parameter_list_label(types, types.parameters(signature.parameters), ", ");
        if signature.parameters.len() == 1 {
            parameters.push(',');
        }

        format!("({parameters})")
    }

    /// Return one function signature's return type label.
    fn function_return_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        signature: &dir::FunctionSignatureType,
    ) -> String {
        signature
            .return_type
            .map(|ty| self.type_id_label(types, ty))
            .unwrap_or_else(|| "void".to_string())
    }

    /// Return one type id label through a table.
    fn type_id_label(&self, types: &dir::TypeTable<'_>, type_id: dir::GlobalTypeId) -> String {
        if type_id.module_id != types.module_id {
            return self.global_type_label(type_id);
        }

        self.type_table_label(types, type_id.local_id)
    }

    /// Return one primitive type label.
    fn primitive_type_label(primitive: dir::PrimitiveType) -> String {
        match primitive {
            dir::PrimitiveType::Boolean => "boolean".to_string(),
            dir::PrimitiveType::Character => "char".to_string(),
            dir::PrimitiveType::String => "string".to_string(),
            dir::PrimitiveType::Bigint => "bigint".to_string(),
            dir::PrimitiveType::Integer(integer) => integer.as_str(),
            dir::PrimitiveType::Float(float) => float.as_str().to_string(),
        }
    }

    /// Return one instance type label.
    fn instance_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        instance: &dir::GenericApplication,
    ) -> String {
        if instance.arguments.is_empty() {
            return self.reference_symbol_label(instance.symbol);
        }

        let arguments = types.type_ids(instance.arguments);
        if let Some(label) = self.collection_type_label(types, instance.symbol, arguments) {
            return label;
        }

        let arguments = arguments
            .iter()
            .map(|argument| self.type_id_label(types, *argument))
            .collect::<Vec<_>>()
            .join(", ");
        let symbol = self.reference_symbol_label(instance.symbol);

        format!("{symbol}<{arguments}>")
    }

    /// Return one reference symbol label.
    pub(super) fn reference_symbol_label(&self, symbol: dir::GlobalSymbolId) -> String {
        if let Some(item) = self.language_item_by_symbol.get(&symbol) {
            let key = item.to_string();
            if let Some((_, name)) = key.rsplit_once('.') {
                return name.to_string();
            }

            return key;
        }

        if let Some(names) = self.global_names_by_symbol.get(&symbol)
            && names.len() == 1
            && let Some(name) = names.first()
        {
            return self.strings.get(*name).to_string();
        }

        self.symbol_path_label(symbol)
    }

    /// Return one member type label.
    fn member_type_label(&self, types: &dir::TypeTable<'_>, member: &dir::MemberType) -> String {
        let owner = self.type_id_label(types, member.owner);
        let key = self.static_key(member.key);
        if member.arguments.is_empty() {
            return format!("{owner}.{key}");
        }

        let arguments = self.type_id_list_label(types, types.type_ids(member.arguments), ", ");

        format!("{owner}.{key}<{arguments}>")
    }

    /// Return one collection type label.
    pub(super) fn collection_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> Option<String> {
        let item = self.language_item_by_symbol.get(&symbol)?;

        match (item, arguments) {
            (dir::LanguageItem::Array, [element]) => {
                let element = self.type_id_label(types, *element);

                Some(format!("{element}[]"))
            }
            (dir::LanguageItem::ReadonlyArray, [element]) => {
                let element = self.type_id_label(types, *element);

                Some(format!("ReadonlyArray<{element}>"))
            }
            (dir::LanguageItem::FixedArray, [element, count]) => {
                let element = self.type_id_label(types, *element);
                let count = self.type_id_label(types, *count);

                Some(format!("FixedArray<{element}, {count}>"))
            }
            (dir::LanguageItem::Slice, [element]) => {
                let element = self.type_id_label(types, *element);

                Some(format!("Slice<{element}>"))
            }
            _ => None,
        }
    }

    /// Return one form type label.
    fn form_type_label(&self, types: &dir::TypeTable<'_>, form: &dir::FormType) -> String {
        // group a payload that binds looser than the form
        let value = self.type_id_label(types, form.value);
        let is_grouped = self.type_at(types, form.value).is_some_and(|payload| {
            payload
                .needs_parentheses(
                    dir::TypeOperand::Prefix,
                    |id| Ok::<_, ()>(self.operation_at(types, form.value.module_id, id)),
                    |receiver| Ok(self.receiver_mode_at(types, receiver)),
                )
                .expect("snapshot type tables are complete")
        });
        let value = match is_grouped {
            true => format!("({value})"),
            false => value,
        };

        // render canonical form constructors
        match &form.form {
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed(borrow) => {
                let borrow = types.borrow_form(*borrow);
                let (region, access) = dir::read_borrow(
                    borrow,
                    |id| self.type_at(types, id).ok_or(()),
                    |text| self.strings.get(text).to_string(),
                    |parameter| Ok(self.parameter_type_label(&parameter)),
                )
                .expect("snapshot type tables are complete");
                match dir::borrow_text(region.as_ref(), access.as_ref(), &value) {
                    Some(text) => text,
                    None => {
                        let region = self.type_id_label(types, borrow.region);
                        let access = self.type_id_label(types, borrow.access);

                        format!("Borrowed<{value}, {region}, {access}>")
                    }
                }
            }
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
        }
    }

    /// Return one type operation through the table owning it.
    fn operation_at(
        &self,
        types: &dir::TypeTable<'_>,
        module: ModuleId,
        id: dir::TypeOperationId,
    ) -> dir::TypeOperation {
        match module == types.module_id {
            true => *types.operation(id),
            false => *self.foreign_types[&module].operation(id),
        }
    }

    /// Return one type through the table owning it, none for a module without a table.
    fn type_at(&self, types: &dir::TypeTable<'_>, type_id: dir::GlobalTypeId) -> Option<dir::Type> {
        if type_id.module_id == types.module_id {
            return Some(types.get_type(type_id.local_id));
        }

        self.foreign_types
            .get(&type_id.module_id)
            .map(|types| types.get_type(type_id.local_id))
    }

    /// Return one operation type label.
    fn operation_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        operation: &dir::TypeOperation,
    ) -> String {
        match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = self.type_id_label(types, *target);
                let mapping = Self::string_mapping_label(*mapping);

                format!("{mapping}<{target}>")
            }
            dir::TypeOperation::Conditional(conditional) => {
                self.conditional_type_label(types, conditional)
            }
            dir::TypeOperation::Narrow(narrow) => {
                let source = self.type_id_label(types, narrow.source);
                let target = self.type_id_label(types, narrow.target);
                if narrow.is_positive {
                    format!("Narrow<{source}, {target}>")
                } else {
                    format!("Narrow<{source}, !{target}>")
                }
            }
            dir::TypeOperation::Mapped(mapped) => self.mapped_type_label(types, mapped),
            dir::TypeOperation::Index(index) => {
                // render indexed access as source-shaped type text
                let left = self.type_id_label(types, index.left);
                let index = self.type_id_label(types, index.index);

                format!("{left}[{index}]")
            }
            dir::TypeOperation::TypeOf(query) => {
                let value = self.symbol_label(query.symbol);

                format!("typeof {value}")
            }
            dir::TypeOperation::Instantiation(application) => {
                let target = self.type_id_label(types, application.target);
                let arguments = types
                    .type_ids(application.arguments)
                    .iter()
                    .map(|argument| self.type_id_label(types, *argument))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{target}<{arguments}>")
            }
            dir::TypeOperation::TemplateLiteral(template) => {
                self.template_type_label(types, template)
            }
            dir::TypeOperation::Infer(infer) => self.infer_type_label(types, infer),
            dir::TypeOperation::KeyOf(unary) => {
                // render unary type operation
                let target = self.type_id_label(types, unary.target);

                format!("keyof {target}")
            }
            dir::TypeOperation::NoInfer(unary) => {
                let target = self.type_id_label(types, unary.target);

                format!("NoInfer<{target}>")
            }
            dir::TypeOperation::Awaited(unary) => {
                let target = self.type_id_label(types, unary.target);

                format!("Awaited<{target}>")
            }
            dir::TypeOperation::SpaceOf(unary) => {
                let target = self.type_id_label(types, unary.target);

                format!("SpaceOf<{target}>")
            }
            dir::TypeOperation::TryOutput { value } => {
                let value = self.type_id_label(types, *value);

                format!("TryOutput<{value}>")
            }
            dir::TypeOperation::TryResidual { value } => {
                let value = self.type_id_label(types, *value);

                format!("TryResidual<{value}>")
            }
            dir::TypeOperation::TryFailure { value } => {
                let value = self.type_id_label(types, *value);

                format!("TryFailure<{value}>")
            }
            dir::TypeOperation::StaticBinary(binary) => {
                let left = self.type_id_label(types, binary.left);
                let right = self.type_id_label(types, binary.right);
                let operator = static_binary_operator_label(binary.operator);

                format!("{left} {operator} {right}")
            }
            dir::TypeOperation::StaticUnary(unary) => {
                let target = self.type_id_label(types, unary.target);
                let operator = static_unary_operator_label(unary.operator);

                format!("{operator}{target}")
            }
        }
    }

    /// Return one string mapping label.
    fn string_mapping_label(function: dir::StringMapping) -> &'static str {
        match function {
            dir::StringMapping::Uppercase => "Uppercase",
            dir::StringMapping::Lowercase => "Lowercase",
            dir::StringMapping::Capitalize => "Capitalize",
            dir::StringMapping::Uncapitalize => "Uncapitalize",
        }
    }

    /// Return one conditional type label.
    fn conditional_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        conditional: &dir::ConditionalType,
    ) -> String {
        // render the four conditional operands
        let left = self.type_id_label(types, conditional.left);
        let right = self.type_id_label(types, conditional.right);
        let then_type = self.type_id_label(types, conditional.then_type);
        let else_type = self.type_id_label(types, conditional.else_type);

        format!("{left} extends {right} ? {then_type} : {else_type}")
    }

    /// Return one mapped type label.
    fn mapped_type_label(&self, types: &dir::TypeTable<'_>, mapped: &dir::MappedType) -> String {
        // render mapped type components
        let parameter = self.strings.get(mapped.parameter.name);
        let constraint = self.type_id_label(types, mapped.parameter.constraint);
        let value = self.type_id_label(types, mapped.value);
        let readonly = Self::mapped_modifier_label("readonly", mapped.modifiers.readonly);
        let optional = Self::mapped_modifier_label("?", mapped.modifiers.optional);

        // render the mapped key clause
        let mut head = format!("[{parameter} in {constraint}");
        if let Some(key_remap) = mapped.parameter.key_remap {
            let key_remap = self.type_id_label(types, key_remap);
            head.push_str(&format!(" as {key_remap}"));
        }
        head.push(']');

        format!("{{ {readonly}{head}{optional}: {value} }}")
    }

    /// Return one mapped modifier label.
    fn mapped_modifier_label(token: &'static str, modifier: dir::MappedTypeModifier) -> String {
        match modifier {
            dir::MappedTypeModifier::Present => token.to_string(),
            dir::MappedTypeModifier::Add => format!("+{token}"),
            dir::MappedTypeModifier::Remove => format!("-{token}"),
            dir::MappedTypeModifier::None => String::new(),
        }
    }

    /// Return one template literal type label.
    fn template_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        template: &dir::TemplateLiteralType,
    ) -> String {
        let mut result = String::from("`");

        // interleave literal parts with type spans
        let strings = types.strings(template.strings);
        let spans = types.type_ids(template.spans);
        for (index, string) in strings.iter().enumerate() {
            result.push_str(self.strings.get(*string));
            if let Some(span) = spans.get(index) {
                let span = self.type_id_label(types, *span);
                result.push_str(&format!("${{{span}}}"));
            }
        }

        result.push('`');

        result
    }

    /// Return one infer type label.
    fn infer_type_label(&self, types: &dir::TypeTable<'_>, infer: &dir::InferType) -> String {
        // render anonymous infer bindings as `_`
        let name = infer
            .name
            .map(|name| self.strings.get(name).to_string())
            .unwrap_or_else(|| "_".to_string());

        // include the pattern constraint when present
        if let Some(constraint) = infer.constraint {
            let constraint = self.type_id_label(types, constraint);

            format!("infer {name} extends {constraint}")
        } else {
            format!("infer {name}")
        }
    }

    /// Return one fixed array type label.
    fn fixed_array_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        array: &dir::FixedArrayType,
    ) -> String {
        // intrinsic collections render their declared names
        let element = self.type_id_label(types, array.element);
        let count = self.type_id_label(types, array.count);

        format!("FixedArray<{element}, {count}>")
    }

    /// Return one range type label.
    fn range_type_label(&self, range: &dir::RangeType) -> String {
        // render omitted bounds as open ends
        let start = range
            .start
            .as_ref()
            .map(|literal| self.scalar_literal_label(literal))
            .unwrap_or_default();
        let end = range
            .end
            .as_ref()
            .map(|literal| self.scalar_literal_label(literal))
            .unwrap_or_default();
        let operator = if range.is_inclusive { "..=" } else { ".." };

        format!("{start}{operator}{end}")
    }

    /// Return one slice type label.
    fn slice_type_label(&self, types: &dir::TypeTable<'_>, slice: &dir::SliceType) -> String {
        // intrinsic collections render their declared names
        let element = self.type_id_label(types, slice.element);

        format!("Slice<{element}>")
    }

    /// Return one tuple type label.
    fn tuple_type_label(&self, types: &dir::TypeTable<'_>, tuple: &dir::TupleType) -> String {
        // render tuple elements with labels and modifiers
        let mut elements = self.tuple_element_list_label(types, types.elements(tuple.elements));

        match tuple.form {
            dir::TupleForm::Tuple => {
                if tuple.elements.len() == 1 {
                    elements.push(',');
                }

                format!("({elements})")
            }
            dir::TupleForm::Array => format!("[{elements}]"),
        }
    }

    /// Return one tuple element label.
    fn tuple_element_label(
        &self,
        types: &dir::TypeTable<'_>,
        element: &dir::TypeElement,
    ) -> String {
        // render element modifiers independently
        let ty = if element.is_rest {
            self.rest_type_label(types, element.ty)
        } else {
            self.type_id_label(types, element.ty)
        };
        let label = element
            .label
            .map(|label| format!("{}: ", self.strings.get(label)))
            .unwrap_or_default();
        let readonly = if element.is_readonly { "readonly " } else { "" };
        let rest = if element.is_rest { "..." } else { "" };
        let optional = if element.is_optional { "?" } else { "" };

        format!("{readonly}{rest}{label}{ty}{optional}")
    }

    /// Return one rest tuple element type label.
    fn rest_type_label(&self, types: &dir::TypeTable<'_>, ty: dir::GlobalTypeId) -> String {
        if ty.module_id != types.module_id {
            return self.global_type_label(ty);
        }

        self.type_id_label(types, ty)
    }

    /// Return one shape type label.
    fn object_type_label(&self, types: &dir::TypeTable<'_>, shape: &dir::ObjectType) -> String {
        // render fields first, then signatures
        let mut fields = types
            .properties(shape.properties)
            .iter()
            .map(|field| self.type_field_label(types, field))
            .collect::<Vec<_>>();

        for signature in types.type_ids(shape.call_signatures) {
            let signature = self.type_id_label(types, *signature);
            fields.push(format!("<call>: {signature}"));
        }

        for signature in types.type_ids(shape.construct_signatures) {
            let signature = self.type_id_label(types, *signature);
            fields.push(format!("<new>: {signature}"));
        }

        for signature in types.index_signatures(shape.index_signatures) {
            fields.push(self.index_signature_label(types, signature));
        }

        if fields.is_empty() {
            return "{}".to_string();
        }

        let fields = fields.join("; ");

        format!("{{ {fields} }}")
    }

    /// Return one type property label.
    fn type_field_label(&self, types: &dir::TypeTable<'_>, field: &dir::TypeProperty) -> String {
        // render common property modifiers
        let key = self.type_field_key_label(field.key);
        let optional = if field.is_optional { "?" } else { "" };

        match field.access {
            dir::PropertyAccess::Read(ty) => {
                if ty.module_id == types.module_id
                    && let dir::Type::FunctionSignature(function) = types.get_type(ty.local_id)
                {
                    let signature = self.method_signature_label(types, types.signature(function));

                    return format!("get {key}(){signature}");
                }

                format!(
                    "readonly {key}{optional}: {}",
                    self.type_id_label(types, ty)
                )
            }
            dir::PropertyAccess::Write(ty) => {
                format!("set {key}(value: {})", self.type_id_label(types, ty))
            }
            dir::PropertyAccess::ReadWrite { read, write } if read == write => {
                if read.module_id == types.module_id
                    && let dir::Type::FunctionSignature(function) = types.get_type(read.local_id)
                {
                    let signature = self.method_signature_label(types, types.signature(function));

                    return format!("{key}{optional}{signature}");
                }

                format!("{key}{optional}: {}", self.type_id_label(types, read))
            }
            dir::PropertyAccess::ReadWrite { read, write } => format!(
                "get {key}(): {}; set {key}(value: {})",
                self.type_id_label(types, read),
                self.type_id_label(types, write)
            ),
        }
    }

    /// Return one field key as it appears in object type text.
    fn type_field_key_label(&self, key: dir::StaticKey) -> String {
        self.static_key(key)
    }

    /// Return one index signature label.
    fn index_signature_label(
        &self,
        types: &dir::TypeTable<'_>,
        signature: &dir::TypeIndexSignature,
    ) -> String {
        // render index signature parts
        let name = self.strings.get(signature.name);
        let key_type = self.type_id_label(types, signature.key_type);
        let value_type = self.type_id_label(types, signature.value_type);
        let readonly = if signature.is_readonly {
            "readonly "
        } else {
            ""
        };
        let optional = if signature.is_optional { "?" } else { "" };

        format!("{readonly}[{name}: {key_type}]{optional}: {value_type}")
    }

    /// Return one method-shaped function label.
    fn method_signature_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionSignatureType,
    ) -> String {
        // render method generics and parameters
        let generics = self.function_generic_label(types, function);
        let parameters =
            self.function_parameter_list_label(types, types.parameters(function.parameters), ", ");

        // render the result type
        let return_type = function
            .return_type
            .map(|ty| self.type_id_label(types, ty))
            .unwrap_or_else(|| "void".to_string());

        format!("{generics}({parameters}): {return_type}")
    }

    /// Return one function type label.
    fn function_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionSignatureType,
    ) -> String {
        // render generics and explicit this parameter
        let generics = self.function_generic_label(types, function);
        let this_parameter = function
            .this_parameter
            .map(|ty| format!("this: {}", self.type_id_label(types, ty)));

        // merge this with ordinary parameters
        let parameters = self.function_parameters_label(
            types,
            this_parameter,
            types.parameters(function.parameters),
        );

        // render the result and function modifiers
        let return_type = function
            .return_type
            .map(|ty| self.type_id_label(types, ty))
            .unwrap_or_else(|| "void".to_string());
        let prefix = if function.asynchrony == dir::Asynchrony::Async {
            "async "
        } else if function.is_construct {
            "new "
        } else {
            ""
        };
        let generator = if function.is_generator { "*" } else { "" };

        format!("{prefix}{generics}({parameters}) => {generator}{return_type}")
    }

    /// Return one function parameter list label.
    fn function_parameters_label(
        &self,
        types: &dir::TypeTable<'_>,
        this_parameter: Option<String>,
        parameters: &[dir::FunctionParameterType],
    ) -> String {
        let parameters = parameters
            .iter()
            .map(|parameter| self.function_parameter_label(types, parameter));

        this_parameter
            .into_iter()
            .chain(parameters)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return one function parameter list label.
    fn function_parameter_list_label(
        &self,
        types: &dir::TypeTable<'_>,
        parameters: &[dir::FunctionParameterType],
        separator: &'static str,
    ) -> String {
        parameters
            .iter()
            .map(|parameter| self.function_parameter_label(types, parameter))
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Return one function parameter label.
    fn function_parameter_label(
        &self,
        types: &dir::TypeTable<'_>,
        parameter: &dir::FunctionParameterType,
    ) -> String {
        let mut label = String::new();
        if parameter.is_rest {
            label.push_str("...");
        }
        let ty = if parameter.is_rest {
            self.rest_type_label(types, parameter.ty)
        } else {
            self.type_id_label(types, parameter.ty)
        };
        label.push_str(&ty);
        if parameter.is_optional {
            label.push('?');
        }

        label
    }

    /// Return one function generic parameter label.
    fn function_generic_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionSignatureType,
    ) -> String {
        let Some(template) = function.template else {
            return String::new();
        };
        let arguments = types.generic_arguments(function.arguments);
        let parameters = self.function_generic_parameter_list_label(types, template, arguments);
        if parameters.is_empty() {
            return String::new();
        }

        format!("<{parameters}>")
    }

    /// Return one generic parameter label in a function signature.
    fn function_generic_parameter_label(
        &self,
        types: &dir::TypeTable<'_>,
        parameter: dir::GlobalGenericParameterId,
    ) -> String {
        let label = self.parameter_type_label(&parameter);
        let label = self.generic_parameter_head_label(&parameter, label);
        let suffix = self.generic_parameter_signature_suffix(types, &parameter);

        format!("{label}{suffix}")
    }

    /// Return one generic parameter label.
    fn parameter_type_label(&self, parameter: &dir::GlobalGenericParameterId) -> String {
        let Some((generics, generic)) = self.generic_parameter_context(parameter) else {
            return format!("generic#{}", parameter.local_id.0);
        };
        let template = generics.get_template(generic.template);

        match generic.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                if symbol.module_id == self.tree.module_id {
                    self.symbol_label(symbol)
                } else {
                    self.symbol_path_label(symbol)
                }
            }
            dir::GenericParameterKey::Anonymous => {
                let owner = match template.symbol {
                    Some(symbol) => self.symbol_path_label(symbol),
                    None => self.node_label(template.source),
                };
                let position = generics.parameter_position(parameter.local_id);
                let regions = self.region_names_before(generics, parameter.local_id);
                let name = generic.canonical_name(position, regions.iter().map(String::as_str));

                format!("{owner}.{name}")
            }
        }
    }

    /// Add generic parameter modifiers to one parameter list label.
    fn generic_parameter_head_label(
        &self,
        parameter: &dir::GlobalGenericParameterId,
        label: String,
    ) -> String {
        let Some((_, generic)) = self.generic_parameter_context(parameter) else {
            return label;
        };

        // print tick parameters bare, their kind is implied
        if self.is_tick_parameter(generic, &label) {
            return label;
        }

        self.generic_parameter_binding_head_label(generic, label)
    }

    /// Return the constraint and default label for one generic parameter.
    fn generic_parameter_signature_suffix(
        &self,
        types: &dir::TypeTable<'_>,
        parameter: &dir::GlobalGenericParameterId,
    ) -> String {
        let Some((_, generic)) = self.generic_parameter_context(parameter) else {
            return String::new();
        };

        // print tick parameters bare, their kind is implied
        if generic.memory_parameter() == Some(dir::MemoryParameter::Region) {
            return String::new();
        }

        let constraint = generic
            .constraint
            .map(|ty| format!(": {}", self.type_id_label(types, ty)))
            .unwrap_or_default();
        let default = generic
            .default
            .map(|ty| format!(" = {}", self.type_id_label(types, ty)))
            .unwrap_or_default();

        format!("{constraint}{default}")
    }

    /// Return the generic slot represented by one parameter type.
    fn generic_parameter_context(
        &self,
        parameter: &dir::GlobalGenericParameterId,
    ) -> Option<(&dir::GenericTable<'_>, &dir::GenericParameterBinding)> {
        let generics = if parameter.module_id == self.tree.module_id {
            self.generics.as_ref()
        } else {
            self.foreign_generics.get(&parameter.module_id)
        }?;
        let generic = generics.get_parameter(parameter.local_id);

        Some((generics, generic))
    }

    /// Return one type id list label.
    fn type_id_list_label(
        &self,
        types: &dir::TypeTable<'_>,
        type_ids: &[dir::GlobalTypeId],
        separator: &'static str,
    ) -> String {
        type_ids
            .iter()
            .map(|type_id| self.type_id_label(types, *type_id))
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Return one tuple element list label.
    fn tuple_element_list_label(
        &self,
        types: &dir::TypeTable<'_>,
        elements: &[dir::TypeElement],
    ) -> String {
        elements
            .iter()
            .map(|element| self.tuple_element_label(types, element))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return one function generic parameter list label.
    fn function_generic_parameter_list_label(
        &self,
        types: &dir::TypeTable<'_>,
        template: dir::GlobalGenericTemplateId,
        arguments: &[dir::GenericArgumentBinding],
    ) -> String {
        let Some(generics) = self.generic_table(template.module_id) else {
            return String::new();
        };
        let template = generics.get_template(template.local_id);

        template
            .parameters
            .iter()
            .map(|parameter| parameter.into_global(generics.module_id))
            .filter(|parameter| {
                !arguments
                    .iter()
                    .any(|binding| binding.parameter == *parameter)
            })
            .map(|parameter| self.function_generic_parameter_label(types, parameter))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return the generic table for one module.
    pub(super) fn generic_table(
        &self,
        module: destack_source::ModuleId,
    ) -> Option<&dir::GenericTable<'_>> {
        if module == self.tree.module_id {
            self.generics.as_ref()
        } else {
            self.foreign_generics.get(&module)
        }
    }
}

/// Return one static binary operator token.
fn static_binary_operator_label(operator: dir::StaticBinaryOperator) -> &'static str {
    match operator {
        dir::StaticBinaryOperator::Add => "+",
        dir::StaticBinaryOperator::Subtract => "-",
        dir::StaticBinaryOperator::Multiply => "*",
        dir::StaticBinaryOperator::Divide => "/",
        dir::StaticBinaryOperator::Remainder => "%",
        dir::StaticBinaryOperator::Exponent => "**",
        dir::StaticBinaryOperator::ShiftLeft => "<<",
        dir::StaticBinaryOperator::ShiftRight => ">>",
        dir::StaticBinaryOperator::UnsignedShiftRight => ">>>",
        dir::StaticBinaryOperator::BitwiseAnd => "&",
        dir::StaticBinaryOperator::BitwiseXor => "^",
        dir::StaticBinaryOperator::BitwiseOr => "|",
        dir::StaticBinaryOperator::Equal => "==",
        dir::StaticBinaryOperator::EqualStrict => "===",
        dir::StaticBinaryOperator::NotEqual => "!=",
        dir::StaticBinaryOperator::NotEqualStrict => "!==",
        dir::StaticBinaryOperator::LessThan => "<",
        dir::StaticBinaryOperator::LessThanOrEqual => "<=",
        dir::StaticBinaryOperator::GreaterThan => ">",
        dir::StaticBinaryOperator::GreaterThanOrEqual => ">=",
        dir::StaticBinaryOperator::And => "&&",
        dir::StaticBinaryOperator::Or => "||",
    }
}

/// Return one static unary operator token.
fn static_unary_operator_label(operator: dir::StaticUnaryOperator) -> &'static str {
    match operator {
        dir::StaticUnaryOperator::Not => "!",
        dir::StaticUnaryOperator::Negate => "-",
        dir::StaticUnaryOperator::BitwiseNot => "~",
    }
}
