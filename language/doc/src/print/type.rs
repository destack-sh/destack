use destack_dir as dir;

use crate::{DocError, DocResult};

use super::Printer;
use super::literal::quote_string;

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

/// One borrow lifetime's display form.
enum BorrowLifetime {
    /// Lifetime written directly in the borrow prefix.
    Prefix(String),
    /// Const lifetime parameter applied through `WithLifetime`.
    Parameter(String),
}

impl Printer<'_, '_, '_> {
    /// Format one authored type expression.
    pub(super) fn type_expression(
        &self,
        type_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> DocResult<String> {
        let view = self.module.view();
        let span = self.module.node_span(view, type_id.into())?;

        self.module.source_text(span)
    }

    /// Format one node type.
    pub(crate) fn node_type(&self, node_id: dir::GlobalNodeIdAny) -> DocResult<String> {
        let Some(type_id) = self.types().get_node_type_id(node_id) else {
            return Err(DocError::missing(format!("node type: {node_id:?}")));
        };

        self.global_type(type_id)
    }

    /// Format one global type id.
    pub(crate) fn global_type(&self, type_id: dir::GlobalTypeId) -> DocResult<String> {
        self.read_type(type_id, |type_value, formatter| {
            formatter.local_type(type_value)
        })
    }

    /// Format one type owned by this printer's module.
    pub(super) fn local_type(&self, type_value: &dir::Type) -> DocResult<String> {
        let text = match type_value {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Hole(hole) => format!("?{hole}"),
            dir::Type::Rigid(rigid) => format!("^{rigid}"),
            dir::Type::Never => "never".to_string(),
            dir::Type::Any => "any".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Object(shape) => return self.object(*shape),
            dir::Type::Primitive(primitive) => self.primitive(*primitive),
            dir::Type::Literal(literal) => self.literal(*literal),
            dir::Type::Reference(reference) => return self.symbol(reference.symbol),
            dir::Type::Application(instance) => return self.instance(*instance),
            dir::Type::Parameter(parameter) => return self.generic_parameter_type(*parameter),
            dir::Type::Erased(_) => "*".to_string(),
            dir::Type::Member(member) => return self.member(*self.types().member(*member)),
            dir::Type::Refined(refined) => {
                let refined = *self.types().refined(*refined);
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
                return self.function_type(self.types().signature(*function), None);
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
                return self.operation(self.types().operation(*operation));
            }
            dir::Type::Key(key) => self.key_type(*key),
            dir::Type::Memory(literal) => quote_string(literal.text()),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::Variable(_) | dir::Type::Static(_) => {
                return Err(DocError::invalid(format!(
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
    fn instance(&self, instance: dir::GenericApplication) -> DocResult<String> {
        let symbol = self.symbol(instance.symbol)?;
        let arguments = self.types().type_ids(instance.arguments);

        if arguments.is_empty() {
            return Ok(symbol);
        }

        let arguments = self.join_types(arguments, ", ")?;

        Ok(format!("{symbol}<{arguments}>"))
    }

    /// Format one member type.
    fn member(&self, member: dir::MemberType) -> DocResult<String> {
        let owner = self.type_operand(member.owner, TypeOperand::Postfix)?;
        let key = self.member_key(member.key);
        let arguments = self.types().type_ids(member.arguments);

        if arguments.is_empty() {
            return Ok(format!("{owner}{key}"));
        }

        let arguments = self.join_types(arguments, ", ")?;

        Ok(format!("{owner}{key}<{arguments}>"))
    }

    /// Format one canonical memory form.
    pub(super) fn form(&self, form: dir::FormType) -> DocResult<String> {
        let value = self.type_operand(form.value, TypeOperand::Prefix)?;
        let text = match form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed(borrow) => self.borrowed_form(borrow, &value)?,
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Placed { place } => self.placed_form(place, &value)?,
            dir::Form::Readonly => format!("readonly {value}"),
        };

        Ok(text)
    }

    /// Format one borrowed form.
    fn borrowed_form(&self, borrow: dir::BorrowFormId, value: &str) -> DocResult<String> {
        let borrow = *self.types().borrow_form(borrow);
        let lifetime = self.borrow_lifetime(borrow.lifetime)?;

        match lifetime {
            BorrowLifetime::Prefix(lifetime) => self.borrow_access(borrow.access, &lifetime, value),
            BorrowLifetime::Parameter(lifetime) => {
                let borrowed = self.borrow_access(borrow.access, "", value)?;
                let symbol = self
                    .program
                    .environment()
                    .language
                    .symbol(dir::LanguageItem::WithLifetime)
                    .ok_or(DocError::missing("WithLifetime language item"))?;
                let with_lifetime = self.symbol(symbol)?;

                Ok(format!("{with_lifetime}<{borrowed}, {lifetime}>"))
            }
        }
    }

    /// Format one borrow lifetime.
    fn borrow_lifetime(&self, type_id: dir::GlobalTypeId) -> DocResult<BorrowLifetime> {
        self.program
            .read_type(type_id, |type_value, module| match type_value {
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)) => {
                    Ok(BorrowLifetime::Prefix(String::new()))
                }
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Static)) => {
                    Ok(BorrowLifetime::Prefix("'static ".to_string()))
                }
                dir::Type::Parameter(parameter) => {
                    let formatter = Printer::new(module, self.program);
                    let binding = module.generics().get_parameter(parameter.local_id);
                    if binding.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                        return Err(DocError::invalid(format!("borrow lifetime: {type_id:?}")));
                    }
                    let lifetime = formatter.generic_parameter_type(*parameter)?;

                    if lifetime.starts_with('\'') {
                        Ok(BorrowLifetime::Prefix(format!("{lifetime} ")))
                    } else {
                        Ok(BorrowLifetime::Parameter(lifetime))
                    }
                }
                _ => Err(DocError::invalid(format!("borrow lifetime: {type_id:?}"))),
            })
    }

    /// Apply one borrow access to a borrowed type.
    fn borrow_access(
        &self,
        type_id: dir::GlobalTypeId,
        lifetime: &str,
        value: &str,
    ) -> DocResult<String> {
        self.read_type(type_id, |type_value, formatter| {
            match type_value {
                // render concrete access with its source modifier
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Mutable)) => {
                    Ok(format!("&{lifetime}{value}"))
                }
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                    Ok(format!("&{lifetime}readonly {value}"))
                }
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                    Ok(format!("&{lifetime}exclusive {value}"))
                }

                // retain generic access through its canonical language form
                dir::Type::Parameter(parameter) => {
                    let parameter_binding = formatter
                        .module
                        .generics()
                        .get_parameter(parameter.local_id);
                    if parameter_binding.memory_parameter() != Some(dir::MemoryParameter::Access) {
                        return Err(DocError::invalid(format!("borrow access: {type_id:?}")));
                    }

                    let borrowed = format!("&{lifetime}{value}");
                    let access = formatter.generic_parameter_type(*parameter)?;
                    let symbol = formatter
                        .program
                        .environment()
                        .language
                        .symbol(dir::LanguageItem::WithAccess)
                        .ok_or(DocError::missing("WithAccess language item"))?;
                    let with_access = formatter.symbol(symbol)?;

                    Ok(format!("{with_access}<{borrowed}, {access}>"))
                }

                // reject invalid checked borrow access
                _ => Err(DocError::invalid(format!("borrow access: {type_id:?}"))),
            }
        })
    }

    /// Format one concrete placement form.
    fn placed_form(&self, type_id: dir::GlobalTypeId, value: &str) -> DocResult<String> {
        self.program
            .read_type(type_id, |type_value, _| match type_value {
                dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))) => {
                    Ok(format!("{} {value}", space.text()))
                }
                _ => Err(DocError::invalid(format!("placed form: {type_id:?}"))),
            })
    }

    /// Format one scalar interval type.
    fn range(&self, range: dir::RangeType) -> DocResult<String> {
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
    fn tuple(&self, tuple: dir::TupleType) -> DocResult<String> {
        let elements = self.types().elements(tuple.elements);
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
    fn tuple_element(&self, element: &dir::TypeElement) -> DocResult<String> {
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
    fn object(&self, shape: dir::ShapeType) -> DocResult<String> {
        let mut members = Vec::new();

        for property in self.types().properties(shape.properties) {
            members.push(self.property(property)?);
        }
        for signature in self.module.types().index_signatures(shape.index_signatures) {
            members.push(self.index_signature(signature)?);
        }

        if members.is_empty() {
            Ok("{}".to_string())
        } else {
            Ok(format!("{{ {} }}", members.join("; ")))
        }
    }

    /// Format one structural property.
    fn property(&self, property: &dir::TypeProperty) -> DocResult<String> {
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
    fn index_signature(&self, signature: &dir::TypeIndexSignature) -> DocResult<String> {
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
    ) -> DocResult<String> {
        let types = self.types().type_ids(list);
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(self.type_operand(*type_id, operand)?);
        }

        Ok(formatted_types.join(separator))
    }

    /// Format and join global type ids.
    fn join_types(&self, types: &[dir::GlobalTypeId], separator: &str) -> DocResult<String> {
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
    ) -> DocResult<String> {
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
    formatter: &Printer<'_, '_, '_>,
) -> DocResult<bool> {
    let needs_parentheses = match type_value {
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
            let operation = formatter.types().operation(*operation);
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
