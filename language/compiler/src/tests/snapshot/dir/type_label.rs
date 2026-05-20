use destack_dir as dir;

use super::DirSnapshotBuilder;

impl DirSnapshotBuilder<'_> {
    /// Return one semantic type label.
    pub(super) fn semantic_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
    ) -> String {
        // resolve the canonical type slot
        let ty = types.get_type(type_id);

        self.type_text(types, ty)
    }

    /// Return one type text label.
    fn type_text(&self, types: &dir::TypeTable<'_>, ty: &dir::Type) -> String {
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
            dir::Type::Parameter(parameter) => self.symbol_label(parameter.symbol),
            dir::Type::Named(named) => self.named_type_label(named),
            dir::Type::This => "this".to_string(),
            dir::Type::Form(form) => self.form_type_label(types, form),
            dir::Type::ErasedAny(any) => {
                let constraint = self.type_id_label(types, any.constraint);

                format!("Any<{constraint}>")
            }
            dir::Type::Predicate(predicate) => self.predicate_type_label(types, predicate),
            dir::Type::Operation(operation) => self.operation_type_label(types, operation),
            dir::Type::FixedArray(array) => self.fixed_array_type_label(types, array),
            dir::Type::Range(range) => self.range_type_label(range),
            dir::Type::Slice(slice) => self.slice_type_label(types, slice),
            dir::Type::Tuple(tuple) => self.tuple_type_label(types, tuple),
            dir::Type::Shape(shape) => self.shape_type_label(types, shape),
            dir::Type::Function(function) => self.function_type_label(types, function),
            dir::Type::Union(union) => self.type_id_list_label(types, &union.elements, " | "),
            dir::Type::Intersection(intersection) => {
                self.type_id_list_label(types, &intersection.elements, " & ")
            }
        }
    }

    /// Return one type id label through a table.
    fn type_id_label(&self, types: &dir::TypeTable<'_>, type_id: dir::LocalTypeId) -> String {
        self.semantic_type_label(types, type_id)
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

    /// Return one named type label.
    fn named_type_label(&self, named: &dir::NamedType) -> String {
        // render the source declaration path first
        let symbol = self.symbol_path_label(named.symbol);
        if named.arguments.is_empty() {
            return symbol;
        }

        // render static arguments only when the reference is applied
        let arguments = self.static_argument_list_label(&named.arguments);

        format!("{symbol}<{arguments}>")
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
                let lifetime = self.static_label(*lifetime);
                let access = self.static_label(*access);

                format!("Borrowed<{value}, {lifetime}, {access}>")
            }
            dir::Form::Raw => format!("Raw<{value}>"),
            dir::Form::Placed { place } => {
                let place = self.static_label(*place);

                format!("Placed<{value}, {place}>")
            }
            dir::Form::Readonly => format!("Readonly<{value}>"),
        }
    }

    /// Return one predicate type label.
    fn predicate_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        predicate: &dir::PredicateType,
    ) -> String {
        // render the predicate subject
        let subject = match predicate.subject {
            dir::PredicateSubject::Symbol(symbol) => self.symbol_label(symbol),
            dir::PredicateSubject::This => "this".to_string(),
        };

        // render assertion predicates
        if predicate.asserts {
            if let Some(target) = predicate.target {
                let target = self.type_id_label(types, target);

                format!("asserts {subject} is {target}")
            } else {
                format!("asserts {subject}")
            }
        }
        // render ordinary type predicates
        else if let Some(target) = predicate.target {
            let target = self.type_id_label(types, target);

            format!("{subject} is {target}")
        }
        // fall back to the subject
        else {
            subject
        }
    }

    /// Return one operation type label.
    fn operation_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        operation: &dir::TypeOperation,
    ) -> String {
        match operation {
            dir::TypeOperation::BuiltinTypeFunction(function) => {
                Self::builtin_type_function_label(*function).to_string()
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

    /// Return one builtin type function label.
    fn builtin_type_function_label(function: dir::BuiltinTypeFunction) -> &'static str {
        match function {
            dir::BuiltinTypeFunction::Uppercase => "Uppercase",
            dir::BuiltinTypeFunction::Lowercase => "Lowercase",
            dir::BuiltinTypeFunction::Capitalize => "Capitalize",
            dir::BuiltinTypeFunction::Uncapitalize => "Uncapitalize",
            dir::BuiltinTypeFunction::NoInfer => "NoInfer",
            dir::BuiltinTypeFunction::BuiltinIteratorReturn => "BuiltinIteratorReturn",
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
    fn mapped_modifier_label(token: &'static str, modifier: dir::TypeMappedModifier) -> String {
        match modifier {
            dir::TypeMappedModifier::Present => token.to_string(),
            dir::TypeMappedModifier::Add => format!("+{token}"),
            dir::TypeMappedModifier::Remove => format!("-{token}"),
            dir::TypeMappedModifier::None => String::new(),
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
    fn fixed_array_type_label(
        &self,
        types: &dir::TypeTable<'_>,
        array: &dir::FixedArrayType,
    ) -> String {
        // render element and count
        let element = self.type_id_label(types, array.element);
        let count = self.static_label(array.count);
        let prefix = if array.is_readonly { "readonly " } else { "" };

        format!("{prefix}[{element}; {count}]")
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
        // render readonly slice notation
        let element = self.type_id_label(types, slice.element);
        let prefix = if slice.is_readonly { "readonly " } else { "" };

        format!("{prefix}{element}[]")
    }

    /// Return one tuple type label.
    fn tuple_type_label(&self, types: &dir::TypeTable<'_>, tuple: &dir::TupleType) -> String {
        // render tuple elements with labels and modifiers
        let elements = self.tuple_element_list_label(types, &tuple.elements);
        let prefix = if tuple.is_readonly { "readonly " } else { "" };

        format!("{prefix}[{elements}]")
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

        // print function fields as method signatures
        if let dir::Type::Function(function) = types.get_type(field.ty) {
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
        function: &dir::FunctionType,
    ) -> String {
        // render method generics and parameters
        let generics = self.function_generic_label(types, function);
        let parameters = self.type_id_list_label(types, &function.parameters, ", ");

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
        function: &dir::FunctionType,
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
        parameters: &[dir::LocalTypeId],
    ) -> String {
        let parameters = parameters
            .iter()
            .map(|type_id| self.type_id_label(types, *type_id));
        let parameters = this_parameter
            .into_iter()
            .chain(parameters)
            .collect::<Vec<_>>()
            .join(", ");

        parameters
    }

    /// Return one function generic parameter label.
    fn function_generic_label(
        &self,
        types: &dir::TypeTable<'_>,
        function: &dir::FunctionType,
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
        type_id: dir::LocalTypeId,
    ) -> String {
        // render parameter symbols with their constraints
        if let dir::Type::Parameter(parameter) = types.get_type(type_id) {
            let symbol = self.symbol_label(parameter.symbol);

            if let Some(generics) = self.generics.as_ref()
                && let Some(dir::GenericSlot::Type {
                    constraint: Some(constraint),
                    ..
                }) = generics.slot(parameter.symbol)
            {
                let constraint = self.type_id_label(types, *constraint);

                return format!("{symbol}: {constraint}");
            }

            symbol
        }
        // fall back to the nested type label
        else {
            self.type_id_label(types, type_id)
        }
    }

    /// Return one type id list label.
    fn type_id_list_label(
        &self,
        types: &dir::TypeTable<'_>,
        type_ids: &[dir::LocalTypeId],
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
        function: &dir::FunctionType,
    ) -> String {
        function
            .generic_parameters
            .iter()
            .map(|type_id| self.function_generic_parameter_label(types, *type_id))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
