use destack_dir as dir;

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

        self.type_label_from_value(types, ty)
    }

    /// Return one type text label.
    fn type_label_from_value(&self, types: &dir::TypeTable<'_>, ty: &dir::Type) -> String {
        match ty {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Never => "never".to_string(),
            dir::Type::Any => "any".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Object => "object".to_string(),
            dir::Type::Primitive(primitive) => Self::primitive_type_label(*primitive),
            dir::Type::Literal(literal) => self.scalar_literal_label(literal),
            dir::Type::Parameter(parameter) => self.parameter_type_label(parameter),
            dir::Type::Reference(named) => self.reference_type_label(named),
            dir::Type::This => "this".to_string(),
            dir::Type::Member(member) => self.member_type_label(types, member),
            dir::Type::Form(form) => self.form_type_label(types, form),
            dir::Type::Dynamic(any) => {
                let constraint = self.type_id_label(types, any.constraint);

                format!("Dynamic<{constraint}>")
            }
            dir::Type::Operation(operation) => self.operation_type_label(types, operation),
            dir::Type::Array(array) => self.array_type_label(types, array),
            dir::Type::FixedArray(array) => self.fixed_array_type_label(types, array),
            dir::Type::Range(range) => self.range_type_label(range),
            dir::Type::Slice(slice) => self.slice_type_label(types, slice),
            dir::Type::Tuple(tuple) => self.tuple_type_label(types, tuple),
            dir::Type::Shape(shape) => self.shape_type_label(types, shape),
            dir::Type::Function(function) => self.function_type_label(types, function),
            dir::Type::Closure(closure) => self.closure_type_label(types, closure),
            dir::Type::Union(union) => self.type_id_list_label(types, &union.elements, " | "),
            dir::Type::Intersection(intersection) => {
                self.type_id_list_label(types, &intersection.elements, " & ")
            }
        }
    }

    /// Return one closure type label.
    fn closure_type_label(&self, types: &dir::TypeTable<'_>, closure: &dir::ClosureType) -> String {
        let function = self.type_id_label(types, closure.function);
        let environment = self.type_id_label(types, closure.environment);

        format!("Closure<{function}, {environment}>")
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
            dir::PrimitiveType::Symbol => "symbol".to_string(),
            dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
        }
    }

    /// Return one reference type label.
    fn reference_type_label(&self, reference: &dir::ReferenceType) -> String {
        if reference.arguments.is_empty() {
            return self.symbol_path_label(reference.symbol);
        }

        if let Some(label) = self.collection_type_label(reference.symbol, &reference.arguments) {
            return label;
        }

        // render static arguments only when the reference is applied
        let arguments = self.static_argument_list_label(&reference.arguments);
        let symbol = self.symbol_path_label(reference.symbol);

        format!("{symbol}<{arguments}>")
    }

    /// Return one member type label.
    fn member_type_label(&self, types: &dir::TypeTable<'_>, member: &dir::MemberType) -> String {
        let owner = self.type_id_label(types, member.owner);
        let key = self.static_key(member.key);
        if member.arguments.is_empty() {
            return format!("{owner}.{key}");
        }

        let arguments = self.static_argument_list_label(&member.arguments);

        format!("{owner}.{key}<{arguments}>")
    }

    /// Return one collection type label.
    pub(super) fn collection_type_label(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::StaticArgument],
    ) -> Option<String> {
        let item = self.language_item_by_symbol.get(&symbol)?;
        let unnamed = arguments.iter().all(|argument| argument.name.is_none());
        if !unnamed {
            return None;
        }

        match (item, arguments) {
            (dir::LanguageItem::Array, [element]) => {
                let element = self.static_argument_value_label(element.value);

                Some(format!("Array<{element}>"))
            }
            (dir::LanguageItem::ReadonlyArray, [element]) => {
                let element = self.static_argument_value_label(element.value);

                Some(format!("ReadonlyArray<{element}>"))
            }
            (dir::LanguageItem::FixedArray, [element, count]) => {
                let element = self.static_argument_value_label(element.value);
                let count = self.static_argument_value_label(count.value);

                Some(format!("FixedArray<{element}, {count}>"))
            }
            (dir::LanguageItem::Slice, [element]) => {
                let element = self.static_argument_value_label(element.value);

                Some(format!("Slice<{element}>"))
            }
            _ => None,
        }
    }

    /// Return one form type label.
    fn form_type_label(&self, types: &dir::TypeTable<'_>, form: &dir::FormType) -> String {
        // render the payload once, all forms wrap the same value
        let value = self.type_id_label(types, form.value);

        // render canonical form constructors
        match &form.form {
            dir::Form::Managed => format!("Managed<{value}>"),
            dir::Form::Owned => format!("Owned<{value}>"),
            dir::Form::Borrowed { lifetime, access } => {
                let lifetime = self.global_static_label(*lifetime);
                let access = self.global_static_label(*access);

                format!("Borrowed<{value}, {lifetime}, {access}>")
            }
            dir::Form::Raw => format!("Raw<{value}>"),
            dir::Form::Placed { place } => {
                let place = self.global_static_label(*place);

                format!("Placed<{value}, {place}>")
            }
            dir::Form::Readonly => format!("Readonly<{value}>"),
        }
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
            dir::TypeOperation::Mapped(mapped) => self.mapped_type_label(types, mapped),
            dir::TypeOperation::Index(index) => {
                // render indexed access as source-shaped type text
                let left = self.type_id_label(types, index.left);
                let index = self.type_id_label(types, index.index);

                format!("{left}[{index}]")
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
        for (index, string) in template.strings.iter().enumerate() {
            result.push_str(self.strings.get(*string));
            if let Some(span) = template.spans.get(index) {
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
    fn array_type_label(&self, types: &dir::TypeTable<'_>, array: &dir::ArrayType) -> String {
        // render homogeneous array notation
        let element = self.type_id_label(types, array.element);

        format!("{element}[]")
    }

    /// Return one fixed array type label.
    fn fixed_array_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        array: &dir::FixedArrayType,
    ) -> String {
        // render element and count
        let element = self.type_id_label(types, array.element);
        let count = self.global_static_label(array.count);

        format!("[{element}; {count}]")
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
        // render slice notation
        let element = self.type_id_label(types, slice.element);

        format!("[{element}]")
    }

    /// Return one tuple type label.
    fn tuple_type_label(&self, types: &dir::TypeTable<'_>, tuple: &dir::TupleType) -> String {
        // render tuple elements with labels and modifiers
        let mut elements = self.tuple_element_list_label(types, &tuple.elements);

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
        let ty = self.type_id_label(types, element.ty);
        let label = element
            .label
            .map(|label| format!("{}: ", self.strings.get(label)))
            .unwrap_or_default();
        let readonly = if element.is_readonly { "readonly " } else { "" };
        let rest = if element.is_rest { "..." } else { "" };
        let optional = if element.is_optional { "?" } else { "" };

        format!("{readonly}{rest}{label}{ty}{optional}")
    }

    /// Return one shape type label.
    fn shape_type_label(&self, types: &dir::TypeTable<'_>, shape: &dir::ShapeType) -> String {
        // render fields first, then signatures
        let mut fields = shape
            .fields
            .iter()
            .map(|field| self.type_field_label(types, field))
            .collect::<Vec<_>>();

        for signature in &shape.call_signatures {
            let signature = self.type_id_label(types, *signature);
            fields.push(format!("<call>: {signature}"));
        }

        for signature in &shape.construct_signatures {
            let signature = self.type_id_label(types, *signature);
            fields.push(format!("<new>: {signature}"));
        }

        for signature in &shape.index_signatures {
            fields.push(self.index_signature_label(types, signature));
        }

        let fields = fields.join("; ");

        format!("{{ {fields} }}")
    }

    /// Return one type field label.
    fn type_field_label(&self, types: &dir::TypeTable<'_>, field: &dir::TypeField) -> String {
        // render common field modifiers
        let key = self.static_key(field.key);
        let readonly = if field.is_readonly { "readonly " } else { "" };
        let optional = if field.is_optional { "?" } else { "" };

        if field.ty.module_id == types.module_id
            && let dir::Type::Function(function) = types.get_type(field.ty.local_id)
        {
            let signature = self.method_signature_label(types, function);

            format!("{readonly}{key}{optional}{signature}")
        }
        // otherwise print the field as a property
        else {
            let ty = self.type_id_label(types, field.ty);

            format!("{readonly}{key}{optional}: {ty}")
        }
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
        function: &dir::FunctionTypeShape,
    ) -> String {
        // render method generics and parameters
        let generics = self.function_generic_label(types, function);
        let parameters = self.function_parameter_list_label(types, &function.parameters, ", ");

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
        function: &dir::FunctionTypeShape,
    ) -> String {
        // render generics and explicit this parameter
        let generics = self.function_generic_label(types, function);
        let this_parameter = function
            .this_parameter
            .map(|ty| format!("this: {}", self.type_id_label(types, ty)));

        // merge this with ordinary parameters
        let parameters =
            self.function_parameters_label(types, this_parameter, &function.parameters);

        // render the result and function modifiers
        let return_type = function
            .return_type
            .map(|ty| self.type_id_label(types, ty))
            .unwrap_or_else(|| "void".to_string());
        let prefix = if function.asynchrony == dir::Asynchrony::Async {
            "async "
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
        let parameters = this_parameter
            .into_iter()
            .chain(parameters)
            .collect::<Vec<_>>()
            .join(", ");

        parameters
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
        label.push_str(&self.type_id_label(types, parameter.ty));
        if parameter.is_optional {
            label.push('?');
        }

        label
    }

    /// Return one function generic parameter label.
    fn function_generic_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionTypeShape,
    ) -> String {
        if function.generic_parameters.is_empty() {
            return String::new();
        }

        // render generic parameters with constraints
        let parameters = self.function_generic_parameter_list_label(types, function);

        format!("<{parameters}>")
    }

    /// Return one generic parameter label in a function signature.
    fn function_generic_parameter_label(
        &self,
        types: &dir::TypeTable<'_>,
        type_id: dir::GlobalTypeId,
    ) -> String {
        if type_id.module_id != types.module_id {
            return self.global_type_label(type_id);
        }

        // render parameter symbols with their constraints
        if let dir::Type::Parameter(parameter) = types.get_type(type_id.local_id) {
            let label = self.parameter_type_label(parameter);
            let suffix = self.generic_slot_signature_suffix(types, parameter);

            format!("{label}{suffix}")
        }
        // fall back to the nested type label
        else {
            self.type_id_label(types, type_id)
        }
    }

    /// Return one generic parameter label.
    fn parameter_type_label(&self, parameter: &dir::GenericParameterRef) -> String {
        match parameter.key {
            dir::GenericSlotKey::Symbol(symbol) => self.symbol_label(symbol),
            dir::GenericSlotKey::Generated(name) => {
                let owner = self.symbol_path_label(parameter.owner);
                let name = self.strings.get(name);

                format!("{owner}.{name}")
            }
        }
    }

    /// Return the constraint and default label for one generic parameter.
    fn generic_slot_signature_suffix(
        &self,
        types: &dir::TypeTable<'_>,
        parameter: &dir::GenericParameterRef,
    ) -> String {
        let Some(slot) = self.generic_slot_for_parameter(parameter) else {
            return String::new();
        };

        match slot {
            dir::GenericSlot::Type {
                constraint,
                default,
                ..
            }
            | dir::GenericSlot::VariadicType {
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|ty| format!(": {}", self.type_id_label(types, ty)))
                    .unwrap_or_default();
                let default = default
                    .map(|ty| format!(" = {}", self.type_id_label(types, ty)))
                    .unwrap_or_default();

                format!("{constraint}{default}")
            }
            dir::GenericSlot::Static {
                constraint,
                default,
                ..
            }
            | dir::GenericSlot::VariadicStatic {
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|ty| format!(": {}", self.type_id_label(types, ty)))
                    .unwrap_or_default();
                let default = default
                    .map(|static_id| format!(" = {}", self.global_static_label(static_id)))
                    .unwrap_or_default();

                format!("{constraint}{default}")
            }
        }
    }

    /// Return the generic slot represented by one parameter type.
    fn generic_slot_for_parameter(
        &self,
        parameter: &dir::GenericParameterRef,
    ) -> Option<&dir::GenericSlot> {
        let generics = self.generics.as_ref()?;

        generics.iter_slots().find_map(|(_, slot)| {
            let template = generics.get_template(slot.template());
            let is_match = template.owner == parameter.owner
                && slot.key() == parameter.key
                && slot.index() == parameter.index;

            is_match.then_some(slot)
        })
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

    /// Return one static argument list label.
    fn static_argument_list_label(&self, arguments: &[dir::StaticArgument]) -> String {
        arguments
            .iter()
            .map(|argument| self.static_argument_label(argument))
            .collect::<Vec<_>>()
            .join(", ")
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
        function: &dir::FunctionTypeShape,
    ) -> String {
        function
            .generic_parameters
            .iter()
            .map(|type_id| self.function_generic_parameter_label(types, *type_id))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
