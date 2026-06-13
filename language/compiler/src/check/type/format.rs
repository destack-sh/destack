use destack_dir as dir;

use crate::CompilerResult;
use crate::check::CheckState;

/// Nesting depth after which formatted types elide their details.
const FORMAT_DEPTH: usize = 4;

/// Element count after which formatted lists elide their tails.
const FORMAT_WIDTH: usize = 4;

impl CheckState<'_> {
    /// Format one type for diagnostics.
    pub(in crate::check) fn format_type(&self, id: dir::GlobalTypeId) -> String {
        self.format_depth(id, FORMAT_DEPTH)
            .unwrap_or_else(|_| "<error>".to_string())
    }

    /// Format one type up to a nesting depth.
    fn format_depth(&self, id: dir::GlobalTypeId, depth: usize) -> CompilerResult<String> {
        if depth == 0 {
            return Ok("…".to_string());
        }
        let id = self.resolve_root(id)?;
        let next = depth - 1;

        let rendered = match self.ty(id)? {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Never => "never".to_string(),
            dir::Type::Any => "any".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Object => "object".to_string(),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::This => "this".to_string(),
            dir::Type::Variable(_) => "_".to_string(),

            dir::Type::Primitive(primitive) => format_primitive(primitive),
            dir::Type::Literal(literal) => self.format_scalar_literal(literal),
            dir::Type::Memory(literal) => {
                format!("\"{}\"", literal.text())
            }
            dir::Type::Static(value) => self.format_static(*value),
            dir::Type::Range(range) => self.format_range(range),

            dir::Type::Parameter(parameter) => self.format_parameter(*parameter),
            dir::Type::Reference(instance) => {
                let name = self.format_symbol(instance.symbol);
                if instance.arguments.is_empty() {
                    name
                } else {
                    let arguments = self.format_list(&instance.arguments, next)?;

                    format!("{name}<{arguments}>")
                }
            }
            dir::Type::Member(member) => {
                let owner = self.format_depth(member.owner, next)?;
                let key = self.format_static_key(&member.key);

                format!("{owner}.{key}")
            }

            // intrinsic collections render their declared names
            dir::Type::Array(array) => {
                format!("Array<{}>", self.format_depth(array.element, next)?)
            }
            dir::Type::Slice(slice) => {
                format!("Slice<{}>", self.format_depth(slice.element, next)?)
            }
            dir::Type::FixedArray(array) => {
                let element = self.format_depth(array.element, next)?;
                let count = self.format_depth(array.count, next)?;

                format!("FixedArray<{element}, {count}>")
            }
            dir::Type::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|element| element.ty)
                    .collect::<Vec<_>>();

                format!("({})", self.format_list(&elements, next)?)
            }

            dir::Type::Shape(shape) => {
                let mut fields = Vec::new();
                for field in shape.fields.iter().take(FORMAT_WIDTH) {
                    let key = self.format_static_key(&field.key);
                    let optional = if field.is_optional { "?" } else { "" };
                    let ty = self.format_depth(field.ty, next)?;

                    fields.push(format!("{key}{optional}: {ty}"));
                }
                if shape.fields.len() > FORMAT_WIDTH {
                    fields.push("…".to_string());
                }

                format!("{{ {} }}", fields.join("; "))
            }
            dir::Type::Function(function) => {
                let mut parameters = Vec::new();
                for parameter in function.parameters.iter().take(FORMAT_WIDTH) {
                    parameters.push(self.format_depth(parameter.ty, next)?);
                }
                if function.parameters.len() > FORMAT_WIDTH {
                    parameters.push("…".to_string());
                }
                let result = match function.return_type {
                    Some(return_type) => self.format_depth(return_type, next)?,
                    None => "void".to_string(),
                };

                format!("({}) => {result}", parameters.join(", "))
            }
            dir::Type::Closure(closure) => self.format_depth(closure.function, next)?,

            dir::Type::Union(union) => {
                let mut elements = Vec::new();
                for element in union.elements.iter().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth(*element, next)?);
                }
                if union.elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" | ")
            }
            dir::Type::Intersection(intersection) => {
                let mut elements = Vec::new();
                for element in intersection.elements.iter().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth(*element, next)?);
                }
                if intersection.elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" & ")
            }

            dir::Type::Form(form) => self.format_form(form, next)?,
            dir::Type::Dynamic(dynamic) => {
                format!("Dynamic<{}>", self.format_depth(dynamic.constraint, next)?)
            }

            dir::Type::Operation(operation) => self.format_operation(operation, next)?,
        };

        Ok(rendered)
    }

    /// Format one type list up to a nesting depth.
    fn format_list(&self, ids: &[dir::GlobalTypeId], depth: usize) -> CompilerResult<String> {
        let mut formatted = Vec::new();
        for id in ids.iter().take(FORMAT_WIDTH) {
            formatted.push(self.format_depth(*id, depth)?);
        }
        if ids.len() > FORMAT_WIDTH {
            formatted.push("…".to_string());
        }

        Ok(formatted.join(", "))
    }

    /// Format one memory form with its written sigil.
    /// The managed default reads transparently as its payload.
    fn format_form(&self, form: &dir::FormType, depth: usize) -> CompilerResult<String> {
        let value = self.format_depth(form.value, depth)?;

        let rendered = match &form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
            dir::Form::Borrowed { access, .. } => {
                let access = match self.ty(self.resolve_root(*access)?)? {
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                        "&readonly "
                    }
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                        "&exclusive "
                    }
                    _ => "&",
                };

                format!("{access}{value}")
            }
            dir::Form::Placed { place } => {
                let place = match self.ty(self.resolve_root(*place)?)? {
                    dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))) => {
                        match space {
                            dir::Space::Local => "local ",
                            dir::Space::Shared => "shared ",
                            dir::Space::Static => "static ",
                            dir::Space::Frame => "frame ",
                        }
                    }
                    _ => "",
                };

                format!("{place}{value}")
            }
        };

        Ok(rendered)
    }

    /// Format one type operation compactly.
    fn format_operation(
        &self,
        operation: &dir::TypeOperation,
        depth: usize,
    ) -> CompilerResult<String> {
        let rendered = match operation {
            dir::TypeOperation::Conditional(conditional) => format!(
                "{} extends {} ? {} : {}",
                self.format_depth(conditional.left, depth)?,
                self.format_depth(conditional.right, depth)?,
                self.format_depth(conditional.then_type, depth)?,
                self.format_depth(conditional.else_type, depth)?,
            ),
            dir::TypeOperation::KeyOf(unary) => {
                format!("keyof {}", self.format_depth(unary.target, depth)?)
            }
            dir::TypeOperation::Index(index) => format!(
                "{}[{}]",
                self.format_depth(index.left, depth)?,
                self.format_depth(index.index, depth)?,
            ),
            dir::TypeOperation::StaticBinary(binary) => format!(
                "{} {} {}",
                self.format_depth(binary.left, depth)?,
                format_static_binary_operator(binary.operator),
                self.format_depth(binary.right, depth)?,
            ),
            dir::TypeOperation::StaticUnary(unary) => {
                let operator = match unary.operator {
                    dir::StaticUnaryOperator::Not => "!",
                    dir::StaticUnaryOperator::Negate => "-",
                    dir::StaticUnaryOperator::BitwiseNot => "~",
                };

                format!("{operator}{}", self.format_depth(unary.target, depth)?)
            }
            dir::TypeOperation::TryOutput { value } => {
                format!("Output<{}>", self.format_depth(*value, depth)?)
            }
            dir::TypeOperation::TryResidual { value } => {
                format!("Residual<{}>", self.format_depth(*value, depth)?)
            }
            dir::TypeOperation::StringMapping { target, .. } => {
                self.format_depth(*target, depth)?
            }
            dir::TypeOperation::Mapped(_) => "{ [mapped] }".to_string(),
            dir::TypeOperation::TemplateLiteral(_) => "`…`".to_string(),
            dir::TypeOperation::Infer(infer) => match infer.name {
                Some(name) => format!("infer {}", self.text(name)),
                None => "infer _".to_string(),
            },
        };

        Ok(rendered)
    }

    /// Format one scalar literal type.
    fn format_scalar_literal(&self, literal: &dir::ScalarLiteral) -> String {
        match literal {
            dir::ScalarLiteral::String(value) => format!("\"{}\"", self.text(*value)),
            dir::ScalarLiteral::Character(value) => format!("'{value}'"),
            dir::ScalarLiteral::Boolean(value) => value.to_string(),
            dir::ScalarLiteral::Integer(value) => value.to_string(),
            dir::ScalarLiteral::Float(value) => value.to_string(),
            dir::ScalarLiteral::Bigint(value) => format!("{value}n"),
            dir::ScalarLiteral::Null => "null".to_string(),
            dir::ScalarLiteral::Undefined => "undefined".to_string(),
            dir::ScalarLiteral::RegexString { .. } => "regex".to_string(),
        }
    }

    /// Format one interval type.
    fn format_range(&self, range: &dir::RangeType) -> String {
        let start = range
            .start
            .as_ref()
            .map(|literal| self.format_scalar_literal(literal))
            .unwrap_or_default();
        let end = range
            .end
            .as_ref()
            .map(|literal| self.format_scalar_literal(literal))
            .unwrap_or_default();
        let operator = if range.is_inclusive { "..=" } else { ".." };

        format!("{start}{operator}{end}")
    }

    /// Format one committed static value.
    fn format_static(&self, value: dir::GlobalStaticId) -> String {
        match self.r#static(value) {
            dir::StaticTerm::ScalarLiteral { value } => self.format_scalar_literal(value),
            _ => "static".to_string(),
        }
    }

    /// Format one generic parameter by its declared name.
    fn format_parameter(&self, parameter: dir::GlobalGenericParameterId) -> String {
        let Some(binding) = self.generic_parameter(parameter) else {
            return "_".to_string();
        };

        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => self.format_symbol(symbol),
            dir::GenericParameterKey::Generated(name) => self.text(name),
        }
    }

    /// Format one type list for diagnostics.
    pub(in crate::check) fn format_types(&self, ids: &[dir::GlobalTypeId]) -> String {
        if ids.is_empty() {
            return "none".to_string();
        }

        ids.iter()
            .map(|id| format!("'{}'", self.format_type(*id)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Format one symbol by its declared name.
    pub(in crate::check) fn format_symbol(&self, symbol: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table(symbol.module_id);
        let key = bindings.get_symbol(symbol.local_id).key;

        match key {
            Some(key) => self.format_static_key(&key),
            None => "<anonymous>".to_string(),
        }
    }

    /// Format one member or symbol key.
    pub(in crate::check) fn format_static_key(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.text(*name),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(_) => "[symbol]".to_string(), // NOTE #Suspicious: format_static_key?
        }
    }

    /// Return interned text from any loaded module pool.
    fn text(&self, id: dir::StringId) -> String {
        // search component pools first
        for module in self.modules.values() {
            if let Some(text) = module.strings.get_maybe(id) {
                return text.to_string();
            }
        }

        "<string>".to_string()
    }
}

/// Format one primitive type.
fn format_primitive(primitive: &dir::PrimitiveType) -> String {
    match primitive {
        dir::PrimitiveType::Boolean => "boolean".to_string(),
        dir::PrimitiveType::String => "string".to_string(),
        dir::PrimitiveType::Character => "char".to_string(),
        dir::PrimitiveType::Bigint => "bigint".to_string(),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
        dir::PrimitiveType::Integer(integer) => match integer {
            dir::IntegerType::Integer { is_signed: true } => "int".to_string(),
            dir::IntegerType::Integer { is_signed: false } => "uint".to_string(),
            dir::IntegerType::Pointer { is_signed: true } => "isize".to_string(),
            dir::IntegerType::Pointer { is_signed: false } => "usize".to_string(),
            dir::IntegerType::Fixed { width, is_signed } => {
                let sign = if *is_signed { "int" } else { "uint" };

                format!("{sign}{width}")
            }
        },
        dir::PrimitiveType::Float(float) => match float {
            dir::FloatType::Float16 => "float16".to_string(),
            dir::FloatType::Bfloat16 => "bfloat16".to_string(),
            dir::FloatType::Float32 => "float32".to_string(),
            dir::FloatType::Float64 => "float64".to_string(),
            dir::FloatType::Float => "number".to_string(),
        },
    }
}

/// Format one static binary operator.
fn format_static_binary_operator(operator: dir::StaticBinaryOperator) -> &'static str {
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
