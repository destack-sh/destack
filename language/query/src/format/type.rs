use destack_dir as dir;

use crate::{ModuleQueryContext, QueryError, QueryResult};

use super::literal::quote_string;
use super::{Formatter, formatted};

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
    /// Format one node type.
    pub(crate) fn node_type(&self, node_id: dir::GlobalNodeIdAny) -> QueryResult<Option<String>> {
        let Some(type_id) = self.module.types().get_node_type_id(node_id) else {
            return Ok(None);
        };

        self.global_type(type_id)
    }

    /// Format one global type id.
    pub(crate) fn global_type(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<String>> {
        self.module
            .read_global_type(self.program, type_id, |type_value, owner| {
                Formatter::new(owner, self.program).local_type(type_value)
            })?
    }

    /// Format one type owned by this formatter's module.
    pub(super) fn local_type(&self, type_value: &dir::Type) -> QueryResult<Option<String>> {
        let text = match type_value {
            dir::Type::Error => return Ok(None),
            dir::Type::Never => "never".to_string(),
            dir::Type::Any => "any".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Object => "object".to_string(),
            dir::Type::Primitive(primitive) => self.primitive(*primitive),
            dir::Type::Literal(literal) => self.literal(*literal),
            dir::Type::Reference(reference) => return self.symbol(reference.symbol),
            dir::Type::Application(instance) => return self.instance(*instance),
            dir::Type::Parameter(parameter) => return self.generic_parameter_type(*parameter),
            dir::Type::Erased(_) => "*".to_string(),
            dir::Type::Member(member) => return self.member(*self.module.types().member(*member)),
            dir::Type::Refined(refined) => {
                let refined = *self.module.types().refined(*refined);
                let base = formatted!(self.type_operand(refined.base, TypeOperand::Postfix));
                let key = formatted!(self.property_key(refined.key));
                let value = formatted!(self.global_type(refined.value));

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::EnumMember(member) => return self.symbol(member.member),
            dir::Type::Form(form) => return self.form(*form),
            dir::Type::Dynamic(dynamic) => {
                let constraint = formatted!(self.global_type(dynamic.constraint));

                format!("Dynamic<{constraint}>")
            }
            dir::Type::Array(array) => {
                let element = formatted!(self.type_operand(array.element, TypeOperand::Postfix));

                format!("{element}[]")
            }
            dir::Type::FixedArray(array) => {
                let element = formatted!(self.global_type(array.element));
                let count = formatted!(self.global_type(array.count));

                format!("[{element}; {count}]")
            }
            dir::Type::Range(range) => return self.range(*range),
            dir::Type::Slice(slice) => {
                let element = formatted!(self.global_type(slice.element));

                format!("[{element}]")
            }
            dir::Type::Tuple(tuple) => return self.tuple(*tuple),
            dir::Type::Shape(shape) => return self.shape(*shape),
            dir::Type::FunctionSignature(function) => {
                return self.function_type(self.module.types().signature(*function), None);
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
            dir::Type::This => "this".to_string(),
            dir::Type::Operation(operation) => {
                return self.operation(self.module.types().operation(*operation));
            }
            dir::Type::Key(key) => return self.key_type(*key),
            dir::Type::Memory(literal) => quote_string(literal.text()),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::Variable(_) | dir::Type::Static(_) => {
                return Err(QueryError::invalid(format!(
                    "type formatting: {type_value:?}"
                )));
            }
        };

        Ok(Some(text))
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
            dir::PrimitiveType::Symbol => "symbol".to_string(),
            dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
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
    fn instance(&self, instance: dir::GenericApplication) -> QueryResult<Option<String>> {
        let symbol = formatted!(self.symbol(instance.symbol));
        let arguments = self.module.types().type_ids(instance.arguments);

        if arguments.is_empty() {
            return Ok(Some(symbol));
        }

        let arguments = formatted!(self.join_types(arguments, ", "));

        Ok(Some(format!("{symbol}<{arguments}>")))
    }

    /// Format one member type.
    fn member(&self, member: dir::MemberType) -> QueryResult<Option<String>> {
        let owner = formatted!(self.type_operand(member.owner, TypeOperand::Postfix));
        let key = formatted!(self.member_key(member.key));
        let arguments = self.module.types().type_ids(member.arguments);

        if arguments.is_empty() {
            return Ok(Some(format!("{owner}{key}")));
        }

        let arguments = formatted!(self.join_types(arguments, ", "));

        Ok(Some(format!("{owner}{key}<{arguments}>")))
    }

    /// Format one canonical memory form.
    pub(super) fn form(&self, form: dir::FormType) -> QueryResult<Option<String>> {
        let value = formatted!(self.type_operand(form.value, TypeOperand::Prefix));
        let text = match form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed(borrow) => self.borrowed_form(borrow, &value)?,
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Placed { place } => self.placed_form(place, &value)?,
            dir::Form::Readonly => format!("readonly {value}"),
        };

        Ok(Some(text))
    }

    /// Format one borrowed form from its solved lifetime and access.
    fn borrowed_form(&self, borrow: dir::BorrowFormId, value: &str) -> QueryResult<String> {
        let borrow = *self.module.types().borrow_form(borrow);
        let lifetime = self.borrow_lifetime(borrow.lifetime)?;
        let access = self.borrow_access(borrow.access)?;

        Ok(format!("&{lifetime}{access}{value}"))
    }

    /// Format one solved borrow lifetime.
    fn borrow_lifetime(&self, type_id: dir::GlobalTypeId) -> QueryResult<String> {
        self.module.read_global_type(
            self.program,
            type_id,
            |type_value, module| match type_value {
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)) => {
                    Ok(String::new())
                }
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Static)) => {
                    Ok("'static ".to_string())
                }
                dir::Type::Parameter(_) => {
                    let lifetime = Formatter::new(module, self.program)
                        .local_type(type_value)?
                        .ok_or_else(|| {
                            QueryError::invalid(format!("borrow lifetime: {type_id:?}"))
                        })?;
                    let lifetime = lifetime.rsplit('.').next().ok_or_else(|| {
                        QueryError::invalid(format!("borrow lifetime: {type_id:?}"))
                    })?;
                    if !lifetime.starts_with('\'') {
                        return Err(QueryError::invalid(format!("borrow lifetime: {type_id:?}")));
                    }

                    Ok(format!("{lifetime} "))
                }
                _ => Err(QueryError::invalid(format!("borrow lifetime: {type_id:?}"))),
            },
        )?
    }

    /// Format one solved borrow access.
    fn borrow_access(&self, type_id: dir::GlobalTypeId) -> QueryResult<&'static str> {
        self.module
            .read_global_type(self.program, type_id, |type_value, _| match type_value {
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Mutable)) => Ok(""),
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                    Ok("readonly ")
                }
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                    Ok("exclusive ")
                }
                _ => Err(QueryError::invalid(format!("borrow access: {type_id:?}"))),
            })?
    }

    /// Format one concrete placement form.
    fn placed_form(&self, type_id: dir::GlobalTypeId, value: &str) -> QueryResult<String> {
        self.module
            .read_global_type(self.program, type_id, |type_value, _| match type_value {
                dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))) => {
                    Ok(format!("{} {value}", space.text()))
                }
                _ => Err(QueryError::invalid(format!("placed form: {type_id:?}"))),
            })?
    }

    /// Format one scalar interval type.
    fn range(&self, range: dir::RangeType) -> QueryResult<Option<String>> {
        let start = match range.start {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let end = match range.end {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let operator = if range.is_inclusive { "..=" } else { ".." };

        Ok(Some(format!("{start}{operator}{end}")))
    }

    /// Format one tuple type.
    fn tuple(&self, tuple: dir::TupleType) -> QueryResult<Option<String>> {
        let elements = self.module.types().elements(tuple.elements);
        let is_singleton = elements.len() == 1;
        let mut formatted_elements = Vec::with_capacity(elements.len());
        for element in elements {
            formatted_elements.push(formatted!(self.tuple_element(element)));
        }
        let mut elements = formatted_elements.join(", ");
        if is_singleton {
            elements.push(',');
        }

        let text = match tuple.form {
            dir::TupleForm::Tuple => format!("({elements})"),
            dir::TupleForm::Array => format!("[{elements}]"),
        };

        Ok(Some(text))
    }

    /// Format one tuple element.
    fn tuple_element(&self, element: &dir::TypeElement) -> QueryResult<Option<String>> {
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
        let type_text = formatted!(self.global_type(element.ty));
        text.push_str(&type_text);

        Ok(Some(text))
    }

    /// Format one structural shape type.
    fn shape(&self, shape: dir::ShapeType) -> QueryResult<Option<String>> {
        let mut members = Vec::new();

        for field in self.module.types().fields(shape.fields) {
            members.push(formatted!(self.field(field)));
        }
        for signature in self.module.types().index_signatures(shape.index_signatures) {
            members.push(formatted!(self.index_signature(signature)));
        }

        if members.is_empty() {
            Ok(Some("{}".to_string()))
        } else {
            Ok(Some(format!("{{ {} }}", members.join("; "))))
        }
    }

    /// Format one structural field.
    fn field(&self, field: &dir::TypeField) -> QueryResult<Option<String>> {
        let readonly = if field.is_readonly { "readonly " } else { "" };
        let optional = if field.is_optional { "?" } else { "" };
        let key = formatted!(self.property_key(field.key));
        let type_text = formatted!(self.global_type(field.ty));

        Ok(Some(format!("{readonly}{key}{optional}: {type_text}")))
    }

    /// Format one index signature.
    fn index_signature(&self, signature: &dir::TypeIndexSignature) -> QueryResult<Option<String>> {
        let readonly = if signature.is_readonly {
            "readonly "
        } else {
            ""
        };
        let optional = if signature.is_optional { "?" } else { "" };
        let name = self.module.strings().get(signature.name);
        let key_type = formatted!(self.global_type(signature.key_type));
        let value_type = formatted!(self.global_type(signature.value_type));

        Ok(Some(format!(
            "{readonly}[{name}: {key_type}]{optional}: {value_type}"
        )))
    }

    /// Format one list of type operands.
    fn type_list(
        &self,
        list: dir::TypeListId,
        separator: &str,
        operand: TypeOperand,
    ) -> QueryResult<Option<String>> {
        let types = self.module.types().type_ids(list);
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(formatted!(self.type_operand(*type_id, operand)));
        }

        Ok(Some(formatted_types.join(separator)))
    }

    /// Format and join global type ids.
    fn join_types(
        &self,
        types: &[dir::GlobalTypeId],
        separator: &str,
    ) -> QueryResult<Option<String>> {
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(formatted!(self.global_type(*type_id)));
        }

        Ok(Some(formatted_types.join(separator)))
    }

    /// Format one type operand with the grouping required by its parent.
    pub(super) fn type_operand(
        &self,
        type_id: dir::GlobalTypeId,
        operand: TypeOperand,
    ) -> QueryResult<Option<String>> {
        self.module.read_global_type(
            self.program,
            type_id,
            |type_value, module| -> QueryResult<Option<String>> {
                let formatter = Formatter::new(module, self.program);
                let Some(text) = formatter.local_type(type_value)? else {
                    return Ok(None);
                };
                if type_needs_parentheses(type_value, operand, module) {
                    Ok(Some(format!("({text})")))
                } else {
                    Ok(Some(text))
                }
            },
        )?
    }
}

/// Return whether one type needs grouping in its parent position.
fn type_needs_parentheses(
    type_value: &dir::Type,
    operand: TypeOperand,
    module: &ModuleQueryContext<'_>,
) -> bool {
    match type_value {
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
            let operation = module.types().operation(*operation);
            operation_needs_parentheses(operation, operand)
        }
        _ => false,
    }
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
