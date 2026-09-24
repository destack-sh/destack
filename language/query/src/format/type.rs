use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format one authored type expression.
    pub(super) fn type_expression(
        &self,
        type_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> QueryResult<String> {
        let view = self.module.view()?;
        let span = self.module.node_span(view, type_id.into())?;

        self.module.source_text(span)
    }

    /// Format one node type.
    pub(crate) fn node_type(&self, node_id: dir::GlobalNodeIdAny) -> QueryResult<String> {
        let Some(type_id) = self.types()?.get_node_type_id(node_id) else {
            return Err(QueryError::missing(format!("node type: {node_id:?}")));
        };

        self.global_type(type_id)
    }

    /// Format one global type id.
    pub(crate) fn global_type(&self, type_id: dir::GlobalTypeId) -> QueryResult<String> {
        self.read_type(type_id, |type_value, formatter| {
            formatter.local_type(type_value)
        })
    }

    /// Format one type owned by this formatter's module.
    pub(super) fn local_type(&self, type_value: &dir::Type) -> QueryResult<String> {
        let text = match type_value {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Never => "never".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Object(shape) => return self.object(*shape),
            dir::Type::Primitive(primitive) => self.primitive(*primitive),
            dir::Type::Literal(literal) => self.literal(*literal),
            dir::Type::Application(instance) => return self.instance(*instance),
            dir::Type::Reference(reference) => {
                let name = self.instance(dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: reference.arguments,
                })?;
                let module = self.program.module(reference.symbol.module_id)?;
                let bindings = module.bindings()?;

                if bindings.get_symbol(reference.symbol.local_id).kind == dir::SymbolKind::Class {
                    format!("typeof {name}")
                } else {
                    name
                }
            }
            dir::Type::Region(region) => {
                let extent = self.global_type(region.extent)?;
                let space = self.global_type(region.space)?;

                format!("{extent} & {space}")
            }
            dir::Type::Parameter(parameter) => {
                // declared parameters reopen at their use-site arguments
                if let Some(reopen) = self.reopening
                    && let Some(argument) = reopen.parameters.get(parameter)
                {
                    return self.global_type(*argument);
                }

                return self.generic_parameter_type(*parameter);
            }
            dir::Type::Erased(parameter) => {
                return self.generic_parameter_type(*parameter);
            }
            dir::Type::Member(member) => return self.member(*self.types()?.member(*member)),
            dir::Type::Refined(refined) => {
                let refined = *self.types()?.refined(*refined);
                let base = self.type_operand(refined.base, dir::TypeOperand::Postfix)?;
                let key = self.property_key(refined.key);
                let value = self.global_type(refined.value)?;

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::Variant(variant) => return self.symbol(variant.variant),
            dir::Type::Form(form) => return self.form(*form),
            dir::Type::Dynamic(dynamic) => {
                let constraint = self.global_type(dynamic.constraint)?;

                format!("Dynamic<{constraint}>")
            }
            dir::Type::FixedArray(array) => {
                let element = self.global_type(array.element)?;
                let count = self.global_type(array.count)?;

                format!("[{element}; {count}]")
            }
            dir::Type::Range(range) => return self.range(*range),
            dir::Type::Slice(slice) => {
                let element = self.global_type(slice.element)?;

                format!("[{element}]")
            }
            dir::Type::Tuple(tuple) => return self.tuple(*tuple),
            dir::Type::FunctionSignature(function) => {
                return self.function_type(self.types()?.signature(*function), None);
            }
            // print a function value as its signature under the elided receiver
            dir::Type::Function(function) => {
                let mode = self.receiver_mode(function.receiver)?;
                if mode.is_some_and(dir::ReceiverMode::is_elided) {
                    return self.global_type(function.signature);
                }
                let (parameters, return_type) = self.callable_arguments(function.signature)?;

                // print a closed mode as its literal, an open receiver term as itself
                let receiver = match mode {
                    Some(mode) => format!("\"{}\"", mode.text()),
                    None => self.global_type(function.receiver)?,
                };

                return Ok(format!("Function<{parameters}, {return_type}, {receiver}>"));
            }
            dir::Type::FunctionPointer(function) => {
                let (parameters, return_type) = self.callable_arguments(function.signature)?;

                return Ok(format!("FunctionPointer<{parameters}, {return_type}>"));
            }
            dir::Type::Union(union) => {
                return self.type_list(union.elements, " | ", dir::TypeOperand::Union);
            }
            dir::Type::Intersection(intersection) => {
                return self.type_list(
                    intersection.elements,
                    " & ",
                    dir::TypeOperand::Intersection,
                );
            }
            dir::Type::This => {
                // declared `this` reopens at the use-site receiver
                if let Some(reopen) = self.reopening
                    && let Some(receiver) = reopen.receiver
                {
                    return self.global_type(receiver);
                }

                "this".to_string()
            }
            dir::Type::Operation(operation) => {
                return self.operation(self.types()?.operation(*operation));
            }
            dir::Type::Key(key) => self.key_type(*key),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::Variable(_) | dir::Type::Static(_) => {
                return Err(QueryError::invalid(format!(
                    "type formatting: {type_value:?}"
                )));
            }
        };

        Ok(text)
    }

    /// Format one primitive type.
    fn primitive(&self, primitive: dir::PrimitiveType) -> String {
        match primitive {
            dir::PrimitiveType::Boolean => "boolean".to_string(),
            dir::PrimitiveType::Character => "char".to_string(),
            dir::PrimitiveType::String => "string".to_string(),
            dir::PrimitiveType::Bigint => "bigint".to_string(),
            dir::PrimitiveType::Integer(integer) => self.integer(integer),
            dir::PrimitiveType::Float(float) => float.as_str().to_string(),
        }
    }

    /// Format one integer type.
    fn integer(&self, integer: dir::IntegerType) -> String {
        match integer {
            dir::IntegerType::Fixed {
                width,
                is_signed: true,
            } => format!("int{width}"),
            dir::IntegerType::Fixed {
                width,
                is_signed: false,
            } => format!("uint{width}"),
            dir::IntegerType::Pointer { is_signed: true } => "isize".to_string(),
            dir::IntegerType::Pointer { is_signed: false } => "usize".to_string(),
        }
    }

    /// Format one generic instance.
    fn instance(&self, instance: dir::GenericApplication) -> QueryResult<String> {
        let arguments = self.types()?.type_ids(instance.arguments);

        // array applications render in their written rest form
        if self
            .program
            .environment_bound()?
            .language
            .item(instance.symbol)
            == Some(dir::LanguageItem::Array)
            && let [element] = arguments
        {
            let element = self.type_operand(*element, dir::TypeOperand::Postfix)?;

            return Ok(format!("{element}[]"));
        }

        let symbol = self.symbol(instance.symbol)?;
        if arguments.is_empty() {
            return Ok(symbol);
        }

        let arguments = self.join_types(arguments, ", ")?;

        Ok(format!("{symbol}<{arguments}>"))
    }

    /// Format one member type.
    fn member(&self, member: dir::MemberType) -> QueryResult<String> {
        let owner = self.type_operand(member.owner, dir::TypeOperand::Postfix)?;
        let key = self.member_key(member.key);
        let arguments = self.types()?.type_ids(member.arguments);

        if arguments.is_empty() {
            return Ok(format!("{owner}{key}"));
        }

        let arguments = self.join_types(arguments, ", ")?;

        Ok(format!("{owner}{key}<{arguments}>"))
    }

    /// Format one canonical memory form.
    pub(super) fn form(&self, form: dir::FormType) -> QueryResult<String> {
        let value = self.type_operand(form.value, dir::TypeOperand::Prefix)?;
        let text = match form.form {
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed(borrow) => self.borrowed_form(borrow, &value)?,
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
        };

        Ok(text)
    }

    /// Format one borrowed form.
    fn borrowed_form(&self, borrow: dir::BorrowFormId, value: &str) -> QueryResult<String> {
        let borrow = *self.types()?.borrow_form(borrow);
        let (region, access) = dir::read_borrow(
            &borrow,
            |id| self.read_type(id, |type_value, _| Ok(*type_value)),
            |text| self.module.strings().get(text).to_string(),
            |parameter| self.generic_parameter_type(parameter),
        )?;
        match dir::borrow_text(region.as_ref(), access.as_ref(), value) {
            Some(text) => Ok(text),
            None => {
                let region = self.global_type(borrow.region)?;
                let access = self.global_type(borrow.access)?;

                let borrowed = dir::LanguageItem::Borrowed.export_name();

                Ok(format!("{borrowed}<{value}, {region}, {access}>"))
            }
        }
    }

    /// Return the receiver mode one function value names, none for an open receiver term.
    fn receiver_mode(&self, receiver: dir::GlobalTypeId) -> QueryResult<Option<dir::ReceiverMode>> {
        self.read_type(receiver, |type_value, formatter| match type_value {
            dir::Type::Literal(dir::Literal::String(text)) => {
                dir::ReceiverMode::from_text(formatter.module.strings().get(*text))
                    .map(Some)
                    .ok_or_else(|| QueryError::invalid(format!("receiver mode: {receiver:?}")))
            }
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::Variable(_) => Ok(None),
            _ => Err(QueryError::invalid(format!("receiver mode: {receiver:?}"))),
        })
    }

    /// Print the parameter tuple and return type one callable application names.
    fn callable_arguments(&self, signature: dir::GlobalTypeId) -> QueryResult<(String, String)> {
        self.read_type(signature, |type_value, formatter| {
            let dir::Type::FunctionSignature(signature) = type_value else {
                return Err(QueryError::invalid(format!(
                    "callable signature: {signature:?}"
                )));
            };
            let signature = *formatter.types()?.signature(*signature);

            // print each parameter type, spreading a rest parameter
            let mut parameters = Vec::new();
            for parameter in formatter.types()?.parameters(signature.parameters) {
                let ty = formatter.global_type(parameter.ty)?;
                parameters.push(match parameter.is_rest {
                    true => format!("...{ty}"),
                    false => ty,
                });
            }
            let parameters = match parameters.as_slice() {
                [parameter] => format!("({parameter},)"),
                _ => format!("({})", parameters.join(", ")),
            };
            let return_type = match signature.return_type {
                Some(return_type) => formatter.global_type(return_type)?,
                None => "void".to_string(),
            };

            Ok((parameters, return_type))
        })
    }

    /// Format one scalar interval type.
    fn range(&self, range: dir::RangeType) -> QueryResult<String> {
        let start = match range.start {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let end = match range.end {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let operator = if range.is_inclusive { "..=" } else { ".." };

        Ok(format!("{start}{operator}{end}"))
    }

    /// Format one tuple type.
    fn tuple(&self, tuple: dir::TupleType) -> QueryResult<String> {
        let elements = self.types()?.elements(tuple.elements);
        let is_singleton = elements.len() == 1;
        let mut formatted_elements = Vec::with_capacity(elements.len());
        for element in elements {
            formatted_elements.push(self.tuple_element(element)?);
        }
        let mut elements = formatted_elements.join(", ");
        if is_singleton {
            elements.push(',');
        }

        let text = match tuple.form {
            dir::TupleForm::Tuple => format!("({elements})"),
            dir::TupleForm::Array => format!("[{elements}]"),
        };

        Ok(text)
    }

    /// Format one tuple element.
    fn tuple_element(&self, element: &dir::TypeElement) -> QueryResult<String> {
        let mut text = String::new();

        if element.is_readonly {
            text.push_str("readonly ");
        }
        if element.is_rest {
            text.push_str("...");
        }
        if let Some(label) = element.label {
            text.push_str(self.module.strings().get(label));
            if element.is_optional {
                text.push('?');
            }
            text.push_str(": ");
        }
        let type_text = self.global_type(element.ty)?;
        text.push_str(&type_text);

        Ok(text)
    }

    /// Format one anonymous object type.
    fn object(&self, shape: dir::ObjectType) -> QueryResult<String> {
        let mut members = Vec::new();

        for property in self.types()?.properties(shape.properties) {
            members.push(self.property(property)?);
        }
        for signature in self.types()?.index_signatures(shape.index_signatures) {
            members.push(self.index_signature(signature)?);
        }

        if members.is_empty() {
            Ok("{}".to_string())
        } else {
            Ok(format!("{{ {} }}", members.join("; ")))
        }
    }

    /// Format one structural property.
    fn property(&self, property: &dir::TypeProperty) -> QueryResult<String> {
        let optional = if property.is_optional { "?" } else { "" };
        let key = self.property_key(property.key);

        match property.access {
            dir::PropertyAccess::Read(ty) => {
                let ty = self.global_type(ty)?;

                Ok(format!("readonly {key}{optional}: {ty}"))
            }
            dir::PropertyAccess::Write(ty) => {
                let ty = self.global_type(ty)?;

                Ok(format!("set {key}(value: {ty})"))
            }
            dir::PropertyAccess::ReadWrite { read, write } if read == write => {
                let ty = self.global_type(read)?;

                Ok(format!("{key}{optional}: {ty}"))
            }
            dir::PropertyAccess::ReadWrite { read, write } => {
                let read = self.global_type(read)?;
                let write = self.global_type(write)?;

                Ok(format!("get {key}(): {read}; set {key}(value: {write})"))
            }
        }
    }

    /// Format one index signature.
    fn index_signature(&self, signature: &dir::TypeIndexSignature) -> QueryResult<String> {
        let readonly = if signature.is_readonly {
            "readonly "
        } else {
            ""
        };
        let optional = if signature.is_optional { "?" } else { "" };
        let name = self.module.strings().get(signature.name);
        let key_type = self.global_type(signature.key_type)?;
        let value_type = self.global_type(signature.value_type)?;

        Ok(format!(
            "{readonly}[{name}: {key_type}]{optional}: {value_type}"
        ))
    }

    /// Format one list of type operands.
    fn type_list(
        &self,
        list: dir::TypeListId,
        separator: &str,
        operand: dir::TypeOperand,
    ) -> QueryResult<String> {
        let types = self.types()?.type_ids(list);
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(self.type_operand(*type_id, operand)?);
        }

        Ok(formatted_types.join(separator))
    }

    /// Format and join global type ids.
    fn join_types(&self, types: &[dir::GlobalTypeId], separator: &str) -> QueryResult<String> {
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(self.global_type(*type_id)?);
        }

        Ok(formatted_types.join(separator))
    }

    /// Format one type operand with the grouping required by its parent.
    pub(super) fn type_operand(
        &self,
        type_id: dir::GlobalTypeId,
        operand: dir::TypeOperand,
    ) -> QueryResult<String> {
        self.read_type(type_id, |type_value, formatter| {
            let text = formatter.local_type(type_value)?;
            let types = formatter.types()?;
            let is_grouped = type_value.needs_parentheses(
                operand,
                |id| Ok(*types.operation(id)),
                |receiver| formatter.receiver_mode(receiver),
            )?;
            if is_grouped {
                Ok(format!("({text})"))
            } else {
                Ok(text)
            }
        })
    }
}
