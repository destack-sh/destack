use destack_dir as dir;
use destack_source::ModuleId;
use std::collections::BTreeMap;
use std::path::Path;

use crate::sema::{CheckState, GenericTemplateId, VariableKind};
use crate::{CompilerError, CompilerResult};

/// The uri scheme of the standard library, whose names print bare.
const LIBRARY_SCHEME: &str = "destack://";

/// The uri scheme of an on-disk module, whose label is its file stem.
const FILE_SCHEME: &str = "file://";

/// The source extension a module label drops.
const MODULE_EXTENSION: &str = ".ds";

impl CheckState<'_> {
    /// Format one type for diagnostics.
    pub(in crate::sema) fn format_type(&self, id: dir::GlobalTypeId) -> String {
        self.format_type_recursive(None, id, &mut Vec::new())
            .unwrap_or_else(|error| unreachable!("check type formatting failed: {error:?}"))
    }

    /// Format one type relative to a source module.
    pub(in crate::sema) fn format_type_at(
        &self,
        module: ModuleId,
        id: dir::GlobalTypeId,
    ) -> String {
        self.format_type_recursive(Some(module), id, &mut Vec::new())
            .unwrap_or_else(|error| unreachable!("check type formatting failed: {error:?}"))
    }

    /// Format one type relative to an optional source module.
    fn format_type_recursive(
        &self,
        module: Option<ModuleId>,
        id: dir::GlobalTypeId,
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        // reject recursive expansion of the same resolved type
        let id = self.shallow_resolve(id)?;
        if active.contains(&id) {
            return Err(CompilerError::Internal {
                message: format!("recursive type formatting: {id:?}"),
            });
        }
        active.push(id);

        // render by the head of the type
        let rendered = match self.ty(id)? {
            dir::Type::Error => "<error>".to_string(),
            dir::Type::Never => "never".to_string(),
            dir::Type::Unknown => "unknown".to_string(),
            dir::Type::Void => "void".to_string(),
            dir::Type::Null => "null".to_string(),
            dir::Type::Undefined => "undefined".to_string(),
            dir::Type::Intrinsic => "intrinsic".to_string(),
            dir::Type::This => "this".to_string(),
            dir::Type::Variable(variable) => match self.infer.variable(variable)?.kind {
                VariableKind::Type | VariableKind::Memory(_) => "_".to_string(),
                VariableKind::Integer => "{integer}".to_string(),
                VariableKind::Float => "{float}".to_string(),
            },
            dir::Type::Erased(_) => "*".to_string(),

            dir::Type::Primitive(primitive) => format_primitive(&primitive),
            dir::Type::Literal(literal) => self.format_scalar_literal(&literal),
            dir::Type::Key(key) => self.format_key_type(&key),
            dir::Type::Region(region) => format!(
                "{} & {}",
                self.format_type_recursive(module, region.extent, active)?,
                self.format_type_recursive(module, region.space, active)?
            ),
            dir::Type::Static(value) => self.format_static(value),
            dir::Type::Range(range) => self.format_range(&range),

            dir::Type::Parameter(parameter) => self.format_parameter(parameter),
            dir::Type::Reference(reference) => {
                let name = self.format_symbol_path_maybe_at(module, reference.symbol);
                let name = if reference.arguments.is_empty() {
                    name
                } else {
                    let arguments = self.type_ids(id.module_id, reference.arguments)?;
                    let arguments = self.format_list_at(module, arguments, active)?;

                    format!("{name}<{arguments}>")
                };
                let bindings = Self::reached(self.binding_table(reference.symbol.module_id));
                if bindings.get_symbol(reference.symbol.local_id).kind == dir::SymbolKind::Class {
                    format!("typeof {name}")
                } else {
                    name
                }
            }
            // render array applications in their written element form
            dir::Type::Application(_) if let Some(element) = self.array_element(id)? => {
                let element = self.format_type_recursive(module, element, active)?;

                format!("{element}[]")
            }
            dir::Type::Application(instance) => {
                let name = self.format_symbol_path_maybe_at(module, instance.symbol);
                if instance.arguments.is_empty() {
                    name
                } else {
                    let arguments = self.type_ids(id.module_id, instance.arguments)?;
                    let rendered = self.format_list_at(module, arguments, active)?;

                    format!("{name}<{rendered}>")
                }
            }
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, member)?;
                let owner = self.format_type_recursive(module, member.owner, active)?;
                let key = self.format_static_key(&member.key);

                format!("{owner}.{key}")
            }
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(id.module_id, refined)?;
                let base = self.format_type_recursive(module, refined.base, active)?;
                let key = self.format_static_key(&refined.key);
                let value = self.format_type_recursive(module, refined.value, active)?;

                format!("{base}<type {key} = {value}>")
            }
            dir::Type::Variant(variant) => {
                self.format_symbol_path_maybe_at(module, variant.variant)
            }

            // render intrinsic collections by their declared names
            dir::Type::Slice(slice) => {
                format!(
                    "Slice<{}>",
                    self.format_type_recursive(module, slice.element, active)?
                )
            }
            dir::Type::FixedArray(array) => {
                let element = self.format_type_recursive(module, array.element, active)?;
                let count = self.format_type_recursive(module, array.count, active)?;

                format!("FixedArray<{element}, {count}>")
            }
            dir::Type::Tuple(tuple) => {
                // retain each element's arity in the displayed tuple
                let elements = self.tuple_elements(id.module_id, tuple.elements)?;
                let mut formatted = Vec::new();
                for element in elements.iter() {
                    let ty = self.format_type_recursive(module, element.ty, active)?;
                    let rest = if element.is_rest { "..." } else { "" };
                    let optional = if element.is_optional { "?" } else { "" };
                    formatted.push(format!("{rest}{ty}{optional}"));
                }

                format!("({})", formatted.join(", "))
            }

            dir::Type::Object(shape) => {
                let object_properties = self.object_properties(id.module_id, shape.properties)?;
                let index_signatures =
                    self.object_index_signatures(id.module_id, shape.index_signatures)?;

                // render each property
                let mut fields = Vec::new();
                for property in object_properties.iter() {
                    let key = self.format_static_key(&property.key);
                    let optional = if property.is_optional { "?" } else { "" };

                    fields.push(match property.access {
                        dir::PropertyAccess::Read(ty) => {
                            let ty = self.format_type_recursive(module, ty, active)?;

                            format!("readonly {key}{optional}: {ty}")
                        }
                        dir::PropertyAccess::Write(ty) => {
                            let ty = self.format_type_recursive(module, ty, active)?;

                            format!("set {key}(value: {ty})")
                        }
                        dir::PropertyAccess::ReadWrite { read, write } if read == write => {
                            let ty = self.format_type_recursive(module, read, active)?;

                            format!("{key}{optional}: {ty}")
                        }
                        dir::PropertyAccess::ReadWrite { read, write } => {
                            let read = self.format_type_recursive(module, read, active)?;
                            let write = self.format_type_recursive(module, write, active)?;

                            format!("get {key}(): {read}; set {key}(value: {write})")
                        }
                    });
                }

                // render each index signature
                for signature in index_signatures.iter() {
                    let readonly = if signature.is_readonly {
                        "readonly "
                    } else {
                        ""
                    };
                    let optional = if signature.is_optional { "?" } else { "" };
                    let name = self.text(signature.name);
                    let key = self.format_type_recursive(module, signature.key_type, active)?;
                    let value = self.format_type_recursive(module, signature.value_type, active)?;

                    fields.push(format!("{readonly}[{name}: {key}]{optional}: {value}"));
                }

                // render an empty shape as braces
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
                for parameter in signature_parameters.iter() {
                    parameters.push(self.format_function_parameter_at(module, parameter, active)?);
                }
                let result = match function.return_type {
                    Some(return_type) => self.format_type_recursive(module, return_type, active)?,
                    None => "void".to_string(),
                };
                let arguments = self.signature_arguments(id.module_id, function.arguments)?;
                let generic = self.format_generic_parameters_at(
                    module,
                    function.template,
                    arguments,
                    active,
                )?;
                let construct = match function.is_construct {
                    true => "new ",
                    false => "",
                };

                format!(
                    "{construct}{generic}({}) => {result}",
                    parameters.join(", ")
                )
            }
            // print the receiver the stdlib elides as the bare signature
            dir::Type::Function(function) => match self.receiver_mode(function.receiver)? {
                mode if mode.is_elided() => {
                    self.format_type_recursive(module, function.signature, active)?
                }
                mode => {
                    let receiver = format!("\"{}\"", mode.text());

                    self.format_callable_application_at(
                        module,
                        "Function",
                        function.signature,
                        Some(&receiver),
                        active,
                    )?
                }
            },
            dir::Type::FunctionPointer(function) => self.format_callable_application_at(
                module,
                "FunctionPointer",
                function.signature,
                None,
                active,
            )?,

            dir::Type::Union(union) => {
                let union_elements = self.type_ids(id.module_id, union.elements)?;

                let mut elements = Vec::new();
                for element in union_elements.iter().copied() {
                    elements.push(self.format_type_recursive(module, element, active)?);
                }

                elements.join(" | ")
            }
            dir::Type::Intersection(intersection) => {
                let intersection_elements = self.type_ids(id.module_id, intersection.elements)?;

                let mut elements = Vec::new();
                for element in intersection_elements.iter().copied() {
                    elements.push(self.format_type_recursive(module, element, active)?);
                }

                elements.join(" & ")
            }

            dir::Type::Form(form) => self.format_form_at(module, id.module_id, &form, active)?,
            dir::Type::Dynamic(dynamic) => {
                format!(
                    "Dynamic<{}>",
                    self.format_type_recursive(module, dynamic.constraint, active)?
                )
            }

            dir::Type::Operation(operation) => {
                let operation = self.type_operation(id.module_id, operation)?;

                self.format_operation_at(module, id.module_id, &operation, active)?
            }
        };

        active.pop();

        Ok(rendered)
    }

    /// Format one type list relative to an optional source module.
    fn format_list_at(
        &self,
        module: Option<ModuleId>,
        ids: &[dir::GlobalTypeId],
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        let mut formatted = Vec::new();
        for id in ids.iter() {
            formatted.push(self.format_type_recursive(module, *id, active)?);
        }

        Ok(formatted.join(", "))
    }

    /// Format one callable's generic parameters relative to an optional source module.
    fn format_generic_parameters_at(
        &self,
        module: Option<ModuleId>,
        template: Option<dir::GlobalGenericTemplateId>,
        arguments: &[dir::GenericArgumentBinding],
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        let Some(template_id) = template else {
            return Ok(String::new());
        };
        let template =
            self.generic_template(template_id)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("callable template {template_id:?} is missing"),
                })?;

        // render declared parameters in template order
        let declared = template.parameters.clone();
        let mut parameters = Vec::new();
        for parameter in declared {
            let parameter = parameter.into_global(template_id.module_id);
            if arguments
                .iter()
                .any(|binding| binding.parameter == parameter)
            {
                continue;
            }
            let binding = self.generic_parameter(parameter)?.cloned().ok_or_else(|| {
                CompilerError::Internal {
                    message: format!("callable parameter {parameter:?} is missing"),
                }
            })?;

            // take the declared parameter name
            let mut label = self.format_parameter(parameter);

            // print tick parameters bare, their kind is implied
            if binding.memory_parameter() == Some(dir::MemoryParameter::Region)
                && label
                    .rsplit('.')
                    .next()
                    .is_some_and(|name| name.starts_with('\''))
            {
                parameters.push(label);
                continue;
            }

            // wrap the written modifiers around the name from inside out
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
                let constraint = self.format_type_recursive(module, constraint, active)?;
                label = format!("{label}: {constraint}");
            }
            if let Some(default) = binding.default {
                let default = self.format_type_recursive(module, default, active)?;
                label = format!("{label} = {default}");
            }
            parameters.push(label);
        }

        // drop the brackets on an empty parameter row
        if parameters.is_empty() {
            return Ok(String::new());
        }

        Ok(format!("<{}>", parameters.join(", ")))
    }

    /// Format one callable application relative to an optional source module.
    fn format_callable_application_at(
        &self,
        module: Option<ModuleId>,
        head: &str,
        signature: dir::GlobalTypeId,
        tail: Option<&str>,
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        // read the signature the callable applies
        let signature_id = self.shallow_resolve(signature)?;
        let Some(signature) = self.signature_head(signature_id)? else {
            let signature = self.format_type_recursive(module, signature, active)?;

            return Ok(match tail {
                Some(tail) => format!("{head}<{signature}, {tail}>"),
                None => format!("{head}<{signature}>"),
            });
        };
        let signature_parameters =
            self.signature_parameters(signature_id.module_id, signature.parameters)?;

        // render each parameter
        let mut parameters = Vec::new();
        for parameter in signature_parameters {
            let parameter = self.format_function_parameter_at(module, parameter, active)?;

            parameters.push(parameter);
        }

        // render a lone parameter with a trailing comma
        let parameters = match parameters.as_slice() {
            [] => "()".to_string(),
            [parameter] => format!("({parameter},)"),
            _ => format!("({})", parameters.join(", ")),
        };
        let result = match signature.return_type {
            Some(return_type) => self.format_type_recursive(module, return_type, active)?,
            None => "void".to_string(),
        };

        // render the signature with its tail argument
        Ok(match tail {
            Some(tail) => format!("{head}<{parameters}, {result}, {tail}>"),
            None => format!("{head}<{parameters}, {result}>"),
        })
    }

    /// Format one function signature parameter relative to an optional source module.
    fn format_function_parameter_at(
        &self,
        module: Option<ModuleId>,
        parameter: &dir::FunctionParameterType,
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        // render the parameter type, spreading a rest parameter
        let parameter_type = self.format_type_recursive(module, parameter.ty, active)?;
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
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        // group a payload that binds looser than the form
        let payload = self.resolved_ty(form.value)?;
        let value = self.format_type_recursive(module, form.value, active)?;
        let is_grouped = payload.needs_parentheses(
            dir::TypeOperand::Prefix,
            |id| self.type_operation(form.value.module_id, id),
            |receiver| self.receiver_mode(receiver).map(Some),
        )?;
        let value = match is_grouped {
            true => format!("({value})"),
            false => value,
        };

        // render by the form's own constructor
        let rendered = match &form.form {
            dir::Form::Owned => format!("^{value}"),
            dir::Form::Raw => format!("*{value}"),
            dir::Form::Readonly => format!("readonly {value}"),
            dir::Form::Borrowed(borrow) => {
                let borrow = self.type_borrow(owner, *borrow)?;
                let (region, access) = dir::read_borrow(
                    &borrow,
                    |id| self.resolved_ty(id),
                    |text| self.strings().get(text).to_string(),
                    |parameter| Ok(self.format_parameter(parameter)),
                )?;
                match dir::borrow_text(region.as_ref(), access.as_ref(), &value) {
                    Some(text) => text,
                    None => {
                        let region = self.format_type(borrow.region);
                        let access = self.format_type(borrow.access);

                        format!("Borrowed<{value}, {region}, {access}>")
                    }
                }
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
        active: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<String> {
        // render by the operation kind
        let rendered = match operation {
            dir::TypeOperation::Conditional(conditional) => format!(
                "{} extends {} ? {} : {}",
                self.format_type_recursive(module, conditional.left, active)?,
                self.format_type_recursive(module, conditional.right, active)?,
                self.format_type_recursive(module, conditional.then_type, active)?,
                self.format_type_recursive(module, conditional.else_type, active)?,
            ),
            dir::TypeOperation::Narrow(narrow) => {
                let source = self.format_type_recursive(module, narrow.source, active)?;
                let target = self.format_type_recursive(module, narrow.target, active)?;
                if narrow.is_positive {
                    format!("Narrow<{source}, {target}>")
                } else {
                    format!("Narrow<{source}, !{target}>")
                }
            }
            dir::TypeOperation::KeyOf(unary) => {
                format!(
                    "keyof {}",
                    self.format_type_recursive(module, unary.target, active)?
                )
            }
            dir::TypeOperation::NoInfer(unary) => {
                format!(
                    "NoInfer<{}>",
                    self.format_type_recursive(module, unary.target, active)?
                )
            }
            dir::TypeOperation::Awaited(unary) => {
                format!(
                    "Awaited<{}>",
                    self.format_type_recursive(module, unary.target, active)?
                )
            }
            dir::TypeOperation::SpaceOf(unary) => {
                format!(
                    "SpaceOf<{}>",
                    self.format_type_recursive(module, unary.target, active)?
                )
            }
            dir::TypeOperation::Index(index) => format!(
                "{}[{}]",
                self.format_type_recursive(module, index.left, active)?,
                self.format_type_recursive(module, index.index, active)?,
            ),
            dir::TypeOperation::TypeOf(query) => {
                format!("typeof {}", self.format_symbol(query.symbol))
            }
            dir::TypeOperation::Instantiation(application) => {
                let target = self.format_type_recursive(module, application.target, active)?;
                let mut arguments = Vec::new();
                for argument in self.type_ids(owner, application.arguments)? {
                    arguments.push(self.format_type_recursive(module, *argument, active)?);
                }
                let arguments = arguments.join(", ");

                format!("{target}<{arguments}>")
            }
            dir::TypeOperation::StaticBinary(binary) => format!(
                "{} {} {}",
                self.format_type_recursive(module, binary.left, active)?,
                format_static_binary_operator(binary.operator),
                self.format_type_recursive(module, binary.right, active)?,
            ),
            dir::TypeOperation::StaticUnary(unary) => {
                let operator = match unary.operator {
                    dir::StaticUnaryOperator::Not => "!",
                    dir::StaticUnaryOperator::Negate => "-",
                    dir::StaticUnaryOperator::BitwiseNot => "~",
                };

                format!(
                    "{operator}{}",
                    self.format_type_recursive(module, unary.target, active)?
                )
            }
            dir::TypeOperation::TryOutput { value } => {
                format!(
                    "Output<{}>",
                    self.format_type_recursive(module, *value, active)?
                )
            }
            dir::TypeOperation::TryResidual { value } => {
                format!(
                    "Residual<{}>",
                    self.format_type_recursive(module, *value, active)?
                )
            }
            dir::TypeOperation::TryFailure { value } => {
                format!(
                    "Failure<{}>",
                    self.format_type_recursive(module, *value, active)?
                )
            }
            dir::TypeOperation::StringMapping { target, .. } => {
                self.format_type_recursive(module, *target, active)?
            }
            dir::TypeOperation::Mapped(_) => "{ [mapped] }".to_string(),
            dir::TypeOperation::TemplateLiteral(template) => {
                let strings = self.template_strings(owner, template.strings)?;
                let spans = self.type_ids(owner, template.spans)?;
                let mut rendered = String::from("`");
                for (index, segment) in strings.iter().enumerate() {
                    rendered.push_str(&self.text(*segment));
                    if let Some(span) = spans.get(index) {
                        let span = self.format_type_recursive(module, *span, active)?;
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

    /// Format one scalar literal type.
    pub(in crate::sema) fn format_scalar_literal(&self, literal: &dir::Literal) -> String {
        // render each literal in its written form
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

    /// Unwrap one read of a module the pass reached before formatting it.
    fn reached<T>(read: CompilerResult<T>) -> T {
        read.unwrap_or_else(|error| {
            unreachable!("formatting reads a module the pass has not reached: {error:?}")
        })
    }

    /// Format one committed static value.
    fn format_static(&self, value: dir::GlobalStaticId) -> String {
        self.format_static_term(Self::reached(self.r#static(value)))
    }

    /// Format one static term with its structural payload.
    fn format_static_term(&self, term: &dir::StaticTerm) -> String {
        // render each static term in its written form
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

    /// Return the region names one template and its owners declare ahead of one parameter.
    fn region_names_in_scope(
        &self,
        template: GenericTemplateId,
        parameter: dir::GlobalGenericParameterId,
    ) -> Vec<String> {
        // walk the owner templates outermost first, this template last
        let mut templates = vec![template];
        let mut current = Self::reached(self.parent_generic_template(template));
        while let Some(owner) = current {
            templates.push(owner);
            current = Self::reached(self.parent_generic_template(owner));
        }
        templates.reverse();

        // name each region in scope order, stopping at the parameter itself
        let mut names = Vec::new();
        for template in templates {
            let Some(declared) = Self::reached(self.generic_template(template)) else {
                continue;
            };
            for candidate in declared.parameters.clone() {
                let candidate = candidate.into_global(template.module_id);
                if candidate == parameter {
                    return names;
                }
                let Some(binding) = Self::reached(self.generic_parameter(candidate)) else {
                    continue;
                };
                if binding.memory_parameter() != Some(dir::MemoryParameter::Region) {
                    continue;
                }
                let name = match binding.key {
                    dir::GenericParameterKey::Symbol(symbol) => self.format_symbol(symbol),
                    dir::GenericParameterKey::Anonymous => {
                        dir::free_region_name(names.iter().map(String::as_str))
                    }
                };
                names.push(name);
            }
        }

        names
    }

    /// Format one generic parameter by its declared name.
    pub(in crate::sema) fn format_parameter(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> String {
        let Some(binding) = Self::reached(self.generic_parameter(parameter)) else {
            return "_".to_string();
        };

        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => self.format_symbol(symbol),
            dir::GenericParameterKey::Anonymous => {
                let template = binding.template.into_global(parameter.module_id);
                let position = Self::reached(self.generic_template(template))
                    .and_then(|template| {
                        template
                            .parameters
                            .iter()
                            .position(|candidate| *candidate == parameter.local_id)
                    })
                    .unwrap_or_else(|| {
                        unreachable!("parameter {parameter:?} is missing from its template")
                    });
                let regions_in_scope = self.region_names_in_scope(template, parameter);

                binding.canonical_name(position, regions_in_scope.iter().map(String::as_str))
            }
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
        let bindings = Self::reached(self.binding_table(symbol.module_id));
        let key = bindings.get_symbol(symbol.local_id).key;

        // render each key in its written form
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
        let mut paths = BTreeMap::new();

        self.format_symbol_path_base(symbol.module_id, symbol.local_id, &mut paths)
    }

    fn format_symbol_path_maybe_at(
        &self,
        module: Option<ModuleId>,
        symbol: dir::GlobalSymbolId,
    ) -> String {
        // prefer the compact language item name
        if let Some(name) = self.language_item_symbol_name(symbol) {
            return name;
        }

        // prefer the name the source module imports
        if let Some(name) =
            module.and_then(|module| self.visible_global_symbol_name(module, symbol))
        {
            return name;
        }

        // render own-module symbols by their path alone
        let path = self.format_symbol_path(symbol);
        if module.is_some_and(|module| module == symbol.module_id) {
            return path;
        }

        // qualify foreign symbols with their module label, library names staying bare
        match module {
            Some(_) if !self.is_library_module(symbol.module_id) => {
                format!("{}.{}", self.format_module_label(symbol.module_id), path)
            }
            _ => path,
        }
    }

    /// Return whether one module belongs to the standard library, which owns bare names.
    fn is_library_module(&self, module: ModuleId) -> bool {
        if let Some(module) = self.module_maybe(module) {
            return module.module.uri.as_ref().starts_with(LIBRARY_SCHEME);
        }

        // read the module from the repository
        if let Ok(Some(module)) = self
            .compiler
            .repository
            .module(self.context.revision(), module)
        {
            return module.uri.as_ref().starts_with(LIBRARY_SCHEME);
        }

        false
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

    /// Format one symbol's path by climbing its owners, reusing the paths already rendered.
    fn format_symbol_path_base(
        &self,
        module: ModuleId,
        symbol: dir::LocalSymbolId,
        paths: &mut BTreeMap<dir::LocalSymbolId, String>,
    ) -> String {
        // reuse the path this symbol already rendered
        if let Some(path) = paths.get(&symbol) {
            return path.clone();
        }

        // read the symbol and its owner, keeping bare symbols and root scopes at their label
        let (is_qualified, owner) = {
            let bindings = Self::reached(self.binding_table(module));
            let entry = bindings.get_symbol(symbol);
            let is_qualified = Self::is_qualified_symbol(entry);
            let owner = bindings
                .get_scope_by_id(entry.scope.id)
                .owner
                .filter(|owner| {
                    let owner = bindings.get_symbol(*owner);
                    owner.role != dir::SymbolRole::Namespace || owner.name().is_some()
                });
            (is_qualified, owner)
        };
        let label = self.format_symbol(dir::GlobalSymbolId {
            module_id: module,
            local_id: symbol,
        });
        let Some(owner) = owner.filter(|_| is_qualified) else {
            paths.insert(symbol, label.clone());

            return label;
        };

        // qualify the label with its owner's path
        let owner = self.format_symbol_path_base(module, owner, paths);
        let path = format!("{owner}.{label}");
        paths.insert(symbol, path.clone());

        path
    }

    /// Return whether one symbol should be owner-qualified.
    fn is_qualified_symbol(symbol: &dir::Symbol) -> bool {
        symbol.role == dir::SymbolRole::Item
            || symbol.role == dir::SymbolRole::Namespace
            || symbol.kind == dir::SymbolKind::TypeAlias
            || symbol.kind == dir::SymbolKind::GenericTypeParameter
            || symbol.kind == dir::SymbolKind::GenericConstParameter
            || symbol.kind == dir::SymbolKind::GenericLifetimeParameter
    }

    /// Format one module as a compact qualifier.
    pub(in crate::sema) fn format_module_label(&self, module: ModuleId) -> String {
        if let Some(module) = self.module_maybe(module) {
            return trim_module_uri(module.module.uri.as_ref());
        }

        // read the module from the repository
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
    // take the file stem of a file uri
    if let Some(path) = uri.strip_prefix(FILE_SCHEME) {
        return Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_string();
    }

    // fold the remaining uri into a dotted label
    let uri = uri.strip_prefix(LIBRARY_SCHEME).unwrap_or(uri);
    let uri = uri.strip_suffix(MODULE_EXTENSION).unwrap_or(uri);
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
