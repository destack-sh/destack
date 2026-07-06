use std::collections::HashSet;

use destack_core::{StringId, StringPool};
use destack_dir as dir;
use destack_repository::{Module, Package};

use crate::ModuleQueryContext;

/// Formatter for checked DIR types.
struct TypeFormatter<'module, 'query> {
    /// The module that owns list ids read by this formatter.
    module: &'module ModuleQueryContext<'query>,
}

impl<'module, 'query> TypeFormatter<'module, 'query> {
    /// Create a formatter for one checked module.
    fn new(module: &'module ModuleQueryContext<'query>) -> Self {
        Self { module }
    }

    /// Format one global type id.
    fn global(&self, ty_id: dir::GlobalTypeId) -> Option<String> {
        self.module
            .read_global_type(ty_id, |ty, owner| TypeFormatter::new(owner).ty(ty))
    }

    /// Format one checked type owned by this formatter's module.
    fn ty(&self, ty: &dir::Type) -> Option<String> {
        let text = match ty {
            dir::Type::Error => panic!("error type reached type formatting"),
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
            dir::Type::Instance(instance) => return self.instance(*instance),
            dir::Type::Parameter(parameter) => return self.generic_parameter(*parameter),
            dir::Type::Erased(_) => "*".to_string(),
            dir::Type::Member(member) => return self.member(*member),
            dir::Type::EnumMember(member) => return self.symbol(member.member),
            dir::Type::Form(form) => return self.form(*form),
            dir::Type::Dynamic(dynamic) => format!("Dynamic<{}>", self.global(dynamic.constraint)?),
            dir::Type::Array(array) => format!("{}[]", self.global(array.element)?),
            dir::Type::FixedArray(array) => {
                let element = self.global(array.element)?;
                let count = self.global(array.count)?;

                format!("[{element}; {count}]")
            }
            dir::Type::Range(range) => return self.range(*range),
            dir::Type::Slice(slice) => format!("[{}]", self.global(slice.element)?),
            dir::Type::Tuple(tuple) => return self.tuple(*tuple),
            dir::Type::Shape(shape) => return self.shape(*shape),
            dir::Type::FunctionSignature(function) => return self.function(function),
            dir::Type::Function(function) => return self.global(function.signature),
            dir::Type::FunctionPointer(function) => return self.global(function.signature),
            dir::Type::Union(union) => return self.type_list(union.elements, " | "),
            dir::Type::Intersection(intersection) => {
                return self.type_list(intersection.elements, " & ");
            }
            dir::Type::This => "this".to_string(),
            dir::Type::Operation(operation) => return self.operation(operation),
            dir::Type::Key(key) => return self.static_key(*key),
            dir::Type::Variable(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic => return None,
        };

        Some(text)
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

    /// Format one scalar literal.
    fn literal(&self, literal: dir::ScalarLiteral) -> String {
        match literal {
            dir::ScalarLiteral::Null => "null".to_string(),
            dir::ScalarLiteral::Undefined => "undefined".to_string(),
            dir::ScalarLiteral::Boolean(value) => value.to_string(),
            dir::ScalarLiteral::Integer(value) => value.to_string(),
            dir::ScalarLiteral::Bigint(value) => format!("{value}n"),
            dir::ScalarLiteral::Float(value) => value.to_string(),
            dir::ScalarLiteral::Character(value) => {
                let value = value.escape_default();

                format!("'{value}'")
            }
            dir::ScalarLiteral::String(value) => quoted_string(value, self.module.strings()),
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = self.module.strings().get(content);
                let flags = match flags {
                    Some(flags) => self.module.strings().get(flags),
                    None => "",
                };

                format!("/{content}/{flags}")
            }
        }
    }

    /// Format one generic instance.
    fn instance(&self, instance: dir::GenericInstance) -> Option<String> {
        let symbol = self.symbol(instance.symbol)?;
        let arguments = self.module.types().type_ids(instance.arguments);

        if arguments.is_empty() {
            return Some(symbol);
        }

        Some(format!("{symbol}<{}>", self.join_types(arguments, ", ")?))
    }

    /// Format one generic parameter.
    fn generic_parameter(&self, parameter: dir::GlobalGenericParameterId) -> Option<String> {
        let parameter_module = self.module.module_context(parameter.module_id);
        let parameter = parameter_module
            .generics()
            .get_parameter(parameter.local_id);

        match parameter.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                TypeFormatter::new(&parameter_module).symbol(symbol)
            }
            dir::GenericParameterKey::Generated(name) => {
                Some(parameter_module.strings().get(name).to_string())
            }
        }
    }

    /// Format one member type.
    fn member(&self, member: dir::MemberType) -> Option<String> {
        let owner = self.global(member.owner)?;
        let key = self.static_key(member.key)?;
        let arguments = self.module.types().type_ids(member.arguments);

        if arguments.is_empty() {
            return Some(format!("{owner}.{key}"));
        }

        Some(format!(
            "{owner}.{key}<{}>",
            self.join_types(arguments, ", ")?
        ))
    }

    /// Format one canonical form type.
    fn form(&self, form: dir::FormType) -> Option<String> {
        let value = self.global(form.value)?;
        let text = match form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed { .. } => format!("&{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Placed { .. } => format!("placed {value}"),
            dir::Form::Readonly => format!("readonly {value}"),
        };

        Some(text)
    }

    /// Format one scalar interval type.
    fn range(&self, range: dir::RangeType) -> Option<String> {
        let start = match range.start {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let end = match range.end {
            Some(literal) => self.literal(literal),
            None => String::new(),
        };
        let operator = if range.is_inclusive { "..=" } else { ".." };

        Some(format!("{start}{operator}{end}"))
    }

    /// Format one tuple type.
    fn tuple(&self, tuple: dir::TupleType) -> Option<String> {
        let elements = self.module.types().elements(tuple.elements);
        let elements = elements
            .iter()
            .map(|element| self.tuple_element(element))
            .collect::<Option<Vec<_>>>()?
            .join(", ");

        let text = match tuple.form {
            dir::TupleForm::Tuple => format!("({elements})"),
            dir::TupleForm::Array => format!("[{elements}]"),
        };

        Some(text)
    }

    /// Format one tuple element.
    fn tuple_element(&self, element: &dir::TypeElement) -> Option<String> {
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
        text.push_str(&self.global(element.ty)?);

        Some(text)
    }

    /// Format one structural shape type.
    fn shape(&self, shape: dir::ShapeType) -> Option<String> {
        let mut parts = Vec::new();

        parts.extend(
            self.module
                .types()
                .fields(shape.fields)
                .iter()
                .map(|field| self.field(field))
                .collect::<Option<Vec<_>>>()?,
        );
        parts.extend(
            self.module
                .types()
                .index_signatures(shape.index_signatures)
                .iter()
                .map(|signature| self.index_signature(signature))
                .collect::<Option<Vec<_>>>()?,
        );

        Some(format!("{{ {} }}", parts.join("; ")))
    }

    /// Format one structural field.
    fn field(&self, field: &dir::TypeField) -> Option<String> {
        let readonly = if field.is_readonly { "readonly " } else { "" };
        let optional = if field.is_optional { "?" } else { "" };
        let key = self.static_key(field.key)?;
        let ty = self.global(field.ty)?;

        Some(format!("{readonly}{key}{optional}: {ty}"))
    }

    /// Format one index signature.
    fn index_signature(&self, signature: &dir::TypeIndexSignature) -> Option<String> {
        let readonly = if signature.is_readonly {
            "readonly "
        } else {
            ""
        };
        let optional = if signature.is_optional { "?" } else { "" };
        let name = self.module.strings().get(signature.name);
        let key_type = self.global(signature.key_type)?;
        let value_type = self.global(signature.value_type)?;

        Some(format!(
            "{readonly}[{name}: {key_type}]{optional}: {value_type}"
        ))
    }

    /// Format one function signature type.
    fn function(&self, function: &dir::FunctionSignatureType) -> Option<String> {
        let parameters = self.module.types().parameters(function.parameters);
        let parameters = parameters
            .iter()
            .map(|parameter| self.parameter(parameter))
            .collect::<Option<Vec<_>>>()?
            .join(", ");
        let return_type = match function.return_type {
            Some(ty) => self.global(ty)?,
            None => "void".to_string(),
        };
        let prefix = match function.asynchrony {
            dir::Asynchrony::Sync => "",
            dir::Asynchrony::Async => "async ",
        };

        Some(format!("{prefix}({parameters}) => {return_type}"))
    }

    /// Format one function parameter.
    fn parameter(&self, parameter: &dir::FunctionParameterType) -> Option<String> {
        let rest = if parameter.is_rest { "..." } else { "" };
        let optional = if parameter.is_optional { "?" } else { "" };
        let ty = self.global(parameter.ty)?;

        Some(format!("{rest}arg{optional}: {ty}"))
    }

    /// Format one type operation.
    fn operation(&self, operation: &dir::TypeOperation) -> Option<String> {
        match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                Some(format!("{}<{}>", mapping.text(), self.global(*target)?))
            }
            dir::TypeOperation::Conditional(conditional) => {
                self.conditional_operation(*conditional)
            }
            dir::TypeOperation::Narrow(narrow) => self.narrow_operation(*narrow),
            dir::TypeOperation::Mapped(mapped) => self.mapped_operation(*mapped),
            dir::TypeOperation::Index(index) => Some(format!(
                "{}[{}]",
                self.global(index.left)?,
                self.global(index.index)?
            )),
            dir::TypeOperation::Infer(infer) => self.infer_operation(*infer),
            dir::TypeOperation::TypeOf(query) => {
                Some(format!("typeof {}", self.type_query_text(query.value)?))
            }
            dir::TypeOperation::KeyOf(target) => {
                Some(format!("keyof {}", self.global(target.target)?))
            }
            dir::TypeOperation::NoInfer(target) => {
                Some(format!("NoInfer<{}>", self.global(target.target)?))
            }
            dir::TypeOperation::Awaited(target) => {
                Some(format!("Awaited<{}>", self.global(target.target)?))
            }
            dir::TypeOperation::TryOutput { value } => Some(format!("{}?", self.global(*value)?)),
            dir::TypeOperation::TryResidual { value } => {
                Some(format!("residual {}", self.global(*value)?))
            }
            dir::TypeOperation::StaticBinary(binary) => self.static_binary_operation(*binary),
            dir::TypeOperation::StaticUnary(unary) => self.static_unary_operation(*unary),
            dir::TypeOperation::TemplateLiteral(_) => None,
        }
    }

    /// Format one static key.
    fn static_key(&self, key: dir::StaticKey) -> Option<String> {
        let text = match key {
            dir::StaticKey::Name(name) => self.module.strings().get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)) => {
                format_unique_symbol_qualified_name(symbol, self.module)?
            }
            dir::StaticKey::Symbol(dir::SymbolKey::Registry(name)) => {
                let name = quoted_string(name, self.module.strings());

                format!("Symbol.for({name})")
            }
        };

        Some(text)
    }

    /// Format one symbol path.
    fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        format_symbol_path(symbol_id, self.module)
    }

    /// Format one list of type ids.
    fn type_list(&self, list: dir::TypeListId, separator: &str) -> Option<String> {
        self.join_types(self.module.types().type_ids(list), separator)
    }

    /// Format and join global type ids.
    fn join_types(&self, types: &[dir::GlobalTypeId], separator: &str) -> Option<String> {
        let types = types
            .iter()
            .map(|ty| self.global(*ty))
            .collect::<Option<Vec<_>>>()?;

        Some(types.join(separator))
    }

    /// Format one conditional type operation.
    fn conditional_operation(&self, conditional: dir::ConditionalType) -> Option<String> {
        let left = self.global(conditional.left)?;
        let right = self.global(conditional.right)?;
        let then_type = self.global(conditional.then_type)?;
        let else_type = self.global(conditional.else_type)?;

        Some(format!(
            "{left} extends {right} ? {then_type} : {else_type}"
        ))
    }

    /// Format one runtime guard narrowing type operation.
    fn narrow_operation(&self, narrow: dir::NarrowType) -> Option<String> {
        let source = self.global(narrow.source)?;
        let target = self.global(narrow.target)?;
        let operator = if narrow.is_positive { "is" } else { "is not" };

        Some(format!("{source} {operator} {target}"))
    }

    /// Format one mapped type operation.
    fn mapped_operation(&self, mapped: dir::MappedType) -> Option<String> {
        let parameter = self.module.strings().get(mapped.parameter.name);
        let constraint = self.global(mapped.parameter.constraint)?;
        let value = self.global(mapped.value)?;
        let remap = match mapped.parameter.key_remap {
            Some(key_remap) => format!(" as {}", self.global(key_remap)?),
            None => String::new(),
        };
        let readonly = mapped_modifier_text(mapped.modifiers.readonly, "readonly ");
        let optional = mapped_modifier_text(mapped.modifiers.optional, "?");

        Some(format!(
            "{{ {readonly}[{parameter} in {constraint}{remap}]{optional}: {value} }}"
        ))
    }

    /// Format one infer type operation.
    fn infer_operation(&self, infer: dir::InferType) -> Option<String> {
        let name = match (infer.name, infer.symbol) {
            (Some(name), _) => self.module.strings().get(name).to_string(),
            (None, Some(symbol)) => self.symbol(symbol)?,
            (None, None) => return None,
        };
        let constraint = match infer.constraint {
            Some(constraint) => format!(" extends {}", self.global(constraint)?),
            None => String::new(),
        };

        Some(format!("infer {name}{constraint}"))
    }

    /// Format one static binary type operation.
    fn static_binary_operation(&self, binary: dir::StaticBinaryType) -> Option<String> {
        let left = self.global(binary.left)?;
        let right = self.global(binary.right)?;
        let operator = binary.operator.text();

        Some(format!("{left} {operator} {right}"))
    }

    /// Format one static unary type operation.
    fn static_unary_operation(&self, unary: dir::StaticUnaryType) -> Option<String> {
        let target = self.global(unary.target)?;
        let operator = unary.operator.text();

        Some(format!("{operator}{target}"))
    }

    /// Return one type query operand text.
    fn type_query_text(&self, value: dir::GlobalNodeIdAny) -> Option<String> {
        if value.local_id.ty != dir::NodeType::Expression {
            return None;
        }
        if value.module_id != self.module.module_id() {
            return None;
        }

        let id = value.into_typed::<dir::Expression>().local_id;
        let path = self.module.tree().reference_path(id)?;

        Some(
            path.segments
                .iter()
                .map(|segment| self.module.strings().get(*segment))
                .collect::<Vec<_>>()
                .join("."),
        )
    }
}

/// Format a global type id as a human-readable string.
pub fn format_global_type(
    ty_id: dir::GlobalTypeId,
    module: &ModuleQueryContext<'_>,
) -> Option<String> {
    TypeFormatter::new(module).global(ty_id)
}

/// Format a type as a human-readable string.
pub fn format_type(ty: &dir::Type, module: &ModuleQueryContext<'_>) -> Option<String> {
    TypeFormatter::new(module).ty(ty)
}

/// Format the symbol path for a symbol within its module.
pub fn format_symbol_path(
    symbol_id: dir::GlobalSymbolId,
    root_module: &ModuleQueryContext<'_>,
) -> Option<String> {
    let module = root_module.module_context(symbol_id.module_id);
    let symbols = module.symbols();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    let symbol_name = static_key_segment(symbol.key, module.strings())?;
    let mut segments = vec![symbol_name];

    let mut scope_id = symbol.scope.id;
    let mut seen_scopes = HashSet::new();
    loop {
        if !seen_scopes.insert(scope_id) {
            panic!("cyclic symbol scope chain at {scope_id:?}");
        }

        let scope = symbols.get_scope_by_id(scope_id);
        if let Some(owner_id) = scope.owner {
            if owner_id != symbol_id.into_local() {
                let owner = symbols.get_symbol(owner_id);
                let owner_name = static_key_segment(owner.key, module.strings())?;
                segments.push(owner_name);
            }
        }

        let Some(parent) = scope.parent else {
            break;
        };
        scope_id = parent.id;
    }

    segments.reverse();

    Some(segments.join("."))
}

/// Format the qualified name of a symbol with module prefix.
pub fn format_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    root_module: &ModuleQueryContext<'_>,
) -> Option<String> {
    let source_module = root_module
        .repository()
        .module(root_module.revision(), symbol_id.module_id)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read qualified symbol module {:?}: {error}",
                symbol_id.module_id
            )
        })
        .unwrap_or_else(|| panic!("missing qualified symbol module {:?}", symbol_id.module_id));
    let package = root_module
        .repository()
        .package(root_module.revision(), source_module.package_id)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read qualified symbol package {:?}: {error}",
                source_module.package_id
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "missing qualified symbol package {:?}",
                source_module.package_id
            )
        });

    let package_name = package.name.as_ref()?;
    if package_name.is_empty() {
        return None;
    }
    let module_path = module_path_without_extension(source_module.as_ref(), package.as_ref());
    let symbol_path = format_symbol_path(symbol_id, root_module)?;
    let module_prefix = if module_path.is_empty() {
        package_name.to_string()
    } else {
        format!("{package_name}/{module_path}")
    };

    Some(format!("{module_prefix}:{symbol_path}"))
}

/// Format the qualified name of a unique symbol.
pub fn format_unique_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    module: &ModuleQueryContext<'_>,
) -> Option<String> {
    let name = format_symbol_qualified_name(symbol_id, module)?;

    Some(format!("{name}#unique"))
}

/// Resolve the package relative module path without extension.
fn module_path_without_extension(module: &Module, package: &Package) -> String {
    let module_path = if let Some(path) = &module.path {
        let relative = package
            .path
            .as_ref()
            .and_then(|package_path| path.strip_prefix(package_path).ok())
            .unwrap_or(path);
        relative.to_string_lossy().to_string()
    } else {
        module.uri.to_string()
    };

    let module_path = normalize_path_separators(&module_path);
    strip_extension_from_path(&module_path)
}

/// Normalize a module path to use forward slashes.
fn normalize_path_separators(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    normalized.trim_start_matches('/').to_string()
}

/// Strip the file extension from a module path.
fn strip_extension_from_path(path: &str) -> String {
    let (prefix, leaf) = path.rsplit_once('/').unwrap_or(("", path));
    let stripped = leaf.rsplit_once('.').map(|(base, _)| base).unwrap_or(leaf);
    if prefix.is_empty() {
        stripped.to_string()
    } else {
        format!("{prefix}/{stripped}")
    }
}

/// Quote one pooled string as source text.
fn quoted_string(string_id: StringId, strings: &StringPool) -> String {
    let escaped = strings.get(string_id).escape_default().to_string();

    format!("\"{escaped}\"")
}

/// Return the textual prefix for one mapped type modifier.
fn mapped_modifier_text(modifier: dir::MappedTypeModifier, token: &str) -> String {
    if modifier.is_present() {
        format!("{}{token}", modifier.sign())
    } else {
        String::new()
    }
}

/// Convert a static key into a symbol path segment.
fn static_key_segment(key: Option<dir::StaticKey>, strings: &StringPool) -> Option<String> {
    let key = key?;
    match key {
        dir::StaticKey::Name(name) => Some(strings.get(name).to_string()),
        dir::StaticKey::Index(index) => Some(index.to_string()),
        dir::StaticKey::Symbol(_) => None,
    }
}
