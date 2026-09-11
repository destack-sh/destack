use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

/// One syntactic position that may require a grouped type operand.
#[derive(Debug, Clone, Copy)]
pub(super) enum TypeOperand {
    /// A prefix type operator operand.
    Prefix,
    /// A postfix type operator or member receiver.
    Postfix,
    /// One union member.
    Union,
    /// One intersection member.
    Intersection,
    /// One relational type operand.
    Relation,
    /// One static binary operation operand.
    StaticBinary,
}

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
                let base = self.type_operand(refined.base, TypeOperand::Postfix)?;
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
            dir::Type::Function(function) => return self.global_type(function.signature),
            dir::Type::FunctionPointer(function) => {
                return self.global_type(function.signature);
            }
            dir::Type::Union(union) => {
                return self.type_list(union.elements, " | ", TypeOperand::Union);
            }
            dir::Type::Intersection(intersection) => {
                return self.type_list(intersection.elements, " & ", TypeOperand::Intersection);
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
            let element = self.type_operand(*element, TypeOperand::Postfix)?;

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
        let owner = self.type_operand(member.owner, TypeOperand::Postfix)?;
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
        let value = self.type_operand(form.value, TypeOperand::Prefix)?;
        let text = match form.form {
            dir::Form::Managed { place } => self.placed_form(place, &value)?,
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

        // read the region extent and space
        let (extent, space) = self.read_type(borrow.region, |type_value, _| match type_value {
            dir::Type::Region(pair) => Ok((pair.extent, Some(pair.space))),
            _ => Ok((borrow.region, None)),
        })?;
        // render literal referent spaces in target position, keeping written local
        let target = match space {
            None => value.to_string(),
            Some(space) => match self.space_literal(space)? {
                Some(space) => format!("{} {value}", space.text()),
                // induced spaces elide back into the reference sugar
                None if self.is_induced_memory_term(space)? => value.to_string(),
                // render written parametric spaces through the full borrow application
                None => return self.borrow_application(&borrow, value),
            },
        };

        // written non-tick extents render the full borrow application
        let Some(lifetime) = self.borrow_extent_prefix(extent)? else {
            return self.borrow_application(&borrow, value);
        };

        // render concrete qualifiers directly and retain generic arguments in Borrowed
        let access = self.read_type(borrow.access, |ty, _| {
            Ok(match ty {
                dir::Type::Literal(dir::Literal::String(text)) => dir::Access::from_text(*text),
                _ => None,
            })
        })?;
        let exclusivity = self.read_type(borrow.exclusivity, |ty, _| {
            Ok(match ty {
                dir::Type::Literal(dir::Literal::String(text)) => {
                    dir::Exclusivity::from_text(*text)
                }
                _ => None,
            })
        })?;
        let (Some(access), Some(exclusivity)) = (access, exclusivity) else {
            return self.borrow_application(&borrow, value);
        };
        let access = match access {
            dir::Access::Mutable => "",
            dir::Access::Readonly => "readonly ",
        };
        let exclusivity = match exclusivity {
            dir::Exclusivity::Aliasable => "",
            dir::Exclusivity::Exclusive => "exclusive ",
        };

        Ok(format!("&{lifetime}{access}{exclusivity}{target}"))
    }

    /// Format one full borrow application.
    fn borrow_application(&self, borrow: &dir::BorrowForm, value: &str) -> QueryResult<String> {
        let region = self.global_type(borrow.region)?;
        let access = self.global_type(borrow.access)?;
        let exclusivity = self.global_type(borrow.exclusivity)?;
        let symbol = self
            .program
            .environment_bound()?
            .language
            .symbol(dir::LanguageItem::Borrowed)
            .ok_or(QueryError::missing("Borrowed language item"))?;
        let borrowed = self.symbol(symbol)?;

        Ok(format!(
            "{borrowed}<{value}, {region}, {access}, {exclusivity}>"
        ))
    }

    /// Return whether one term is an induced memory parameter.
    fn is_induced_memory_term(&self, type_id: dir::GlobalTypeId) -> QueryResult<bool> {
        self.program
            .read_type(type_id, |type_value, module| match type_value {
                dir::Type::Parameter(parameter) => Ok(module
                    .generics()?
                    .get_parameter(parameter.local_id)
                    .induced_memory_parameter()
                    .is_some()),
                _ => Ok(false),
            })
    }

    /// Read one literal space, or None for a parametric place.
    fn space_literal(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<dir::Space>> {
        self.program
            .read_type(type_id, |type_value, _| match type_value {
                dir::Type::Literal(dir::Literal::String(text)) => dir::Space::from_text(*text)
                    .map(Some)
                    .ok_or_else(|| QueryError::invalid(format!("space literal: {type_id:?}"))),
                _ => Ok(None),
            })
    }

    /// Return the source prefix for one directly representable borrow extent.
    fn borrow_extent_prefix(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<String>> {
        self.program
            .read_type(type_id, |type_value, module| match type_value {
                dir::Type::Literal(dir::Literal::String(value))
                    if dir::Lifetime::parse(module.strings().get(*value))
                        == Some(dir::Lifetime::Static) =>
                {
                    Ok(Some("'static ".to_string()))
                }
                // ticks render as prefixes, induced extents elide, written names defer
                dir::Type::Parameter(parameter) => {
                    let formatter = Formatter::new(module, self.program);
                    let lifetime = formatter.generic_parameter_type(*parameter)?;
                    if lifetime.starts_with('\'') {
                        return Ok(Some(format!("{lifetime} ")));
                    }

                    let binding = module.generics()?.get_parameter(parameter.local_id);
                    Ok(binding
                        .induced_memory_parameter()
                        .is_some()
                        .then(String::new))
                }
                // every other extent erases from the reference prefix
                _ => Ok(Some(String::new())),
            })
    }

    /// Format one memory form with its explicit parameter.
    fn memory_application(
        &self,
        item: dir::LanguageItem,
        value: &str,
        argument: &str,
    ) -> QueryResult<String> {
        let symbol = self
            .program
            .environment_bound()?
            .language
            .symbol(item)
            .ok_or(QueryError::missing(format!("{item:?} language item")))?;
        let name = self.symbol(symbol)?;

        Ok(format!("{name}<{value}, {argument}>"))
    }

    /// Format one placement form.
    fn placed_form(&self, type_id: dir::GlobalTypeId, value: &str) -> QueryResult<String> {
        let place = self.global_type(type_id)?;

        self.memory_application(dir::LanguageItem::Managed, value, &place)
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
        for signature in self
            .module
            .types()?
            .index_signatures(shape.index_signatures)
        {
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
        operand: TypeOperand,
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
        operand: TypeOperand,
    ) -> QueryResult<String> {
        self.read_type(type_id, |type_value, formatter| {
            let text = formatter.local_type(type_value)?;
            if type_needs_parentheses(type_value, operand, formatter)? {
                Ok(format!("({text})"))
            } else {
                Ok(text)
            }
        })
    }
}

/// Return whether one type needs grouping in its parent position.
fn type_needs_parentheses(
    type_value: &dir::Type,
    operand: TypeOperand,
    formatter: &Formatter<'_, '_, '_>,
) -> QueryResult<bool> {
    let needs_parentheses = match type_value {
        dir::Type::Region(_) => matches!(
            operand,
            TypeOperand::Prefix
                | TypeOperand::Postfix
                | TypeOperand::Intersection
                | TypeOperand::Relation
                | TypeOperand::StaticBinary
        ),
        dir::Type::Union(_) => matches!(
            operand,
            TypeOperand::Prefix
                | TypeOperand::Postfix
                | TypeOperand::Intersection
                | TypeOperand::Relation
                | TypeOperand::StaticBinary
        ),
        dir::Type::Intersection(_) => matches!(
            operand,
            TypeOperand::Prefix
                | TypeOperand::Postfix
                | TypeOperand::Relation
                | TypeOperand::StaticBinary
        ),
        dir::Type::FunctionSignature(_)
        | dir::Type::Function(_)
        | dir::Type::FunctionPointer(_) => true,
        dir::Type::Form(_) => matches!(operand, TypeOperand::Postfix),
        dir::Type::Range(_) => matches!(
            operand,
            TypeOperand::Prefix
                | TypeOperand::Postfix
                | TypeOperand::Relation
                | TypeOperand::StaticBinary
        ),
        dir::Type::Operation(operation) => {
            let operation = formatter.types()?.operation(*operation);
            operation_needs_parentheses(operation, operand)
        }
        _ => false,
    };

    Ok(needs_parentheses)
}

/// Return whether one type operation needs grouping in its parent position.
fn operation_needs_parentheses(operation: &dir::TypeOperation, operand: TypeOperand) -> bool {
    match operation {
        dir::TypeOperation::Conditional(_) | dir::TypeOperation::StaticBinary(_) => true,
        dir::TypeOperation::Mapped(_) => matches!(operand, TypeOperand::Prefix),
        dir::TypeOperation::Infer(infer) if infer.constraint.is_some() => {
            !matches!(operand, TypeOperand::Relation | TypeOperand::StaticBinary)
        }
        dir::TypeOperation::KeyOf(_)
        | dir::TypeOperation::TypeOf(_)
        | dir::TypeOperation::StaticUnary(_) => matches!(operand, TypeOperand::Postfix),
        _ => false,
    }
}
