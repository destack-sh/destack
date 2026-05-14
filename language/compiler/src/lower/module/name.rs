use std::collections::HashSet;
use {destack_dir as dir, destack_mir as mir};

use destack_core::{StringId, StringPool, stable_hash_text};
use destack_workspace::{Module, Package};

use crate::lower::{ModuleLowerer, static_key_from_key};
use crate::{LowerError, LowerResult};

/// Suffix for object metadata names.
const OBJECT_METADATA_SUFFIX: &str = ".object";
/// Suffix for tuple metadata names.
const TUPLE_METADATA_SUFFIX: &str = "#tuple";
/// Suffix for union metadata names.
const UNION_METADATA_SUFFIX: &str = "#union";
/// Suffix for intersection metadata names.
const INTERSECTION_METADATA_SUFFIX: &str = "#intersection";
/// Suffix for function metadata names.
const FUNCTION_METADATA_SUFFIX: &str = ".function";
/// Suffix for array metadata names.
const ARRAY_METADATA_SUFFIX: &str = "#array";
/// Suffix for class reference metadata names.
const REFERENCE_METADATA_SUFFIX: &str = "#reference";
/// Suffix for return types in union metadata names.
const UNION_RETURN_SUFFIX: &str = ".return";
/// Prefix for intrinsic metadata names.
const INTRINSIC_METADATA_PREFIX: &str = "intrinsic:";
/// Prefix for scalar literal metadata names.
const LITERAL_METADATA_PREFIX: &str = "literal:";
/// Prefix for regex literal metadata names.
const REGEX_METADATA_PREFIX: &str = "regex:";
/// Prefix for string literal metadata names.
const STRING_METADATA_PREFIX: &str = "string:";
/// Prefix for string literal global names.
const STRING_LITERAL_GLOBAL_PREFIX: &str = "stringLiteral";
/// Maximum length of string literal slugs.
const STRING_LITERAL_SLUG_MAX: usize = 32;

/// Convert a static key into a canonical string.
fn static_key_string(key: &dir::StaticKey, strings: &StringPool) -> String {
    match key {
        dir::StaticKey::Name(name_id) | dir::StaticKey::Number(name_id) => {
            strings.get(*name_id).to_string()
        }
        dir::StaticKey::Symbol(symbol_key) => match symbol_key {
            dir::SymbolKey::Registry(name_id) => {
                let name = strings.get(*name_id);
                format!("@Symbol.for({name})")
            }
            dir::SymbolKey::Unique(global_id) => format!("@Symbol#{global_id:?}"),
        },
    }
}

/// Context derived from a type source node for naming.
#[derive(Default)]
struct TypeNameContext {
    /// The symbol owning the type context.
    declaration_symbol: Option<dir::GlobalSymbolId>,
    /// The parameter name when the type is attached to a parameter.
    parameter_name: Option<String>,
    /// The field name when the type is attached to a field.
    field_name: Option<String>,
    /// The method name when the type is attached to a method signature.
    method_name: Option<String>,
    /// The declarator binding name when the type is attached to a local.
    declarator_name: Option<String>,
    /// Whether this type is a return type.
    is_return_type: bool,
    /// Whether this type is the value of a type alias declaration.
    is_type_alias: bool,
}

/// Names derived from a nominal type symbol.
struct NominalMetadataNames {
    /// The metadata name for reference types, when available.
    reference: Option<StringId>,
    /// The metadata name for instance types, when available.
    instance: Option<StringId>,
}

impl ModuleLowerer<'_> {
    /// Return a metadata name for a lowered type.
    pub(crate) fn metadata_name_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<StringId> {
        let dir_type = self.types.get_type(type_id);

        // skip if metadata already exists
        if let Some(name) = self.builder.tree().metadata.types.display_name(mir_type) {
            return Ok(name);
        }

        // use nominal naming when the type resolves to a symbol
        if let dir::Type::Reference(reference) = dir_type
            && matches!(
                self.symbol_form(reference.symbol),
                Some(
                    dir::SymbolForm::Struct
                        | dir::SymbolForm::Class
                        | dir::SymbolForm::Interface
                        | dir::SymbolForm::Enum
                )
            )
        {
            let names = self.metadata_names_for_symbol(reference.symbol, anchor)?;
            let reference_name = names.reference.ok_or_else(|| LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "missing reference metadata name for nominal type".to_string(),
            })?;

            // attach metadata for the current reference type when missing
            self.builder
                .tree_mut()
                .metadata
                .types
                .ensure_display_name(mir_type, reference_name);

            return Ok(reference_name);
        }

        if let Some(symbol) = self.types.symbol_for_instance_type(type_id)
            && matches!(
                self.symbol_form(symbol),
                Some(
                    dir::SymbolForm::Struct
                        | dir::SymbolForm::Class
                        | dir::SymbolForm::Interface
                        | dir::SymbolForm::Enum
                )
            )
        {
            let names = self.metadata_names_for_symbol(symbol, anchor)?;
            let instance_name = names.instance.ok_or_else(|| LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "missing instance metadata name for nominal type".to_string(),
            })?;
            self.builder
                .tree_mut()
                .metadata
                .types
                .ensure_display_name(mir_type, instance_name);
            return Ok(instance_name);
        }

        // resolve the suffix for anonymous types
        let name = if let Some(suffix) = self.anonymous_metadata_suffix(dir_type) {
            self.anonymous_metadata_name_for_type(type_id, dir_type, suffix)
        } else {
            self.default_metadata_name(type_id, dir_type)
        };
        let Some(name) = name else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: format!("missing metadata name for type {type_id:?} ({dir_type:?})"),
            }
            .into());
        };

        // intern the name
        let name_id = self.builder.intern(&name);

        // attach the metadata name
        self.builder
            .tree_mut()
            .metadata
            .types
            .ensure_display_name(mir_type, name_id);

        Ok(name_id)
    }

    /// Build a metadata name for a function type when no context is available.
    fn function_type_metadata_name(&self, type_id: dir::LocalTypeId) -> Option<String> {
        let dir::Type::Function(function) = self.types.get_type(type_id) else {
            return None;
        };

        // start with the function prefix
        let mut name = String::from("fn");

        // record the implicit this parameter when present
        if let Some(this_parameter) = function.this_parameter {
            let this_name = self
                .metadata_base_name_for_type(this_parameter)
                .unwrap_or_else(|| "unknown".to_string());
            name.push_str(".this.");
            name.push_str(&this_name);
        }

        // record dynamic parameter names
        for parameter in &function.parameters {
            let param_name = self
                .metadata_base_name_for_type(*parameter)
                .unwrap_or_else(|| "unknown".to_string());
            name.push('.');
            name.push_str(&param_name);
        }

        // record the return type
        let return_name = function
            .return_type
            .and_then(|return_type| self.metadata_base_name_for_type(return_type))
            .unwrap_or_else(|| "void".to_string());
        name.push_str(".to.");
        name.push_str(&return_name);

        Some(name)
    }

    /// Resolve a metadata name for anonymous types or fall back to generic names.
    fn anonymous_metadata_name_for_type(
        &self,
        type_id: dir::LocalTypeId,
        dir_type: &dir::Type,
        suffix: &str,
    ) -> Option<String> {
        // prefer contextual anonymous metadata names when available
        if let Some(name) = self.anonymous_metadata_name(type_id, suffix) {
            return Some(name);
        }

        // fall back to reference and function metadata names
        if let Some(name) = self.fallback_reference_metadata_name(dir_type) {
            return Some(name);
        }
        if let Some(name) = self.function_type_metadata_name(type_id) {
            return Some(name);
        }

        // fall back to joined union or intersection names without context
        self.union_intersection_metadata_name(dir_type)
    }

    /// Resolve default metadata names for non-anonymous types.
    fn default_metadata_name(
        &self,
        type_id: dir::LocalTypeId,
        dir_type: &dir::Type,
    ) -> Option<String> {
        // resolve reference names directly
        if let Some(name) = self.reference_metadata_name(dir_type) {
            return Some(name);
        }

        // resolve aliases and type literal names
        if let Some(name) = self.alias_metadata_name(type_id) {
            return Some(name);
        }
        if let Some(name) = self.type_literal_metadata_name(dir_type) {
            return Some(name);
        }

        // fall back to joined union or intersection names without context
        self.union_intersection_metadata_name(dir_type)
    }

    /// Return metadata names for a nominal symbol.
    fn metadata_names_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<NominalMetadataNames> {
        // resolve the qualified metadata name
        let name = self
            .qualified_symbol_name(symbol)
            .or_else(|| {
                let dir = self.dir_bound_if_present(symbol.module_id)?;
                self.symbol_path_from_symbols(symbol, &dir.bindings)
            })
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "missing qualified name for nominal type".to_string(),
            })?;
        let name_id = self.builder.intern(&name);
        let mut names = NominalMetadataNames {
            reference: None,
            instance: None,
        };
        names.reference = Some(name_id);

        // resolve Any and instance types separately
        if self.symbol_is(symbol, dir::SymbolForm::Interface) {
            let instance_name = format!("{name}{OBJECT_METADATA_SUFFIX}");
            let instance_name_id = self.builder.intern(&instance_name);
            names.reference = Some(name_id);
            names.instance = Some(instance_name_id);

            if let Some(reference_type_id) = self.nominal_reference_type_id_for_symbol(symbol)
                && let Some(mir_type) = self.type_lowerer.cached_type(reference_type_id)
            {
                self.builder
                    .tree_mut()
                    .metadata
                    .types
                    .ensure_display_name(mir_type, name_id);
            }

            if let Some(instance_type_id) = self.types.get_instance_type_id(symbol)
                && let Some(mir_type) = self.type_lowerer.cached_type(instance_type_id)
            {
                self.builder
                    .tree_mut()
                    .metadata
                    .types
                    .ensure_display_name(mir_type, instance_name_id);
            }

            return Ok(names);
        }

        // resolve class reference types separately
        if self.symbol_is(symbol, dir::SymbolForm::Class)
            && let Some(reference_type_id) = self.nominal_reference_type_id_for_symbol(symbol)
            && let Some(mir_type) = self.type_lowerer.cached_type(reference_type_id)
        {
            let reference_name = format!("{name}{REFERENCE_METADATA_SUFFIX}");
            let reference_name_id = self.builder.intern(&reference_name);
            names.reference = Some(reference_name_id);
            self.builder
                .tree_mut()
                .metadata
                .types
                .ensure_display_name(mir_type, reference_name_id);
        }

        // resolve the instance type id to attach metadata
        let Some(instance_type_id) = self.types.get_instance_type_id(symbol) else {
            return Ok(names);
        };
        let Some(mir_type) = self.type_lowerer.cached_type(instance_type_id) else {
            names.instance = Some(name_id);
            return Ok(names);
        };

        self.builder
            .tree_mut()
            .metadata
            .types
            .ensure_display_name(mir_type, name_id);

        names.instance = Some(name_id);

        Ok(names)
    }

    /// Assign a metadata name for a function signature type.
    pub(crate) fn assign_signature_metadata_name(
        &mut self,
        mir_type: mir::LocalNodeId<mir::Type>,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // resolve the qualified symbol name
        let base = self
            .qualified_symbol_name_for_global(symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "missing qualified name for signature type".to_string(),
            })?;

        // format the signature metadata name
        let name = format!("{base}{FUNCTION_METADATA_SUFFIX}");
        let name_id = self.builder.intern(&name);

        // attach metadata when missing
        self.builder
            .tree_mut()
            .metadata
            .types
            .ensure_display_name(mir_type, name_id);
        Ok(())
    }

    /// Resolve the metadata suffix for an anonymous type.
    fn anonymous_metadata_suffix(&self, dir_type: &dir::Type) -> Option<&'static str> {
        // select a suffix based on the type kind
        match dir_type {
            dir::Type::Object(_) => Some(OBJECT_METADATA_SUFFIX),
            dir::Type::Tuple(_) => Some(TUPLE_METADATA_SUFFIX),
            dir::Type::Union(_) => Some(UNION_METADATA_SUFFIX),
            dir::Type::Intersection(_) => Some(INTERSECTION_METADATA_SUFFIX),
            dir::Type::Function(_) => Some(FUNCTION_METADATA_SUFFIX),
            dir::Type::FixedArray(_) | dir::Type::Slice(_) => Some(ARRAY_METADATA_SUFFIX),
            _ => None,
        }
    }

    /// Build the metadata name for an anonymous type from its source context.
    fn anonymous_metadata_name(&self, type_id: dir::LocalTypeId, suffix: &str) -> Option<String> {
        // resolve the naming context
        let context = self.type_name_context(type_id)?;
        let base = self.qualified_symbol_name(context.declaration_symbol?)?;

        // start with the base path
        let mut name = base;

        // return the alias name without extra context
        if context.is_type_alias {
            return Some(name);
        }

        // parameter name
        if let Some(parameter_name) = context.parameter_name {
            name.push('.');
            name.push_str(&parameter_name);
        }
        // field name
        else if let Some(field_name) = context.field_name {
            name.push('.');
            name.push_str(&field_name);
        }
        // method name
        else if let Some(method_name) = context.method_name {
            name.push('.');
            name.push_str(&method_name);
            // return marker
            if context.is_return_type {
                name.push_str(UNION_RETURN_SUFFIX);
            }
        }
        // return marker for declaration return types
        else if context.is_return_type {
            name.push_str(UNION_RETURN_SUFFIX);
        }
        // local name
        else if let Some(declarator_name) = context.declarator_name {
            name.push_str(".local.");
            name.push_str(&declarator_name);
        }

        // type suffix
        name.push_str(suffix);

        Some(name)
    }

    /// Resolve the metadata name for a type alias value.
    fn alias_metadata_name(&self, type_id: dir::LocalTypeId) -> Option<String> {
        // resolve naming context
        let context = self.type_name_context(type_id)?;
        if !context.is_type_alias {
            return None;
        }

        // return the qualified alias name
        self.qualified_symbol_name(context.declaration_symbol?)
    }

    /// Resolve a metadata name for scalar and literal types.
    fn type_literal_metadata_name(&self, dir_type: &dir::Type) -> Option<String> {
        // only handle type literal nodes
        let dir::Type::Literal(value) = dir_type else {
            return None;
        };

        match value {
            dir::LiteralType::Never => Some("never".to_string()),
            dir::LiteralType::Any => Some("any".to_string()),
            dir::LiteralType::Infer => Some("_".to_string()),
            dir::LiteralType::Undefined => Some("undefined".to_string()),
            dir::LiteralType::Unknown => Some("unknown".to_string()),
            dir::LiteralType::Object => Some("object".to_string()),
            dir::LiteralType::Void => Some("void".to_string()),
            dir::LiteralType::Null => Some("null".to_string()),
            dir::LiteralType::Primitive(primitive) => {
                Some(self.primitive_metadata_name(*primitive))
            }
            dir::LiteralType::Intrinsic(intrinsic) => {
                Some(self.intrinsic_metadata_name(*intrinsic))
            }
            dir::LiteralType::ScalarLiteral(literal) => {
                Some(self.scalar_literal_metadata_name(literal))
            }
        }
    }

    /// Resolve a metadata name for named reference types.
    fn reference_metadata_name(&self, dir_type: &dir::Type) -> Option<String> {
        // only handle reference nodes
        let dir::Type::Reference(reference) = dir_type else {
            return None;
        };

        let name = self.qualified_symbol_name(reference.symbol).or_else(|| {
            let dir = self.dir_bound_if_present(reference.symbol.module_id)?;
            self.symbol_path_from_symbols(reference.symbol, &dir.bindings)
        })?;

        if self.symbol_is(reference.symbol, dir::SymbolForm::Class) {
            return Some(format!("{name}{REFERENCE_METADATA_SUFFIX}"));
        }

        Some(name)
    }

    /// Resolve a metadata name for value or borrowed reference types without context.
    fn fallback_reference_metadata_name(&self, dir_type: &dir::Type) -> Option<String> {
        // resolve the referenced type id
        let dir::Type::Form(form) = dir_type else {
            return None;
        };

        // use the base metadata name when possible
        let base_name = self.metadata_base_name_for_type(form.base)?;
        Some(format!("{base_name}{REFERENCE_METADATA_SUFFIX}"))
    }

    /// Resolve the base metadata name for a type id without context.
    fn metadata_base_name_for_type(&self, type_id: dir::LocalTypeId) -> Option<String> {
        let dir_type = self.types.get_type(type_id);

        // unwrap type-as-value nodes
        if let dir::Type::Value(value) = dir_type {
            return self.metadata_base_name_for_type(value.value);
        }

        // use nominal names without suffix adjustments
        if let dir::Type::Reference(reference) = dir_type {
            return self.qualified_symbol_name(reference.symbol).or_else(|| {
                let dir = self.dir_bound_if_present(reference.symbol.module_id)?;
                self.symbol_path_from_symbols(reference.symbol, &dir.bindings)
            });
        }

        // fall back to aliases and literal metadata names
        self.alias_metadata_name(type_id)
            .or_else(|| self.type_literal_metadata_name(dir_type))
    }

    /// Resolve a fallback metadata name for union and intersection types.
    fn union_intersection_metadata_name(&self, dir_type: &dir::Type) -> Option<String> {
        // select the join separator and suffix
        let (elements, separator, suffix) = match dir_type {
            dir::Type::Union(union) => (&union.elements, "|", UNION_METADATA_SUFFIX),
            dir::Type::Intersection(intersection) => {
                (&intersection.elements, "&", INTERSECTION_METADATA_SUFFIX)
            }
            _ => return None,
        };

        // format element names
        let mut element_names = Vec::with_capacity(elements.len());
        for element in elements {
            let name = self.metadata_base_name_for_type(*element)?;
            element_names.push(name);
        }

        // join and suffix the name
        let joined = element_names.join(separator);
        Some(format!("{joined}{suffix}"))
    }

    /// Resolve a metadata name for primitive types.
    fn primitive_metadata_name(&self, primitive: dir::PrimitiveType) -> String {
        match primitive {
            dir::PrimitiveType::Boolean => "bool".to_string(),
            dir::PrimitiveType::Character => "char".to_string(),
            dir::PrimitiveType::String => "string".to_string(),
            dir::PrimitiveType::Bigint => "bigint".to_string(),
            dir::PrimitiveType::Integer(int_type) => int_type.as_str(),
            dir::PrimitiveType::Float(float_type) => float_type.as_str().to_string(),
            dir::PrimitiveType::Symbol => "symbol".to_string(),
            dir::PrimitiveType::UniqueSymbol => "unique_symbol".to_string(),
        }
    }

    /// Resolve a metadata name for intrinsic types.
    fn intrinsic_metadata_name(&self, intrinsic: dir::IntrinsicType) -> String {
        let suffix = match intrinsic {
            dir::IntrinsicType::Uppercase => "uppercase",
            dir::IntrinsicType::Lowercase => "lowercase",
            dir::IntrinsicType::Capitalize => "capitalize",
            dir::IntrinsicType::Uncapitalize => "uncapitalize",
            dir::IntrinsicType::NoInfer => "no_infer",
            dir::IntrinsicType::BuiltinIteratorReturn => "builtin_iterator_return",
        };

        format!("{INTRINSIC_METADATA_PREFIX}{suffix}")
    }

    /// Resolve a metadata name for scalar literal types.
    fn scalar_literal_metadata_name(&self, literal: &dir::ScalarLiteral) -> String {
        // load string pool for literal formatting
        let strings = &self.strings;

        match literal {
            dir::ScalarLiteral::Null => {
                format!("{LITERAL_METADATA_PREFIX}null")
            }
            dir::ScalarLiteral::Boolean(value) => {
                format!("{LITERAL_METADATA_PREFIX}bool:{value}")
            }
            dir::ScalarLiteral::Integer(value) => {
                format!("{LITERAL_METADATA_PREFIX}int:{value}")
            }
            dir::ScalarLiteral::Bigint(value) => {
                format!("{LITERAL_METADATA_PREFIX}bigint:{value}")
            }
            dir::ScalarLiteral::Float(value) => {
                format!("{LITERAL_METADATA_PREFIX}float:{value}")
            }
            dir::ScalarLiteral::Character(value) => {
                format!("{LITERAL_METADATA_PREFIX}char:U+{:04X}", *value as u32)
            }
            dir::ScalarLiteral::String(value) => {
                let content = strings.get(*value);
                let content = content.as_str();
                let content = self.escape_metadata_fragment(content);
                format!("{LITERAL_METADATA_PREFIX}{STRING_METADATA_PREFIX}{content}")
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = strings.get(*content);
                let content = content.as_str();
                let content = self.escape_metadata_fragment(content);
                let mut name = format!("{LITERAL_METADATA_PREFIX}{REGEX_METADATA_PREFIX}{content}");
                if let Some(flags) = flags {
                    let flags = strings.get(*flags);
                    let flags = flags.as_str();
                    let flags = self.escape_metadata_fragment(flags);
                    name.push_str(":flags:");
                    name.push_str(&flags);
                }
                name
            }
        }
    }

    /// Build a synthetic global name for a string literal.
    pub(crate) fn string_literal_global_name(&self, literal_id: StringId) -> String {
        let literal = self.strings.get(literal_id);
        string_literal_global_name_for_content(literal.as_ref())
    }

    /// Escape metadata fragments to avoid separator collisions.
    fn escape_metadata_fragment(&self, value: &str) -> String {
        let mut escaped = String::new();
        for byte in value.as_bytes() {
            let ch = *byte as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                escaped.push(ch);
            } else {
                escaped.push('%');
                escaped.push_str(&format!("{byte:02X}"));
            }
        }
        escaped
    }

    /// Collect naming context from a type source node.
    fn type_name_context(&self, type_id: dir::LocalTypeId) -> Option<TypeNameContext> {
        // resolve the source node and tracked type node
        let source = self.types.get_type_source(type_id);
        let source_id = Some(source);

        // seed context and traversal cursor
        let mut context = TypeNameContext::default();
        let mut current = Some(source);

        // walk parent nodes for naming context
        while let Some(node_id) = current {
            // match parent node kinds
            match node_id.ty {
                // handle declaration expressions
                dir::NodeType::Expression => {
                    let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
                    let expression = self.dir_tree.get(expression_id);
                    if let dir::Expression::Declaration(declaration_id) = expression {
                        if context.declaration_symbol.is_none() {
                            context.declaration_symbol = self.symbol_for_node(*declaration_id);
                        }

                        let declaration = self.dir_tree.get(*declaration_id);

                        // apply declaration naming context
                        self.apply_declaration_context(declaration, source_id, &mut context);
                    }
                }
                // handle parameter nodes
                dir::NodeType::Parameter => {
                    // fill parameter context when missing
                    if context.parameter_name.is_none() || context.declaration_symbol.is_none() {
                        // load the parameter node
                        let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(node_id.id);
                        let parameter = self.dir_tree.get(parameter_id);

                        // fill parameter name when missing
                        if context.parameter_name.is_none() {
                            context.parameter_name = self.parameter_name_from_node(parameter);
                        }

                        // fill declaration symbol from the parameter owner
                        if context.declaration_symbol.is_none() {
                            context.declaration_symbol = self
                                .symbol_for_node(parameter_id)
                                .and_then(|symbol| self.owner_symbol_from_symbol(symbol.local_id));
                        }
                    }
                }
                // handle member nodes
                dir::NodeType::Member => {
                    // load the member node
                    let member_id = dir::LocalNodeId::<dir::Member>::new(node_id.id);
                    let member = self.dir_tree.get(member_id);

                    // fill declaration symbol from the member owner
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol = self
                            .symbol_for_node(member_id)
                            .and_then(|symbol| self.owner_symbol_from_symbol(symbol.local_id));
                    }

                    // apply member naming context
                    self.apply_member_context(member, source_id, &mut context);
                }
                // handle property nodes
                dir::NodeType::Property => {
                    // load the property node
                    let property_id = dir::LocalNodeId::<dir::Property>::new(node_id.id);
                    let property = self.dir_tree.get(property_id);

                    // fill declaration symbol from the property owner
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol = self
                            .symbol_for_node(property_id)
                            .and_then(|symbol| self.owner_symbol_from_symbol(symbol.local_id));
                    }

                    // apply property naming context
                    self.apply_property_context(property, source_id, &mut context);
                }
                // handle declarator nodes
                dir::NodeType::Declarator => {
                    // load the declarator node
                    let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(node_id.id);
                    let declarator = self.dir_tree.get(declarator_id);

                    // fill declarator name when missing
                    if context.declarator_name.is_none() {
                        context.declarator_name = self.declarator_name_from_node(declarator);
                    }

                    // fill declaration symbol from the declarator binding
                    if context.declaration_symbol.is_none()
                        && let Some(symbol_id) = self.pattern_symbol(declarator.pattern)
                    {
                        context.declaration_symbol = self.owner_symbol_from_symbol(symbol_id);
                    }
                }
                // handle declaration nodes
                dir::NodeType::Declaration => {
                    // load the declaration node
                    let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(node_id.id);
                    let declaration = self.dir_tree.get(declaration_id);

                    // fill declaration symbol from the declaration
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol = self.symbol_for_node(declaration_id);
                    }

                    // apply declaration naming context
                    self.apply_declaration_context(declaration, source_id, &mut context);
                }
                // ignore other node kinds
                _ => {}
            }

            // move to the parent node
            current = self.dir_tree.get_parent(node_id.id);
        }

        // require a declaration symbol
        context.declaration_symbol?;

        // return the collected context
        Some(context)
    }

    /// Apply declaration-specific naming context.
    fn apply_declaration_context(
        &self,
        declaration: &dir::Declaration,
        source_id: Option<dir::LocalNodeIdAny>,
        context: &mut TypeNameContext,
    ) {
        // exit when no source id is available
        let Some(source_id) = source_id else {
            return;
        };

        // check declaration kinds for naming context
        match declaration {
            // handle function declarations
            dir::Declaration::Function(declaration) => {
                // mark function return types
                if declaration
                    .signature
                    .return_type
                    .is_some_and(|return_type| return_type.into_any() == source_id)
                {
                    context.is_return_type = true;
                }
            }
            // handle type alias declarations
            dir::Declaration::Type(declaration) => {
                // mark type alias values
                if declaration.value.into_any() == source_id {
                    context.is_type_alias = true;
                }
            }
            _ => {}
        }
    }

    /// Apply member-specific naming context.
    fn apply_member_context(
        &self,
        member: &dir::Member,
        source_id: Option<dir::LocalNodeIdAny>,
        context: &mut TypeNameContext,
    ) {
        // check member kinds for naming context
        match member {
            // handle field members
            dir::Member::Field { key, .. } => {
                // capture field name when missing
                if context.field_name.is_none() {
                    context.field_name = self.member_name_from_key(Some(*key));
                }
            }
            // handle method members
            dir::Member::Method { key, signature, .. } => {
                // capture method name when missing
                if context.method_name.is_none() {
                    context.method_name = self.member_name_from_key(*key);
                }

                // mark method return types
                if let Some(source_id) = source_id
                    && signature
                        .return_type
                        .is_some_and(|return_type| return_type.into_any() == source_id)
                {
                    context.is_return_type = true;
                }
            }
            _ => {}
        }
    }

    /// Apply property-specific naming context.
    fn apply_property_context(
        &self,
        property: &dir::Property,
        source_id: Option<dir::LocalNodeIdAny>,
        context: &mut TypeNameContext,
    ) {
        // check property kinds for naming context
        match property {
            // handle field properties
            dir::Property::Field { key, .. } => {
                // capture field name when missing
                if context.field_name.is_none() {
                    context.field_name = self.member_name_from_key(Some(*key));
                }
            }
            // handle method properties
            dir::Property::Method { key, signature, .. } => {
                // capture method name when missing
                if context.method_name.is_none() {
                    context.method_name = self.member_name_from_key(*key);
                }

                // mark method return types
                if let Some(source_id) = source_id
                    && signature
                        .return_type
                        .is_some_and(|return_type| return_type.into_any() == source_id)
                {
                    context.is_return_type = true;
                }
            }
            _ => {}
        }
    }

    /// Resolve a parameter name from a parameter node.
    fn parameter_name_from_node(&self, parameter: &dir::Parameter) -> Option<String> {
        // load string pool for name lookup
        let strings = &self.strings;

        // match parameter kinds to resolve names
        match parameter {
            // handle named parameters
            dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                Some(strings.get(*name).to_string())
            }
            // handle pattern parameters
            dir::Parameter::Pattern { pattern, .. } => {
                // prefer the bound symbol name
                if let Some(symbol) = self.symbol_for_node(*pattern) {
                    let symbol = self.symbols.get_symbol(symbol.local_id);
                    if let Some(name_id) = symbol.name() {
                        return Some(strings.get(name_id).to_string());
                    }
                }

                // fall back to the pattern binding name
                self.pattern_binding_name(*pattern)
            }
            dir::Parameter::VariadicPattern { pattern, .. } => {
                if let Some(symbol) = self.symbol_for_node(*pattern) {
                    let symbol = self.symbols.get_symbol(symbol.local_id);
                    if let Some(name_id) = symbol.name() {
                        return Some(strings.get(name_id).to_string());
                    }
                }
                self.pattern_binding_name(*pattern)
            }
            dir::Parameter::Error => None,
        }
    }

    /// Resolve a declarator binding name from a declarator node.
    fn declarator_name_from_node(&self, declarator: &dir::Declarator) -> Option<String> {
        // use the declarator pattern for binding name
        self.pattern_binding_name(declarator.pattern)
    }

    /// Resolve a binding name from a simple pattern.
    fn pattern_binding_name(&self, pattern_id: dir::LocalNodeId<dir::Pattern>) -> Option<String> {
        // load string pool and pattern node
        let strings = &self.strings;
        let pattern = self.dir_tree.get(pattern_id);

        // extract binding names when available
        match pattern {
            // handle simple binding patterns
            dir::Pattern::Binding { name, .. } => Some(strings.get(*name).to_string()),
            // reject unsupported pattern kinds
            _ => None,
        }
    }

    /// Resolve a binding symbol from a simple pattern.
    fn pattern_symbol(
        &self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> Option<dir::LocalSymbolId> {
        self.symbol_for_node(pattern_id)
            .map(|symbol| symbol.local_id)
    }

    /// Resolve the owning declaration symbol for a local symbol.
    fn owner_symbol_from_symbol(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        // seed with the symbol scope
        let symbol = self.symbols.get_symbol(symbol_id);
        let mut scope_id = symbol.scope.id;

        // walk up scopes to find an owner
        let mut seen_scopes = HashSet::new();
        loop {
            // avoid cycles in scope ownership
            if !seen_scopes.insert(scope_id) {
                break;
            }

            // return the first scope owner
            let scope = self.symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner {
                return Some(owner_id.into_global(self.module_id));
            }

            // move to the parent scope
            let Some(parent_scope) = scope.parent else {
                break;
            };
            scope_id = parent_scope.id;
        }

        // fall back to the symbol itself when no owner is registered
        Some(symbol_id.into_global(self.module_id))
    }

    /// Resolve a name string from a member key.
    pub(crate) fn member_name_from_key(&self, key: Option<dir::Key>) -> Option<String> {
        // require a key for name resolution
        let key = key?;

        // resolve the key into a static key
        let key = static_key_from_key(self.dir_tree, self.strings, key)?;

        // return the static key name
        Some(self.static_key_name(key))
    }

    /// Resolve a name string from a static key.
    fn static_key_name(&self, key: dir::StaticKey) -> String {
        static_key_string(&key, &self.strings)
    }

    /// Resolve a module-local static member name from an owner symbol and key.
    pub(crate) fn static_member_name(
        &self,
        owner_symbol: dir::GlobalSymbolId,
        key: Option<dir::Key>,
    ) -> Option<String> {
        let owner = self.symbol_path_name(owner_symbol)?;
        let member = self.member_name_from_key(key)?;
        Some(format!("{owner}.{member}"))
    }

    /// Resolve the module-local symbol path for a symbol.
    pub(crate) fn symbol_path_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        if symbol_id.module_id != self.module_id {
            return None;
        }

        self.symbol_path_from_symbols(symbol_id, self.symbols)
    }

    /// Resolve the qualified name for a symbol in this module.
    pub(crate) fn qualified_symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        if symbol_id.module_id != self.module_id {
            return self.qualified_symbol_name_for_global(symbol_id);
        }

        self.qualified_symbol_name_for_module(symbol_id, self.module, self.symbols)
    }

    /// Resolve the qualified name for a symbol in any module.
    pub(crate) fn qualified_symbol_name_for_global(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<String> {
        // load module metadata for the symbol
        let module = self
            .compiler
            .module(self.context.revision(), symbol_id.module_id);
        let dir = self.dir_bound_if_present(symbol_id.module_id)?;
        self.qualified_symbol_name_for_module(symbol_id, module.as_ref(), &dir.bindings)
    }

    /// Resolve the qualified name for a symbol and module pair.
    fn qualified_symbol_name_for_module(
        &self,
        symbol_id: dir::GlobalSymbolId,
        module: &Module,
        symbols: &dir::BindingTable,
    ) -> Option<String> {
        // load the owning package
        let package = self
            .compiler
            .package(self.context.revision(), module.package_id);

        // build the module prefix
        let package_name = package.name.as_ref()?;
        // require a non empty package name
        if package_name.is_empty() {
            return None;
        }
        let module_path = self.module_path_without_extension(module, package.as_ref())?;
        let module_prefix = if module_path.is_empty() {
            // use the package name for root modules
            package_name.to_string()
        } else {
            // append the module path for nested modules
            format!("{package_name}/{module_path}")
        };

        // build the symbol path
        let symbol_path = self.symbol_path_from_symbols(symbol_id, symbols)?;

        Some(format!("{module_prefix}:{symbol_path}"))
    }

    /// Resolve the package relative module path without extension.
    fn module_path_without_extension(&self, module: &Module, package: &Package) -> Option<String> {
        // prefer package relative paths when available
        let module_path = if let Some(path) = &module.path {
            let relative = package
                .path
                .as_ref()
                .and_then(|package_path| path.strip_prefix(package_path).ok())
                .unwrap_or(path);
            relative.to_string_lossy().to_string()
        } else {
            // fall back to the module uri
            module.uri.to_string()
        };

        // normalize separators and drop extension
        let module_path = self.normalize_path_separators(&module_path);
        Some(self.strip_extension_from_path(&module_path))
    }

    /// Build a symbol path from the module symbol table.
    fn symbol_path_from_symbols(
        &self,
        symbol_id: dir::GlobalSymbolId,
        symbols: &dir::BindingTable,
    ) -> Option<String> {
        // seed with the symbol name
        let symbol = symbols.get_symbol(symbol_id.into_local());
        let symbol_name = self.static_key_name(symbol.key?);
        let mut segments = vec![symbol_name];

        // walk owner scopes for namespaces and types
        let mut scope_id = symbol.scope.id;
        let mut seen_scopes = HashSet::new();
        loop {
            // avoid cycles in scope ownership
            if !seen_scopes.insert(scope_id) {
                break;
            }

            // collect named owners into the path
            let scope = symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner
                && owner_id != symbol_id.into_local()
            {
                let owner = symbols.get_symbol(owner_id);
                if let Some(owner_name) = owner.key {
                    let owner_name = self.static_key_name(owner_name);
                    // append the owner name to the path
                    segments.push(owner_name);
                }
            }

            // climb to the parent scope
            let Some(parent_scope) = scope.parent else {
                break;
            };
            scope_id = parent_scope.id;
        }

        // reverse for root to leaf order
        segments.reverse();
        Some(segments.join("."))
    }

    /// Normalize a module path to use forward slashes.
    fn normalize_path_separators(&self, path: &str) -> String {
        // replace separators and trim leading slashes
        let normalized = path.replace('\\', "/");
        normalized.trim_start_matches('/').to_string()
    }

    /// Strip the file extension from a module path.
    fn strip_extension_from_path(&self, path: &str) -> String {
        // split prefix and leaf for extension removal
        let (prefix, leaf) = path.rsplit_once('/').unwrap_or(("", path));
        let stripped = leaf.rsplit_once('.').map(|(base, _)| base).unwrap_or(leaf);

        // rebuild the path without the extension
        // return the leaf when no prefix is present
        if prefix.is_empty() {
            stripped.to_string()
        }
        // return the rebuilt path with the prefix
        else {
            format!("{prefix}/{stripped}")
        }
    }
}

/// Build a slug suitable for string literal global names.
fn string_literal_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_underscore = false;

    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };

        if mapped == '_' {
            if !last_was_underscore {
                slug.push('_');
                last_was_underscore = true;
            }
        } else {
            slug.push(mapped);
            last_was_underscore = false;
        }

        if slug.len() >= STRING_LITERAL_SLUG_MAX {
            break;
        }
    }

    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        return "string".to_string();
    }

    if slug
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
    {
        slug
    } else {
        format!("s_{slug}")
    }
}

/// Build a synthetic global name for a string literal value.
pub(crate) fn string_literal_global_name_for_content(value: &str) -> String {
    let hash = stable_hash_text(value);
    let slug = string_literal_slug(value);
    let suffix = string_literal_name_suffix(&slug);
    format!("{STRING_LITERAL_GLOBAL_PREFIX}{suffix}H{hash:016x}")
}

/// Convert a slug into an upper camel case name suffix.
fn string_literal_name_suffix(slug: &str) -> String {
    let mut suffix = String::new();

    for segment in slug.split('_').filter(|segment| !segment.is_empty()) {
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            continue;
        };

        suffix.push(first.to_ascii_uppercase());
        suffix.extend(chars);
    }

    if suffix.is_empty() {
        "String".to_string()
    } else {
        suffix
    }
}

/// Convert a static key to a field name.
///
/// For name and number keys, returns the string directly.
/// For symbol keys, generates a synthetic name with `@` prefix to avoid conflicts.
pub(crate) fn static_key_to_field_name(
    key: &dir::StaticKey,
    builder: &mut mir::ModuleBuilder,
) -> StringId {
    let name = static_key_string(key, builder.strings());
    builder.intern(&name)
}

#[cfg(test)]
mod tests {
    use super::static_key_to_field_name;
    use {destack_dir as dir, destack_mir as mir};

    /// Name keys return the string directly.
    #[test]
    fn test_static_key_name() {
        let mut builder = mir::ModuleBuilder::new();
        let name = builder.intern("foo");
        let key = dir::StaticKey::Name(name);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, name);
    }

    /// Number keys return the string directly.
    #[test]
    fn test_static_key_number() {
        let mut builder = mir::ModuleBuilder::new();
        let num = builder.intern("42");
        let key = dir::StaticKey::Number(num);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, num);
    }

    /// Registry symbol keys get synthetic names with @ prefix.
    #[test]
    fn test_static_key_registry_symbol() {
        let mut builder = mir::ModuleBuilder::new();
        let registry_key = builder.intern("myKey");
        let key = dir::StaticKey::Symbol(dir::SymbolKey::Registry(registry_key));

        let result = static_key_to_field_name(&key, &mut builder);
        let result_str = builder.strings().get(result);
        assert_eq!(&*result_str, "@Symbol.for(myKey)");
    }
}
