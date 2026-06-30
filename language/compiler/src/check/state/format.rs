use destack_dir as dir;
use destack_source::ModuleId;
use std::collections::BTreeMap;
use std::path::Path;

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

    /// Format one type relative to a source module.
    pub(in crate::check) fn format_type_at(
        &self,
        module: ModuleId,
        id: dir::GlobalTypeId,
    ) -> String {
        self.format_depth_at(Some(module), id, FORMAT_DEPTH)
            .unwrap_or_else(|_| "<error>".to_string())
    }

    /// Format one type up to a nesting depth.
    fn format_depth(&self, id: dir::GlobalTypeId, depth: usize) -> CompilerResult<String> {
        self.format_depth_at(None, id, depth)
    }

    /// Format one type relative to an optional source module.
    fn format_depth_at(
        &self,
        module: Option<ModuleId>,
        id: dir::GlobalTypeId,
        depth: usize,
    ) -> CompilerResult<String> {
        if depth == 0 {
            return Ok("…".to_string());
        }
        let id = self.settled_root(id)?;
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
            dir::Type::Reference(reference) => {
                self.format_symbol_path_maybe_at(module, reference.symbol)
            }
            dir::Type::Instance(instance) => {
                let name = self.format_symbol_path_maybe_at(module, instance.symbol);
                if instance.arguments.is_empty() {
                    name
                } else {
                    let arguments = self.format_list_at(module, &instance.arguments, next)?;

                    format!("{name}<{arguments}>")
                }
            }
            dir::Type::Member(member) => {
                let owner = self.format_depth_at(module, member.owner, next)?;
                let key = self.format_static_key(&member.key);

                format!("{owner}.{key}")
            }
            dir::Type::EnumMember(member) => {
                self.format_symbol_path_maybe_at(module, member.member)
            }

            // intrinsic collections render their declared names
            dir::Type::Array(array) => {
                format!(
                    "Array<{}>",
                    self.format_depth_at(module, array.element, next)?
                )
            }
            dir::Type::Slice(slice) => {
                format!(
                    "Slice<{}>",
                    self.format_depth_at(module, slice.element, next)?
                )
            }
            dir::Type::FixedArray(array) => {
                let element = self.format_depth_at(module, array.element, next)?;
                let count = self.format_depth_at(module, array.count, next)?;

                format!("FixedArray<{element}, {count}>")
            }
            dir::Type::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|element| element.ty)
                    .collect::<Vec<_>>();

                format!("({})", self.format_list_at(module, &elements, next)?)
            }

            dir::Type::Shape(shape) => {
                let mut fields = Vec::new();
                for field in shape.fields.iter().take(FORMAT_WIDTH) {
                    let key = self.format_type_field_key(&field.key);
                    let optional = if field.is_optional { "?" } else { "" };
                    let ty = self.format_depth_at(module, field.ty, next)?;

                    fields.push(format!("{key}{optional}: {ty}"));
                }

                for signature in shape
                    .index_signatures
                    .iter()
                    .take(FORMAT_WIDTH - fields.len())
                {
                    let readonly = if signature.is_readonly {
                        "readonly "
                    } else {
                        ""
                    };
                    let optional = if signature.is_optional { "?" } else { "" };
                    let name = self.text(signature.name);
                    let key = self.format_depth_at(module, signature.key_type, next)?;
                    let value = self.format_depth_at(module, signature.value_type, next)?;

                    fields.push(format!("{readonly}[{name}: {key}]{optional}: {value}"));
                }

                let field_count = shape.fields.len() + shape.index_signatures.len();
                if field_count > FORMAT_WIDTH {
                    fields.push("…".to_string());
                }

                if fields.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{ {} }}", fields.join("; "))
                }
            }
            dir::Type::FunctionSignature(function) => {
                let mut parameters = Vec::new();
                for parameter in function.parameters.iter().take(FORMAT_WIDTH) {
                    parameters.push(self.format_function_parameter_at(module, parameter, next)?);
                }
                if function.parameters.len() > FORMAT_WIDTH {
                    parameters.push("…".to_string());
                }
                let result = match function.return_type {
                    Some(return_type) => self.format_depth_at(module, return_type, next)?,
                    None => "void".to_string(),
                };

                format!("({}) => {result}", parameters.join(", "))
            }
            dir::Type::Function(function) => {
                self.format_depth_at(module, function.signature, next)?
            }
            dir::Type::FunctionPointer(function) => {
                self.format_function_pointer_at(module, function, next)?
            }

            dir::Type::Union(union) => {
                let mut elements = Vec::new();
                for element in union.elements.iter().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth_at(module, *element, next)?);
                }
                if union.elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" | ")
            }
            dir::Type::Intersection(intersection) => {
                let mut elements = Vec::new();
                for element in intersection.elements.iter().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth_at(module, *element, next)?);
                }
                if intersection.elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" & ")
            }

            dir::Type::Form(form) => self.format_form_at(module, form, next)?,
            dir::Type::Dynamic(dynamic) => {
                format!(
                    "Dynamic<{}>",
                    self.format_depth_at(module, dynamic.constraint, next)?
                )
            }

            dir::Type::Operation(operation) => self.format_operation_at(module, operation, next)?,
        };

        Ok(rendered)
    }

    /// Format one type list relative to an optional source module.
    fn format_list_at(
        &self,
        module: Option<ModuleId>,
        ids: &[dir::GlobalTypeId],
        depth: usize,
    ) -> CompilerResult<String> {
        let mut formatted = Vec::new();
        for id in ids.iter().take(FORMAT_WIDTH) {
            formatted.push(self.format_depth_at(module, *id, depth)?);
        }
        if ids.len() > FORMAT_WIDTH {
            formatted.push("…".to_string());
        }

        Ok(formatted.join(", "))
    }

    /// Format one function pointer type relative to an optional source module.
    fn format_function_pointer_at(
        &self,
        module: Option<ModuleId>,
        function: &dir::FunctionPointerType,
        depth: usize,
    ) -> CompilerResult<String> {
        let signature = self.settled_root(function.signature)?;
        let dir::Type::FunctionSignature(signature) = self.ty(signature)? else {
            let signature = self.format_depth_at(module, function.signature, depth)?;

            return Ok(format!("FunctionPointer<{signature}>"));
        };

        let mut parameters = Vec::new();
        for parameter in signature.parameters.iter().take(FORMAT_WIDTH) {
            let parameter = self.format_function_parameter_at(module, parameter, depth)?;

            parameters.push(parameter);
        }
        if signature.parameters.len() > FORMAT_WIDTH {
            parameters.push("…".to_string());
        }

        let parameters = match parameters.as_slice() {
            [] => "()".to_string(),
            [parameter] => format!("({parameter},)"),
            _ => format!("({})", parameters.join(", ")),
        };
        let result = match signature.return_type {
            Some(return_type) => self.format_depth_at(module, return_type, depth)?,
            None => "void".to_string(),
        };

        Ok(format!("FunctionPointer<{parameters}, {result}>"))
    }

    /// Format one function signature parameter relative to an optional source module.
    fn format_function_parameter_at(
        &self,
        module: Option<ModuleId>,
        parameter: &dir::FunctionParameterType,
        depth: usize,
    ) -> CompilerResult<String> {
        let parameter_type = self.format_depth_at(module, parameter.ty, depth)?;
        let parameter_type = if parameter.is_rest {
            format!("...{parameter_type}")
        } else {
            parameter_type
        };

        Ok(parameter_type)
    }

    /// Format one memory form relative to an optional source module.
    fn format_form_at(
        &self,
        module: Option<ModuleId>,
        form: &dir::FormType,
        depth: usize,
    ) -> CompilerResult<String> {
        let value = self.format_depth_at(module, form.value, depth)?;

        let rendered = match &form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
            dir::Form::Borrowed { access, .. } => {
                let access = match self.ty(self.settled_root(*access)?)? {
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
                let place = match self.ty(self.settled_root(*place)?)? {
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

    /// Format one type operation relative to an optional source module.
    fn format_operation_at(
        &self,
        module: Option<ModuleId>,
        operation: &dir::TypeOperation,
        depth: usize,
    ) -> CompilerResult<String> {
        let rendered = match operation {
            dir::TypeOperation::Conditional(conditional) => format!(
                "{} extends {} ? {} : {}",
                self.format_depth_at(module, conditional.left, depth)?,
                self.format_depth_at(module, conditional.right, depth)?,
                self.format_depth_at(module, conditional.then_type, depth)?,
                self.format_depth_at(module, conditional.else_type, depth)?,
            ),
            dir::TypeOperation::Narrow(narrow) => {
                let source = self.format_depth_at(module, narrow.source, depth)?;
                let target = self.format_depth_at(module, narrow.target, depth)?;
                if narrow.is_positive {
                    format!("Narrow<{source}, {target}>")
                } else {
                    format!("Narrow<{source}, !{target}>")
                }
            }
            dir::TypeOperation::KeyOf(unary) => {
                format!(
                    "keyof {}",
                    self.format_depth_at(module, unary.target, depth)?
                )
            }
            dir::TypeOperation::NoInfer(unary) => {
                format!(
                    "NoInfer<{}>",
                    self.format_depth_at(module, unary.target, depth)?
                )
            }
            dir::TypeOperation::Awaited(unary) => {
                format!(
                    "Awaited<{}>",
                    self.format_depth_at(module, unary.target, depth)?
                )
            }
            dir::TypeOperation::Index(index) => format!(
                "{}[{}]",
                self.format_depth_at(module, index.left, depth)?,
                self.format_depth_at(module, index.index, depth)?,
            ),
            dir::TypeOperation::StaticBinary(binary) => format!(
                "{} {} {}",
                self.format_depth_at(module, binary.left, depth)?,
                format_static_binary_operator(binary.operator),
                self.format_depth_at(module, binary.right, depth)?,
            ),
            dir::TypeOperation::StaticUnary(unary) => {
                let operator = match unary.operator {
                    dir::StaticUnaryOperator::Not => "!",
                    dir::StaticUnaryOperator::Negate => "-",
                    dir::StaticUnaryOperator::BitwiseNot => "~",
                };

                format!(
                    "{operator}{}",
                    self.format_depth_at(module, unary.target, depth)?
                )
            }
            dir::TypeOperation::TryOutput { value } => {
                format!("Output<{}>", self.format_depth_at(module, *value, depth)?)
            }
            dir::TypeOperation::TryResidual { value } => {
                format!("Residual<{}>", self.format_depth_at(module, *value, depth)?)
            }
            dir::TypeOperation::StringMapping { target, .. } => {
                self.format_depth_at(module, *target, depth)?
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

    /// Format the source path written by one assignment.
    pub(in crate::check) fn format_assignment_binding(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> String {
        let Ok(source) = source.try_into_typed::<dir::Expression>() else {
            return self.format_symbol(symbol);
        };

        self.format_assignment_expression(source.module_id, source.local_id)
            .unwrap_or_else(|| self.format_symbol(symbol))
    }

    /// Format one assignment target expression when it is a simple path.
    fn format_assignment_expression(
        &self,
        module: ModuleId,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        match self.module(module).view().get(source) {
            dir::Expression::Identifier { name } => {
                Some(self.format_static_key(&dir::StaticKey::Name(*name)))
            }
            dir::Expression::Member {
                left,
                name: Some(name),
            } => {
                let left = self.format_assignment_expression(module, *left)?;
                let name = self.format_static_key(&dir::StaticKey::Name(*name));

                Some(format!("{left}.{name}"))
            }
            _ => None,
        }
    }

    /// Format one symbol by its owner-qualified declared name.
    pub(in crate::check) fn format_symbol_path(&self, symbol: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table(symbol.module_id);
        let mut paths = BTreeMap::new();

        self.format_symbol_path_base(&bindings, symbol.local_id, &mut paths)
    }

    /// Format one semantic symbol path relative to an optional source module.
    fn format_symbol_path_maybe_at(
        &self,
        module: Option<ModuleId>,
        symbol: dir::GlobalSymbolId,
    ) -> String {
        let path = self.format_symbol_path(symbol);
        if module.is_some_and(|module| module == symbol.module_id) {
            return path;
        }

        match module {
            Some(_) => format!("{}.{}", self.format_module_label(symbol.module_id), path),
            None => path,
        }
    }

    /// Format one local symbol path without duplicate suffixes.
    fn format_symbol_path_base(
        &self,
        bindings: &dir::BindingTable<'_>,
        symbol: dir::LocalSymbolId,
        paths: &mut BTreeMap<dir::LocalSymbolId, String>,
    ) -> String {
        if let Some(path) = paths.get(&symbol) {
            return path.clone();
        }

        let entry = bindings.get_symbol(symbol);
        let label = self.format_symbol(dir::GlobalSymbolId {
            module_id: bindings.module_id,
            local_id: symbol,
        });
        if !Self::should_qualify_symbol(entry) {
            paths.insert(symbol, label.clone());

            return label;
        }

        let scope = bindings.get_scope_by_id(entry.scope.id);
        let Some(owner) = scope.owner else {
            paths.insert(symbol, label.clone());

            return label;
        };

        let owner_symbol = bindings.get_symbol(owner);
        if owner_symbol.role == dir::SymbolRole::Namespace && owner_symbol.name().is_none() {
            paths.insert(symbol, label.clone());

            return label;
        }

        let owner = self.format_symbol_path_base(bindings, owner, paths);
        let path = format!("{owner}.{label}");
        paths.insert(symbol, path.clone());

        path
    }

    /// Return whether one symbol should be owner-qualified.
    fn should_qualify_symbol(symbol: &dir::Symbol) -> bool {
        symbol.role == dir::SymbolRole::Item
            || symbol.role == dir::SymbolRole::Namespace
            || symbol.kind == dir::SymbolKind::TypeAlias
            || symbol.kind == dir::SymbolKind::GenericTypeParameter
            || symbol.kind == dir::SymbolKind::GenericValueParameter
    }

    /// Format one module as a compact qualifier.
    fn format_module_label(&self, module: ModuleId) -> String {
        if let Some(module) = self.modules.get(&module) {
            return trim_module_uri(module.module.uri.as_ref());
        }

        if let Ok(Some(module)) = self
            .compiler
            .repository
            .module(self.context.revision(), module)
        {
            return trim_module_uri(module.uri.as_ref());
        }

        format!("module#{module}")
    }

    /// Format one member or symbol key.
    pub(in crate::check) fn format_static_key(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.text(*name),
            dir::StaticKey::Index(index) => index.to_string(),
            // a unique symbol shows its declaring binding's name
            dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)) => self.format_symbol(*symbol),
            dir::StaticKey::Symbol(dir::SymbolKey::Registry(name)) => {
                format!("Symbol.for(\"{}\")", self.text(*name))
            }
        }
    }

    /// Format one field key as it appears in object type text.
    fn format_type_field_key(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Symbol(_) => format!("[{}]", self.format_static_key(key)),
            _ => self.format_static_key(key),
        }
    }

    /// Format one owner.case variant label.
    pub(in crate::check) fn format_variant_case(
        &self,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> String {
        let owner = self.format_type(owner);
        let key = self.format_static_key(&key);

        format!("{owner}.{key}")
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

/// Trim one module URI to a compact label.
fn trim_module_uri(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix("file://") {
        return Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_string();
    }

    let uri = uri.strip_prefix("destack://").unwrap_or(uri);
    let uri = uri.strip_suffix(".ds").unwrap_or(uri);
    let uri = uri.trim_start_matches("./");
    let uri = uri.trim_start_matches(['/', '\\']);
    let uri = uri.replace(['/', '\\'], ".");

    uri.trim_matches('.').to_string()
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
