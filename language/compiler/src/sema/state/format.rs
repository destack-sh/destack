use destack_dir as dir;
use destack_source::ModuleId;
use std::collections::BTreeMap;
use std::path::Path;

use crate::sema::{CheckState, VariableKind};
use crate::{CompilerError, CompilerResult};

/// Nesting depth after which formatted types elide their details.
const FORMAT_DEPTH: usize = 4;

/// Element count after which formatted lists elide their tails.
const FORMAT_WIDTH: usize = 4;

impl CheckState<'_> {
    /// Format one type for diagnostics.
    pub(in crate::sema) fn format_type(&self, id: dir::GlobalTypeId) -> String {
        self.format_depth(id, FORMAT_DEPTH)
            .unwrap_or_else(|error| unreachable!("check type formatting failed: {error:?}"))
    }

    /// Format one type relative to a source module.
    pub(in crate::sema) fn format_type_at(
        &self,
        module: ModuleId,
        id: dir::GlobalTypeId,
    ) -> String {
        self.format_depth_at(Some(module), id, FORMAT_DEPTH)
            .unwrap_or_else(|error| unreachable!("check type formatting failed: {error:?}"))
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

        // resolve the root before rendering it
        let id = self.shallow_resolve(id)?;
        let next = depth - 1;

        let rendered = match self.ty(id)? {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Hole(hole) => format!("?{hole}"),
            dir::Type::Rigid(rigid) => format!("^{rigid}"),
            dir::Type::Never => "never".to_string(),
            dir::Type::Any => "any".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::This => "this".to_string(),
            dir::Type::Variable(variable) => match self.infer.variable(variable)?.kind {
                VariableKind::Type => "_".to_string(),
                VariableKind::Integer => "{integer}".to_string(),
                VariableKind::Float => "{float}".to_string(),
            },
            dir::Type::Erased(_) => "*".to_string(),

            dir::Type::Primitive(primitive) => format_primitive(&primitive),
            dir::Type::Literal(literal) => self.format_scalar_literal(&literal),
            dir::Type::Key(key) => self.format_key_type(&key),
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)) => {
                format!("'{}", dir::MemoryLiteral::Lifetime(lifetime).text())
            }
            dir::Type::Memory(literal) => {
                format!("\"{}\"", literal.text())
            }
            dir::Type::Static(value) => self.format_static(value),
            dir::Type::Range(range) => self.format_range(&range),

            dir::Type::Parameter(parameter) => self.format_parameter(parameter),
            dir::Type::Reference(reference) => {
                self.format_symbol_path_maybe_at(module, reference.symbol)
            }
            // array applications render in their written rest form
            dir::Type::Application(_) if let Some(element) = self.array_element(id)? => {
                let element = self.format_depth_at(module, element, next)?;

                format!("{element}[]")
            }
            dir::Type::Application(instance) => {
                let name = self.format_symbol_path_maybe_at(module, instance.symbol);
                if instance.arguments.is_empty() {
                    name
                } else {
                    let arguments = self.type_ids(id.module_id, instance.arguments)?;
                    let arguments = self.format_list_at(module, arguments, next)?;

                    format!("{name}<{arguments}>")
                }
            }
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;
                let owner = self.format_depth_at(module, member.owner, next)?;
                let key = self.format_static_key(&member.key);

                format!("{owner}.{key}")
            }
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(id.module_id, refined)?;
                let base = self.format_depth_at(module, refined.base, next)?;
                let key = self.format_static_key(&refined.key);
                let value = self.format_depth_at(module, refined.value, next)?;

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::Variant(variant) => {
                self.format_symbol_path_maybe_at(module, variant.variant)
            }

            // intrinsic collections render their declared names
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
                let elements = self
                    .tuple_elements(id.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect::<Vec<_>>();

                format!("({})", self.format_list_at(module, &elements, next)?)
            }

            dir::Type::Object(shape) => {
                let shape_properties = self.shape_properties(id.module_id, shape.properties)?;
                let index_signatures =
                    self.shape_index_signatures(id.module_id, shape.index_signatures)?;

                let mut fields = Vec::new();
                for property in shape_properties.iter().take(FORMAT_WIDTH) {
                    let key = self.format_static_key(&property.key);
                    let optional = if property.is_optional { "?" } else { "" };

                    fields.push(match property.access {
                        dir::PropertyAccess::Read(ty) => {
                            let ty = self.format_depth_at(module, ty, next)?;

                            format!("readonly {key}{optional}: {ty}")
                        }
                        dir::PropertyAccess::Write(ty) => {
                            let ty = self.format_depth_at(module, ty, next)?;

                            format!("set {key}(value: {ty})")
                        }
                        dir::PropertyAccess::ReadWrite { read, write } if read == write => {
                            let ty = self.format_depth_at(module, read, next)?;

                            format!("{key}{optional}: {ty}")
                        }
                        dir::PropertyAccess::ReadWrite { read, write } => {
                            let read = self.format_depth_at(module, read, next)?;
                            let write = self.format_depth_at(module, write, next)?;

                            format!("get {key}(): {read}; set {key}(value: {write})")
                        }
                    });
                }

                for signature in index_signatures.iter().take(FORMAT_WIDTH - fields.len()) {
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

                let field_count = shape_properties.len() + index_signatures.len();
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
                let function = self.type_signature(id.module_id, function)?;
                let signature_parameters =
                    self.signature_parameters(id.module_id, function.parameters)?;

                let mut parameters = Vec::new();
                for parameter in signature_parameters.iter().take(FORMAT_WIDTH) {
                    parameters.push(self.format_function_parameter_at(module, parameter, next)?);
                }
                if signature_parameters.len() > FORMAT_WIDTH {
                    parameters.push("…".to_string());
                }
                let result = match function.return_type {
                    Some(return_type) => self.format_depth_at(module, return_type, next)?,
                    None => "void".to_string(),
                };
                let generic = self.format_generic_parameters_at(module, function.template, next)?;
                let construct = match function.is_construct {
                    true => "new ",
                    false => "",
                };

                format!(
                    "{construct}{generic}({}) => {result}",
                    parameters.join(", ")
                )
            }
            dir::Type::Function(function) => {
                self.format_depth_at(module, function.signature, next)?
            }
            dir::Type::FunctionPointer(function) => {
                self.format_function_pointer_at(module, &function, next)?
            }

            dir::Type::Union(union) => {
                let union_elements = self.type_ids(id.module_id, union.elements)?;

                let mut elements = Vec::new();
                for element in union_elements.iter().copied().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth_at(module, element, next)?);
                }
                if union_elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" | ")
            }
            dir::Type::Intersection(intersection) => {
                let intersection_elements = self.type_ids(id.module_id, intersection.elements)?;

                let mut elements = Vec::new();
                for element in intersection_elements.iter().copied().take(FORMAT_WIDTH) {
                    elements.push(self.format_depth_at(module, element, next)?);
                }
                if intersection_elements.len() > FORMAT_WIDTH {
                    elements.push("…".to_string());
                }

                elements.join(" & ")
            }

            dir::Type::Form(form) => self.format_form_at(module, id.module_id, &form, next)?,
            dir::Type::Dynamic(dynamic) => {
                format!(
                    "Dynamic<{}>",
                    self.format_depth_at(module, dynamic.constraint, next)?
                )
            }

            dir::Type::Operation(operation) => {
                let operation = self.type_operation(id.module_id, operation)?;

                self.format_operation_at(module, id.module_id, &operation, next)?
            }
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

    /// Format one callable's generic parameters relative to an optional source module.
    fn format_generic_parameters_at(
        &self,
        module: Option<ModuleId>,
        template: Option<dir::GlobalGenericTemplateId>,
        depth: usize,
    ) -> CompilerResult<String> {
        let Some(template_id) = template else {
            return Ok(String::new());
        };
        let template =
            self.generic_template(template_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("callable template {template_id:?} is missing"),
                })?;

        // render declared parameters in template order
        let mut parameters = Vec::new();
        for parameter in &template.parameters {
            let parameter = parameter.into_global(template_id.module_id);
            let binding =
                self.generic_parameter(parameter)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("callable parameter {parameter:?} is missing"),
                    })?;

            // render modifiers from inside out
            let mut label = match binding.key {
                dir::GenericParameterKey::Symbol(symbol) => self.format_symbol(symbol),
                dir::GenericParameterKey::Generated(name) => self.text(name),
            };

            // print tick parameters bare, their kind is implied
            if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                && label
                    .rsplit('.')
                    .next()
                    .is_some_and(|name| name.starts_with('\''))
            {
                parameters.push(label);
                continue;
            }
            if binding.is_variadic {
                label = format!("...{label}");
            }
            if let Some(variance) = binding.variance {
                label = format!("{} {label}", variance.as_str());
            }
            if binding.is_const {
                label = format!("const {label}");
            }

            // render the declared bound and default
            if let Some(constraint) = binding.constraint {
                let constraint = self.format_depth_at(module, constraint, depth)?;
                label = format!("{label}: {constraint}");
            }
            if let Some(default) = binding.default {
                let default = self.format_depth_at(module, default, depth)?;
                label = format!("{label} = {default}");
            }
            parameters.push(label);
        }
        if parameters.is_empty() {
            return Ok(String::new());
        }

        Ok(format!("<{}>", parameters.join(", ")))
    }

    /// Format one function pointer type relative to an optional source module.
    fn format_function_pointer_at(
        &self,
        module: Option<ModuleId>,
        function: &dir::FunctionPointerType,
        depth: usize,
    ) -> CompilerResult<String> {
        let signature_id = self.shallow_resolve(function.signature)?;
        let Some(signature) = self.signature_head(signature_id)? else {
            let signature = self.format_depth_at(module, function.signature, depth)?;

            return Ok(format!("FunctionPointer<{signature}>"));
        };
        let signature_parameters =
            self.signature_parameters(signature_id.module_id, signature.parameters)?;

        let mut parameters = Vec::new();
        for parameter in signature_parameters.iter().take(FORMAT_WIDTH) {
            let parameter = self.format_function_parameter_at(module, parameter, depth)?;

            parameters.push(parameter);
        }
        if signature_parameters.len() > FORMAT_WIDTH {
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

        // prefix the authored parameter name where the declaration wrote one
        let parameter_type = match parameter.name {
            Some(name) => format!("{}: {parameter_type}", self.strings().get(name)),
            None => parameter_type,
        };

        Ok(parameter_type)
    }

    /// Format one memory form relative to an optional source module.
    fn format_form_at(
        &self,
        module: Option<ModuleId>,
        owner: ModuleId,
        form: &dir::FormType,
        depth: usize,
    ) -> CompilerResult<String> {
        let value = self.format_depth_at(module, form.value, depth)?;

        let rendered = match &form.form {
            dir::Form::Managed => value,
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
            dir::Form::Borrowed(borrow) => {
                let borrow = self.type_borrow(owner, *borrow)?;
                let access = match self.ty(self.shallow_resolve(borrow.access)?)? {
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)) => {
                        "readonly "
                    }
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive)) => {
                        "exclusive "
                    }
                    _ => "",
                };

                // render named and static provenance, eliding the frame default
                let lifetime = match self.ty(self.shallow_resolve(borrow.lifetime)?)? {
                    dir::Type::Parameter(parameter) => {
                        let name = self.format_parameter(parameter);
                        match name
                            .rsplit('.')
                            .next()
                            .is_some_and(|name| name.starts_with('\''))
                        {
                            true => format!("{name} "),
                            false => String::new(),
                        }
                    }
                    dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Static)) => {
                        "'static ".to_string()
                    }
                    _ => String::new(),
                };

                format!("&{lifetime}{access}{value}")
            }
            dir::Form::Placed { place } => {
                let place = match self.ty(self.shallow_resolve(*place)?)? {
                    dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))) => {
                        match space {
                            dir::Space::Local => "local ",
                            dir::Space::Shared => "shared ",
                        }
                    }
                    _ => return Ok(format!("Placed<{value}, {}>", self.format_type(*place))),
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
        owner: ModuleId,
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
            dir::TypeOperation::TypeOf(query) => {
                format!("typeof {}", self.format_type_query(query.value))
            }
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
            dir::TypeOperation::TemplateLiteral(template) => {
                let strings = self.template_strings(owner, template.strings)?.to_vec();
                let spans = self.type_ids(owner, template.spans)?.to_vec();
                let mut rendered = String::from("`");
                for (index, segment) in strings.iter().enumerate() {
                    rendered.push_str(&self.text(*segment));
                    if let Some(span) = spans.get(index) {
                        let span = self.format_depth_at(module, *span, depth)?;
                        rendered.push_str(&format!("${{{span}}}"));
                    }
                }
                rendered.push('`');

                rendered
            }
            dir::TypeOperation::Infer(infer) => match infer.name {
                Some(name) => format!("infer {}", self.text(name)),
                None => "infer _".to_string(),
            },
        };

        Ok(rendered)
    }

    /// Format one type query operand.
    fn format_type_query(&self, value: dir::GlobalNodeIdAny) -> String {
        if value.local_id.ty != dir::NodeType::Expression {
            return self.node_label(value);
        }

        let id = value.into_typed::<dir::Expression>().local_id;
        let Some(path) = self.module(value.module_id).view().reference_path(id) else {
            return self.node_label(value);
        };

        path.segments
            .iter()
            .map(|segment| self.text(*segment))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Format one scalar literal type.
    pub(in crate::sema) fn format_scalar_literal(&self, literal: &dir::Literal) -> String {
        match literal {
            dir::Literal::String(value) => format!("\"{}\"", self.text(*value)),
            dir::Literal::Character(value) => format!("'{value}'"),
            dir::Literal::Boolean(value) => value.to_string(),
            dir::Literal::Integer(value) => value.to_string(),
            dir::Literal::Float(value) => value.to_string(),
            dir::Literal::Bigint(value) => format!("{value}n"),
            dir::Literal::Null => "null".to_string(),
            dir::Literal::Undefined => "undefined".to_string(),
            dir::Literal::RegexString { .. } => "regex".to_string(),
        }
    }

    /// Format one exact property key type.
    fn format_key_type(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => format!("\"{}\"", self.text(*name)),
            dir::StaticKey::Index(index) => index.to_string(),
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
        self.format_static_term(self.r#static(value))
    }

    /// Format one static term with its structural payload.
    fn format_static_term(&self, term: &dir::StaticTerm) -> String {
        match term {
            dir::StaticTerm::Literal { value } => self.format_scalar_literal(value),
            dir::StaticTerm::Object { properties } => {
                let fields = properties
                    .iter()
                    .filter_map(dir::StaticProperty::as_field)
                    .map(|(key, value)| {
                        format!(
                            "{}: {}",
                            self.format_static_key(&key),
                            self.format_static_term(value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");

                if fields.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{ {fields} }}")
                }
            }
            dir::StaticTerm::Struct { ty, properties } => {
                let fields = properties
                    .iter()
                    .filter_map(dir::StaticProperty::as_field)
                    .map(|(key, value)| {
                        format!(
                            "{}: {}",
                            self.format_static_key(&key),
                            self.format_static_term(value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");

                if fields.is_empty() {
                    format!("{} {{}}", self.format_type(*ty))
                } else {
                    format!("{} {{ {fields} }}", self.format_type(*ty))
                }
            }
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
    pub(in crate::sema) fn format_types(&self, ids: &[dir::GlobalTypeId]) -> String {
        if ids.is_empty() {
            return "none".to_string();
        }

        ids.iter()
            .map(|id| format!("'{}'", self.format_type(*id)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Format one symbol by its declared name.
    pub(in crate::sema) fn format_symbol(&self, symbol: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table(symbol.module_id);
        let key = bindings.get_symbol(symbol.local_id).key;

        match key {
            Some(key) => self.format_static_key(&key),
            None => "<anonymous>".to_string(),
        }
    }

    /// Format the source path written by one assignment.
    pub(in crate::sema) fn format_assignment_binding(
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
                ..
            } => {
                let left = self.format_assignment_expression(module, *left)?;
                let name = self.format_static_key(&dir::StaticKey::Name(*name));

                Some(format!("{left}.{name}"))
            }
            _ => None,
        }
    }

    /// Format one symbol by its owner-qualified declared name.
    pub(in crate::sema) fn format_symbol_path(&self, symbol: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table(symbol.module_id);
        let mut paths = BTreeMap::new();

        self.format_symbol_path_base(&bindings, symbol.local_id, &mut paths)
    }

    /// Format one symbol path relative to an optional source module.
    fn format_symbol_path_maybe_at(
        &self,
        module: Option<ModuleId>,
        symbol: dir::GlobalSymbolId,
    ) -> String {
        if let Some(name) = self.language_item_symbol_name(symbol) {
            return name;
        }

        if let Some(name) =
            module.and_then(|module| self.visible_global_symbol_name(module, symbol))
        {
            return name;
        }

        let path = self.format_symbol_path(symbol);
        if module.is_some_and(|module| module == symbol.module_id) {
            return path;
        }

        match module {
            Some(_) => format!("{}.{}", self.format_module_label(symbol.module_id), path),
            None => path,
        }
    }

    /// Return the compact language item name for one symbol.
    fn language_item_symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<String> {
        let item = self.environment_bound.language.item(symbol)?;
        let key = item.key();

        if let Some((_, name)) = key.rsplit_once('.') {
            Some(name.to_string())
        } else {
            Some(key)
        }
    }

    /// Return the visible global name for one imported symbol.
    fn visible_global_symbol_name(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<String> {
        let imports = &self.module(module).resolved.imports;

        // use only unambiguous global symbol imports
        imports
            .global_resolution_by_key
            .iter()
            .find_map(|(key, resolutions)| {
                let [resolution] = resolutions.as_slice() else {
                    return None;
                };
                let is_target = resolution
                    .target
                    .symbol_ids()
                    .is_some_and(|symbols| symbols.contains(&symbol));
                if is_target {
                    Some(self.format_static_key(key))
                } else {
                    None
                }
            })
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
            || symbol.kind == dir::SymbolKind::GenericConstParameter
            || symbol.kind == dir::SymbolKind::GenericLifetimeParameter
    }

    /// Format one module as a compact qualifier.
    fn format_module_label(&self, module: ModuleId) -> String {
        if let Some(module) = self.module_maybe(module) {
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

    /// Format one member key.
    pub(in crate::sema) fn format_static_key(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.text(*name),
            dir::StaticKey::Index(index) => index.to_string(),
        }
    }

    /// Format one owner.case variant label.
    pub(in crate::sema) fn format_variant_case(
        &self,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> String {
        let owner = self.format_type(owner);
        let key = self.format_static_key(&key);

        format!("{owner}.{key}")
    }

    /// Return interned text from the shared string pool.
    fn text(&self, id: dir::StringId) -> String {
        self.strings().get(id).to_string()
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
    primitive.as_str()
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
