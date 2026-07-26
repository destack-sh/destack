use destack_core::{StringId, StringPool};
use destack_dir as dir;
use destack_repository::{Module, Package, RepositoryError};
use rustc_hash::FxHashSet;

use crate::source::strip_module_extension;
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

macro_rules! formatted {
    ($expression:expr) => {
        match $expression? {
            Some(text) => text,
            None => return Ok(None),
        }
    };
}

/// Formatter for checked DIR types.
struct TypeFormatter<'owner, 'module, 'program> {
    /// The module that owns list ids read by this formatter.
    module: &'owner ModuleQueryContext<'module>,
    /// The program used for global type and symbol reads.
    program: &'owner ProgramQueryContext<'program>,
}

impl<'owner, 'module, 'program> TypeFormatter<'owner, 'module, 'program> {
    /// Create a formatter for one checked module.
    fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
    ) -> Self {
        Self { module, program }
    }

    /// Format one global type id.
    fn global(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<String>> {
        self.module
            .read_global_type(self.program, type_id, |checked_type, owner| {
                TypeFormatter::new(owner, self.program).format(checked_type)
            })?
    }

    /// Format one callable type with authored parameter names.
    fn callable(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: &[String],
    ) -> QueryResult<Option<String>> {
        self.module
            .read_global_type(self.program, type_id, |checked_type, owner| {
                let formatter = TypeFormatter::new(owner, self.program);
                match checked_type {
                    dir::Type::FunctionSignature(function) => formatter
                        .named_function(owner.types().signature(*function), parameter_names),
                    dir::Type::Function(function) => {
                        formatter.callable(function.signature, parameter_names)
                    }
                    dir::Type::FunctionPointer(function) => {
                        formatter.callable(function.signature, parameter_names)
                    }
                    _ => formatter.format(checked_type),
                }
            })?
    }

    /// Format one constructor type with its selected nominal result.
    fn constructor(
        &self,
        type_id: dir::GlobalTypeId,
        parameter_names: &[String],
        result: &str,
    ) -> QueryResult<Option<String>> {
        self.module
            .read_global_type(self.program, type_id, |checked_type, owner| {
                TypeFormatter::new(owner, self.program).constructor_type(
                    checked_type,
                    parameter_names,
                    result,
                )
            })?
    }

    /// Format one locally owned constructor type.
    fn constructor_type(
        &self,
        checked_type: &dir::Type,
        parameter_names: &[String],
        result: &str,
    ) -> QueryResult<Option<String>> {
        match checked_type {
            dir::Type::FunctionSignature(function) => self.named_constructor(
                self.module.types().signature(*function),
                parameter_names,
                result,
            ),
            dir::Type::Function(function) => {
                self.constructor(function.signature, parameter_names, result)
            }
            dir::Type::FunctionPointer(function) => {
                self.constructor(function.signature, parameter_names, result)
            }
            _ => Err(QueryError::invalid("formatted type")),
        }
    }

    /// Format one checked type owned by this formatter's module.
    fn format(&self, checked_type: &dir::Type) -> QueryResult<Option<String>> {
        let text = match checked_type {
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
            dir::Type::Parameter(parameter) => return self.generic_parameter(*parameter),
            dir::Type::Erased(_) => "*".to_string(),
            dir::Type::Member(member) => return self.member(*self.module.types().member(*member)),
            dir::Type::Refined(refined) => {
                let refined = *self.module.types().refined(*refined);
                let base = formatted!(self.global(refined.base));
                let key = formatted!(self.static_key(refined.key));
                let value = formatted!(self.global(refined.value));

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::EnumMember(member) => return self.symbol(member.member),
            dir::Type::Form(form) => return self.form(*form),
            dir::Type::Dynamic(dynamic) => {
                let constraint = formatted!(self.global(dynamic.constraint));

                format!("Dynamic<{constraint}>")
            }
            dir::Type::Array(array) => {
                let element = formatted!(self.global(array.element));

                format!("{element}[]")
            }
            dir::Type::FixedArray(array) => {
                let element = formatted!(self.global(array.element));
                let count = formatted!(self.global(array.count));

                format!("[{element}; {count}]")
            }
            dir::Type::Range(range) => return self.range(*range),
            dir::Type::Slice(slice) => {
                let element = formatted!(self.global(slice.element));

                format!("[{element}]")
            }
            dir::Type::Tuple(tuple) => return self.tuple(*tuple),
            dir::Type::Shape(shape) => return self.shape(*shape),
            dir::Type::FunctionSignature(function) => {
                return self.function(self.module.types().signature(*function));
            }
            dir::Type::Function(function) => return self.global(function.signature),
            dir::Type::FunctionPointer(function) => return self.global(function.signature),
            dir::Type::Union(union) => return self.type_list(union.elements, " | "),
            dir::Type::Intersection(intersection) => {
                return self.type_list(intersection.elements, " & ");
            }
            dir::Type::This => "this".to_string(),
            dir::Type::Operation(operation) => {
                return self.operation(self.module.types().operation(*operation));
            }
            dir::Type::Key(key) => return self.static_key(*key),
            dir::Type::Variable(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic => return Err(QueryError::invalid("formatted type")),
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
    fn instance(&self, instance: dir::GenericApplication) -> QueryResult<Option<String>> {
        let symbol = formatted!(self.symbol(instance.symbol));
        let arguments = self.module.types().type_ids(instance.arguments);

        if arguments.is_empty() {
            return Ok(Some(symbol));
        }

        let arguments = formatted!(self.join_types(arguments, ", "));

        Ok(Some(format!("{symbol}<{arguments}>")))
    }

    /// Format one generic parameter.
    fn generic_parameter(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> QueryResult<Option<String>> {
        let parameter_module = self.program.module(parameter.module_id)?;
        let parameter = parameter_module
            .generics()
            .get_parameter(parameter.local_id);

        let name = match parameter.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                let symbol = parameter_module.symbols().get_symbol(symbol.local_id);

                static_key_segment(symbol.key, parameter_module.strings())
            }
            dir::GenericParameterKey::Generated(name) => {
                Some(parameter_module.strings().get(name).to_string())
            }
        };

        Ok(name)
    }

    /// Format one member type.
    fn member(&self, member: dir::MemberType) -> QueryResult<Option<String>> {
        let owner = formatted!(self.global(member.owner));
        let key = formatted!(self.static_key(member.key));
        let arguments = self.module.types().type_ids(member.arguments);

        if arguments.is_empty() {
            return Ok(Some(format!("{owner}.{key}")));
        }

        let arguments = formatted!(self.join_types(arguments, ", "));

        Ok(Some(format!("{owner}.{key}<{arguments}>")))
    }

    /// Format one canonical form type.
    fn form(&self, form: dir::FormType) -> QueryResult<Option<String>> {
        let value = formatted!(self.global(form.value));
        let text = match form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Borrowed(_) => format!("&{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Placed { .. } => format!("placed {value}"),
            dir::Form::Readonly => format!("readonly {value}"),
        };

        Ok(Some(text))
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
        let mut formatted_elements = Vec::with_capacity(elements.len());
        for element in elements {
            formatted_elements.push(formatted!(self.tuple_element(element)));
        }
        let elements = formatted_elements.join(", ");

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
        let type_text = formatted!(self.global(element.ty));
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

        Ok(Some(format!("{{ {} }}", members.join("; "))))
    }

    /// Format one structural field.
    fn field(&self, field: &dir::TypeField) -> QueryResult<Option<String>> {
        let readonly = if field.is_readonly { "readonly " } else { "" };
        let optional = if field.is_optional { "?" } else { "" };
        let key = formatted!(self.static_key(field.key));
        let type_text = formatted!(self.global(field.ty));

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
        let key_type = formatted!(self.global(signature.key_type));
        let value_type = formatted!(self.global(signature.value_type));

        Ok(Some(format!(
            "{readonly}[{name}: {key_type}]{optional}: {value_type}"
        )))
    }

    /// Format one function signature type.
    fn function(&self, function: &dir::FunctionSignatureType) -> QueryResult<Option<String>> {
        let parameters = self.module.types().parameters(function.parameters);
        let parameter_names = vec!["arg".to_string(); parameters.len()];

        self.named_function(function, &parameter_names)
    }

    /// Format one function signature type with selected parameter names.
    fn named_function(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: &[String],
    ) -> QueryResult<Option<String>> {
        let return_type = match function.return_type {
            Some(type_id) => formatted!(self.global(type_id)),
            None => "void".to_string(),
        };

        self.named_function_with_return(function, parameter_names, &return_type)
    }

    /// Format one constructor signature with its selected nominal result.
    fn named_constructor(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: &[String],
        result: &str,
    ) -> QueryResult<Option<String>> {
        self.named_function_with_return(function, parameter_names, result)
    }

    /// Format one function signature with its selected return text.
    fn named_function_with_return(
        &self,
        function: &dir::FunctionSignatureType,
        parameter_names: &[String],
        return_type: &str,
    ) -> QueryResult<Option<String>> {
        let parameters = self.module.types().parameters(function.parameters);
        if parameters.len() != parameter_names.len() {
            return Ok(None);
        }
        let mut formatted_parameters = Vec::with_capacity(parameters.len());
        for (parameter, name) in parameters.iter().zip(parameter_names) {
            let parameter = formatted!(self.parameter(parameter, name.trim_start_matches("...")));
            formatted_parameters.push(parameter);
        }
        let parameters = formatted_parameters.join(", ");
        let prefix = match function.asynchrony {
            dir::Asynchrony::Sync => "",
            dir::Asynchrony::Async => "async ",
        };

        Ok(Some(format!("{prefix}({parameters}) => {return_type}")))
    }

    /// Format one function parameter.
    fn parameter(
        &self,
        parameter: &dir::FunctionParameterType,
        parameter_name: &str,
    ) -> QueryResult<Option<String>> {
        let rest = if parameter.is_rest { "..." } else { "" };
        let optional = if parameter.is_optional { "?" } else { "" };
        let type_text = formatted!(self.global(parameter.ty));

        Ok(Some(format!(
            "{rest}{parameter_name}{optional}: {type_text}"
        )))
    }

    /// Format one type operation.
    fn operation(&self, operation: &dir::TypeOperation) -> QueryResult<Option<String>> {
        match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = formatted!(self.global(*target));

                Ok(Some(format!("{}<{target}>", mapping.text())))
            }
            dir::TypeOperation::Conditional(conditional) => {
                self.conditional_operation(*conditional)
            }
            dir::TypeOperation::Narrow(narrow) => self.narrow_operation(*narrow),
            dir::TypeOperation::Mapped(mapped) => self.mapped_operation(*mapped),
            dir::TypeOperation::Index(index) => {
                let left = formatted!(self.global(index.left));
                let index = formatted!(self.global(index.index));

                Ok(Some(format!("{left}[{index}]")))
            }
            dir::TypeOperation::Infer(infer) => self.infer_operation(*infer),
            dir::TypeOperation::TypeOf(query) => {
                let value = formatted!(self.type_query_text(query.value));

                Ok(Some(format!("typeof {value}")))
            }
            dir::TypeOperation::KeyOf(target) => {
                let target = formatted!(self.global(target.target));

                Ok(Some(format!("keyof {target}")))
            }
            dir::TypeOperation::NoInfer(target) => {
                let target = formatted!(self.global(target.target));

                Ok(Some(format!("NoInfer<{target}>")))
            }
            dir::TypeOperation::Awaited(target) => {
                let target = formatted!(self.global(target.target));

                Ok(Some(format!("Awaited<{target}>")))
            }
            dir::TypeOperation::TryOutput { value } => {
                let value = formatted!(self.global(*value));

                Ok(Some(format!("{value}?")))
            }
            dir::TypeOperation::TryResidual { value } => {
                let value = formatted!(self.global(*value));

                Ok(Some(format!("residual {value}")))
            }
            dir::TypeOperation::StaticBinary(binary) => self.static_binary_operation(*binary),
            dir::TypeOperation::StaticUnary(unary) => self.static_unary_operation(*unary),
            dir::TypeOperation::TemplateLiteral(template) => self.template_literal(*template),
        }
    }

    /// Format one static key.
    fn static_key(&self, key: dir::StaticKey) -> QueryResult<Option<String>> {
        let text = match key {
            dir::StaticKey::Name(name) => self.module.strings().get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)) => {
                formatted!(format_unique_symbol_qualified_name(
                    symbol,
                    self.module,
                    self.program
                ))
            }
            dir::StaticKey::Symbol(dir::SymbolKey::Registry(name)) => {
                let name = quoted_string(name, self.module.strings());

                format!("Symbol.for({name})")
            }
        };

        Ok(Some(text))
    }

    /// Format one symbol path.
    fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<Option<String>> {
        format_symbol_path(symbol_id, self.program)
    }

    /// Format one list of type ids.
    fn type_list(&self, list: dir::TypeListId, separator: &str) -> QueryResult<Option<String>> {
        self.join_types(self.module.types().type_ids(list), separator)
    }

    /// Format and join global type ids.
    fn join_types(
        &self,
        types: &[dir::GlobalTypeId],
        separator: &str,
    ) -> QueryResult<Option<String>> {
        let mut formatted_types = Vec::with_capacity(types.len());
        for type_id in types {
            formatted_types.push(formatted!(self.global(*type_id)));
        }

        Ok(Some(formatted_types.join(separator)))
    }

    /// Format one conditional type operation.
    fn conditional_operation(
        &self,
        conditional: dir::ConditionalType,
    ) -> QueryResult<Option<String>> {
        let left = formatted!(self.global(conditional.left));
        let right = formatted!(self.global(conditional.right));
        let then_type = formatted!(self.global(conditional.then_type));
        let else_type = formatted!(self.global(conditional.else_type));

        Ok(Some(format!(
            "{left} extends {right} ? {then_type} : {else_type}"
        )))
    }

    /// Format one runtime guard narrowing type operation.
    fn narrow_operation(&self, narrow: dir::NarrowType) -> QueryResult<Option<String>> {
        let source = formatted!(self.global(narrow.source));
        let target = formatted!(self.global(narrow.target));
        let operator = if narrow.is_positive { "is" } else { "is not" };

        Ok(Some(format!("{source} {operator} {target}")))
    }

    /// Format one mapped type operation.
    fn mapped_operation(&self, mapped: dir::MappedType) -> QueryResult<Option<String>> {
        let parameter = self.module.strings().get(mapped.parameter.name);
        let constraint = formatted!(self.global(mapped.parameter.constraint));
        let value = formatted!(self.global(mapped.value));
        let remap = match mapped.parameter.key_remap {
            Some(key_remap) => {
                let key_remap = formatted!(self.global(key_remap));

                format!(" as {key_remap}")
            }
            None => String::new(),
        };
        let readonly = mapped_modifier_text(mapped.modifiers.readonly, "readonly ");
        let optional = mapped_modifier_text(mapped.modifiers.optional, "?");

        Ok(Some(format!(
            "{{ {readonly}[{parameter} in {constraint}{remap}]{optional}: {value} }}"
        )))
    }

    /// Format one infer type operation.
    fn infer_operation(&self, infer: dir::InferType) -> QueryResult<Option<String>> {
        let name = match (infer.name, infer.symbol) {
            (Some(name), _) => self.module.strings().get(name).to_string(),
            (None, Some(symbol)) => formatted!(self.symbol(symbol)),
            (None, None) => "_".to_string(),
        };
        let constraint = match infer.constraint {
            Some(constraint) => {
                let constraint = formatted!(self.global(constraint));

                format!(" extends {constraint}")
            }
            None => String::new(),
        };

        Ok(Some(format!("infer {name}{constraint}")))
    }

    /// Format one static binary type operation.
    fn static_binary_operation(
        &self,
        binary: dir::StaticBinaryType,
    ) -> QueryResult<Option<String>> {
        let left = formatted!(self.global(binary.left));
        let right = formatted!(self.global(binary.right));
        let operator = binary.operator.text();

        Ok(Some(format!("{left} {operator} {right}")))
    }

    /// Format one static unary type operation.
    fn static_unary_operation(&self, unary: dir::StaticUnaryType) -> QueryResult<Option<String>> {
        let target = formatted!(self.global(unary.target));
        let operator = unary.operator.text();

        Ok(Some(format!("{operator}{target}")))
    }

    /// Format one template literal type operation.
    fn template_literal(&self, template: dir::TemplateLiteralType) -> QueryResult<Option<String>> {
        let strings = self.module.types().type_ids(template.strings);
        let spans = self.module.types().type_ids(template.spans);
        if strings.len() != spans.len() + 1 {
            return Err(QueryError::invalid("template literal"));
        }

        let mut text = String::from("`");
        for (index, string_type) in strings.iter().copied().enumerate() {
            let segment = formatted!(self.read_template_segment(string_type));
            text.push_str(&segment);

            if let Some(span_type) = spans.get(index) {
                text.push_str("${");
                let span = formatted!(self.global(*span_type));
                text.push_str(&span);
                text.push('}');
            }
        }
        text.push('`');

        Ok(Some(text))
    }

    /// Read one literal string segment from a template type.
    fn read_template_segment(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<String>> {
        self.module
            .read_global_type(
                self.program,
                type_id,
                |checked_type, module| match checked_type {
                    dir::Type::Literal(dir::ScalarLiteral::String(string_id)) => {
                        Ok(Some(module.strings().get(*string_id).to_string()))
                    }
                    dir::Type::Error => Ok(None),
                    _ => Err(QueryError::invalid(format!(
                        "template segment: {type_id:?}"
                    ))),
                },
            )?
    }

    /// Return one type query operand text.
    fn type_query_text(&self, value: dir::GlobalNodeIdAny) -> QueryResult<Option<String>> {
        if value.local_id.ty != dir::NodeType::Expression {
            return Err(QueryError::invalid(format!("type query: {value:?}")));
        }
        if value.module_id != self.module.module_id() {
            return Err(QueryError::invalid(format!("type query: {value:?}")));
        }

        let id = value.into_typed::<dir::Expression>().local_id;
        let path = self
            .module
            .view()
            .reference_path(id)
            .ok_or(QueryError::invalid(format!("type query: {value:?}")))?;

        Ok(Some(
            path.segments
                .iter()
                .map(|segment| self.module.strings().get(*segment))
                .collect::<Vec<_>>()
                .join("."),
        ))
    }
}

/// Format a global type id as a human-readable string.
pub(crate) fn format_global_type(
    type_id: dir::GlobalTypeId,
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    TypeFormatter::new(module, program).global(type_id)
}

/// Format a global callable type with authored parameter names.
pub(crate) fn format_global_callable_type(
    type_id: dir::GlobalTypeId,
    parameter_names: &[String],
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    TypeFormatter::new(module, program).callable(type_id, parameter_names)
}

/// Format a global constructor type with its selected nominal result.
pub(crate) fn format_global_constructor_type(
    type_id: dir::GlobalTypeId,
    parameter_names: &[String],
    result: &str,
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    TypeFormatter::new(module, program).constructor(type_id, parameter_names, result)
}

/// Format the symbol path for a symbol within its module.
pub(crate) fn format_symbol_path(
    symbol_id: dir::GlobalSymbolId,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    let module = program.module(symbol_id.module_id)?;
    let symbols = module.symbols();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    let Some(symbol_name) = static_key_segment(symbol.key, module.strings()) else {
        return Ok(None);
    };
    let mut segments = vec![symbol_name];

    let mut scope_id = symbol.scope.id;
    let mut seen_scopes = FxHashSet::default();
    loop {
        if !seen_scopes.insert(scope_id) {
            return Err(QueryError::cycle(format!("symbol scope: {symbol_id:?}")));
        }

        let scope = symbols.get_scope_by_id(scope_id);

        // include named containers but omit anonymous namespace owners
        if let Some(owner_id) = scope.owner
            && owner_id != symbol_id.into_local()
        {
            let owner = symbols.get_symbol(owner_id);
            if let Some(owner_name) = static_key_segment(owner.key, module.strings()) {
                segments.push(owner_name);
            } else if owner.role != dir::SymbolRole::Namespace {
                return Ok(None);
            }
        }

        let Some(parent) = scope.parent else {
            break;
        };
        scope_id = parent.id;
    }

    segments.reverse();

    Ok(Some(segments.join(".")))
}

/// Format the qualified name of a symbol with module prefix.
pub(crate) fn format_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    root_module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    let source_module = root_module
        .repository()
        .module(root_module.revision(), symbol_id.module_id)?
        .ok_or(RepositoryError::MissingModule {
            module: symbol_id.module_id,
        })?;
    let package = root_module
        .repository()
        .package(root_module.revision(), source_module.package_id)?
        .ok_or(RepositoryError::MissingPackage {
            package: source_module.package_id,
        })?;

    let Some(package_name) = package.name.as_ref() else {
        return Ok(None);
    };
    if package_name.is_empty() {
        return Ok(None);
    }
    let Some(module_path) =
        module_path_without_extension(source_module.as_ref(), package.as_ref())?
    else {
        return Ok(None);
    };
    let symbol_path = formatted!(format_symbol_path(symbol_id, program));
    let module_prefix = if module_path.is_empty() {
        package_name.to_string()
    } else {
        format!("{package_name}/{module_path}")
    };

    Ok(Some(format!("{module_prefix}:{symbol_path}")))
}

/// Format the qualified name of a unique symbol.
pub(crate) fn format_unique_symbol_qualified_name(
    symbol_id: dir::GlobalSymbolId,
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
) -> QueryResult<Option<String>> {
    let name = formatted!(format_symbol_qualified_name(symbol_id, module, program));

    Ok(Some(format!("{name}#unique")))
}

/// Resolve the package relative module path without extension.
fn module_path_without_extension(
    module: &Module,
    package: &Package,
) -> QueryResult<Option<String>> {
    let Some(module_path) = module.path.as_ref() else {
        return Ok(None);
    };
    let Some(package_path) = package.path.as_ref() else {
        return Ok(None);
    };
    let relative = module_path
        .strip_prefix(package_path)
        .map_err(|_| QueryError::invalid(format!("module path: {:?}", module.id)))?;
    let relative = relative
        .to_str()
        .ok_or_else(|| QueryError::invalid(format!("non-Unicode module path: {relative:?}")))?;

    let module_path = normalize_path_separators(relative);

    Ok(Some(strip_module_extension(&module_path)))
}

/// Normalize a module path to use forward slashes.
fn normalize_path_separators(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    normalized.trim_start_matches('/').to_string()
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
