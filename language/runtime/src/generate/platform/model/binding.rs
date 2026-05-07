use std::collections::BTreeMap;

use destack_compiler::Compiler;
use destack_core::{StringPool, stable_hash_text};
use destack_dir::{
    self as dir, Argument, Declaration, DependencyItem, Expression, GlobalSymbolId, LanguageItem,
    PrimitiveType, StaticArgument, StaticExpression, TypeLiteral, WellKnownSymbol,
};
use destack_query::format::{format_local_type, format_type_literal};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::{
    BindingEnumValue, BindingEnumVariant, BindingField, BindingParameter, BindingReturn,
    BindingTaggedUnionVariant, BindingType,
};
use crate::context::GeneratorContext;

/// Binding type analysis context for one committed module artifact.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BindingTypeContext<'a> {
    /// The compiler used for shared semantic helpers.
    compiler: &'a Compiler,
    /// The generator context that owns retained semantic artifacts.
    context: &'a GeneratorContext,
    /// The committed module tree.
    tree: &'a dir::Tree,
    /// The committed module types.
    types: &'a dir::TypeTable,
    /// The committed module symbols.
    symbols: &'a dir::SymbolTable,
    /// The module registry for cross-module reads.
    modules: &'a GeneratorContext,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The active profile.
    profile_id: ProfileId,
    /// Canonical binding wrapper symbols.
    binding_symbols: &'a BindingTypeSymbols,
    /// The current platform domain.
    domain: &'a str,
}

impl<'a> BindingTypeContext<'a> {
    /// Build one binding type analysis context.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a GeneratorContext,
        tree: &'a dir::Tree,
        types: &'a dir::TypeTable,
        symbols: &'a dir::SymbolTable,
        modules: &'a GeneratorContext,
        strings: &'a StringPool,
        profile_id: ProfileId,
        binding_symbols: &'a BindingTypeSymbols,
        domain: &'a str,
    ) -> Self {
        Self {
            compiler,
            context,
            tree,
            types,
            symbols,
            modules,
            strings,
            profile_id,
            binding_symbols,
            domain,
        }
    }

    /// Lower one committed type id into one runtime binding type.
    pub(crate) fn binding_type_from_type_id(&self, type_id: dir::LocalTypeId) -> BindingType {
        binding_type_from_type_id(
            self.compiler,
            self.context,
            type_id,
            self.tree,
            self.types,
            self.symbols,
            self.modules,
            self.strings,
            self.profile_id,
            self.binding_symbols,
            self.domain,
        )
    }

    /// Lower one committed symbol into one runtime binding type.
    pub(crate) fn binding_type_from_symbol(&self, symbol_id: GlobalSymbolId) -> BindingType {
        binding_type_from_symbol(
            self.compiler,
            self.context,
            symbol_id,
            self.modules,
            self.strings,
            self.profile_id,
            self.binding_symbols,
        )
    }
}

/// Canonical symbol ids used for binding type resolution.
#[derive(Debug, Clone)]
pub(crate) struct BindingTypeSymbols {
    /// Result type symbol id.
    result: GlobalSymbolId,
    /// AsyncResult type symbol id.
    async_result: Option<GlobalSymbolId>,
    /// Slice type symbol id.
    slice: Option<GlobalSymbolId>,
    /// Array type symbol id.
    array: Option<GlobalSymbolId>,
    /// ReadonlyArray type symbol id.
    readonly_array: Option<GlobalSymbolId>,
}

impl BindingTypeSymbols {
    /// Return true if the symbol is a Result wrapper.
    fn is_result(&self, symbol: GlobalSymbolId) -> bool {
        symbol == self.result
    }

    /// Return true if the symbol is an AsyncResult wrapper.
    fn is_async_result(&self, symbol: GlobalSymbolId) -> bool {
        matches!(self.async_result, Some(id) if id == symbol)
    }

    /// Return the slice kind for a well-known symbol.
    fn slice_kind(&self, symbol: GlobalSymbolId) -> Option<SliceKind> {
        if matches!(self.slice, Some(id) if id == symbol) {
            return Some(SliceKind::Slice);
        }
        if matches!(self.array, Some(id) if id == symbol) {
            return Some(SliceKind::Array);
        }
        if matches!(self.readonly_array, Some(id) if id == symbol) {
            return Some(SliceKind::ReadonlyArray);
        }
        None
    }
}

/// Slice/array binding kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SliceKind {
    /// Borrowed slice type.
    Slice,
    /// Array type.
    Array,
    /// Readonly array type.
    ReadonlyArray,
}

/// Resolve binding type symbols for a profile.
pub(crate) fn binding_type_symbols(
    context: &GeneratorContext,
    profile_id: ProfileId,
) -> BindingTypeSymbols {
    // resolve semantic environments for the active profile
    let language_environment = context.language_environment(profile_id);
    let lib_environment = context.library_environment(profile_id);
    let result = language_environment
        .item(LanguageItem::Result)
        .unwrap_or_else(|| panic!("missing Result symbol for profile {profile_id:?}"));
    let async_result = language_environment.item(LanguageItem::AsyncResult);

    let well_known = lib_environment.well_known_symbols();
    let slice = well_known.get_type_symbol(WellKnownSymbol::Slice);
    let array = well_known.get_type_symbol(WellKnownSymbol::Array);
    let readonly_array = well_known.get_type_symbol(WellKnownSymbol::ReadonlyArray);

    BindingTypeSymbols {
        result,
        async_result,
        slice,
        array,
        readonly_array,
    }
}

/// Format a declaration signature for binding metadata.
pub(crate) fn format_declared_signature(
    context: &GeneratorContext,
    compiler: &Compiler,
    declaration_id: dir::LocalNodeId<Declaration>,
    declaration: &dir::Declaration,
    module: &Module,
    strings: &StringPool,
    profile_id: ProfileId,
) -> String {
    // load the relevant module state for formatting
    let dir = context.dir(module.id, profile_id);
    let tree = dir.tree();
    let types = dir.types();

    // gather declaration naming and export metadata
    let descriptor = declaration.descriptor();
    let name = descriptor
        .name
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let export_prefix = if descriptor.export.is_some() {
        "export "
    } else {
        ""
    };

    let dir::Declaration::Function { signature, .. } = declaration else {
        return format!("{export_prefix}function {name}()");
    };

    // render the async and parameter lists
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    let parameters_text = signature
        .parameters
        .iter()
        .map(|parameter_id| {
            format_parameter_declared(*parameter_id, module.id, context, &tree, &types, strings)
        })
        .collect::<Vec<_>>()
        .join(", ");

    // resolve the declared or inferred return type
    let return_text =
        resolve_return_type_text(declaration_id, signature, context, &tree, &types, strings)
            .unwrap_or_default();

    // emit the final signature string
    format!("{export_prefix}{async_prefix}function {name}({parameters_text}){return_text}")
}

/// Collect binding parameter metadata for a declaration signature.
pub(crate) fn collect_binding_params(
    context: &BindingTypeContext<'_>,
    signature: &dir::FunctionSignature,
) -> Vec<BindingParameter> {
    // build binding parameters from the signature list
    signature
        .parameters
        .iter()
        .map(|parameter_id| {
            let name = parameter_name(*parameter_id, context.tree, context.strings);
            let is_optional = parameter_is_optional(*parameter_id, context.tree);
            let Some(type_id) = type_id_for_parameter(*parameter_id, context.types) else {
                unsupported_binding_type(
                    "parameter",
                    "platform binding parameter is missing a type annotation",
                );
            };
            let type_text = type_text_for_signature(
                type_id,
                context.context,
                context.tree,
                context.types,
                context.strings,
            );
            let binding_type = context.binding_type_from_type_id(type_id);
            let binding_type = if is_optional {
                BindingType::Optional(Box::new(binding_type))
            } else {
                binding_type
            };

            BindingParameter {
                name,
                type_text,
                binding_type,
            }
        })
        .collect()
}

/// Return whether one parameter is optional at the call boundary.
fn parameter_is_optional(parameter_id: dir::LocalNodeId<dir::Parameter>, tree: &dir::Tree) -> bool {
    let parameter = tree.get::<dir::Parameter>(parameter_id);

    parameter
        .modifiers()
        .is_some_and(|modifiers| matches!(modifiers.kind, Some(dir::BindingKind::Maybe)))
        || parameter.has_default()
}

/// Collect binding return metadata for a declaration.
pub(crate) fn collect_binding_return(
    context: &BindingTypeContext<'_>,
    declaration_id: dir::LocalNodeId<Declaration>,
    signature: &dir::FunctionSignature,
) -> BindingReturn {
    // resolve a typed return id when possible
    let return_type_id = resolve_return_type_id(declaration_id, signature, context.types);
    if let Some(type_id) = return_type_id {
        let is_result = is_result_type_id(type_id, context.types, context.binding_symbols);
        let binding_type = context.binding_type_from_type_id(type_id);
        return BindingReturn {
            binding_type,
            is_result,
        };
    }

    // fall back to an untyped binding
    unsupported_binding_type(
        "return type",
        "platform binding return type is missing a type annotation",
    )
}

/// Resolve the parameter name string from a node.
fn parameter_name(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    tree: &dir::Tree,
    strings: &StringPool,
) -> String {
    let parameter = tree.get::<dir::Parameter>(parameter_id);
    match parameter {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::Pattern { .. } => "_".to_string(),
        dir::Parameter::VariadicNamed { name, .. } => {
            format!("...{}", strings.get(*name).as_ref())
        }
        dir::Parameter::VariadicPattern { .. } => "..._".to_string(),
        dir::Parameter::Error { .. } => "_".to_string(),
    }
}

/// Resolve the type id for a parameter declaration.
fn type_id_for_parameter(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    types: &dir::TypeTable,
) -> Option<dir::LocalTypeId> {
    let node_id = parameter_id.into_global_any(types.module_id);

    // prefer the committed semantic type, not the authored annotation slot
    types
        .get_inferred_type_id(node_id)
        .or_else(|| types.get_declared_type_id(node_id))
}

/// Resolve the return type id for a declaration signature.
fn resolve_return_type_id(
    declaration_id: dir::LocalNodeId<Declaration>,
    signature: &dir::FunctionSignature,
    types: &dir::TypeTable,
) -> Option<dir::LocalTypeId> {
    // prefer the committed semantic return type over the authored annotation slot
    if let Some(return_node) = signature.return_type {
        let node_id = return_node.into_global_any(types.module_id);
        if let Some(type_id) = types
            .get_inferred_type_id(node_id)
            .or_else(|| types.get_declared_type_id(node_id))
        {
            return Some(type_id);
        }
    }

    // fall back to signature-inferred function type when return annotation is missing
    let global_declaration_id = declaration_id.into_global_any(types.module_id);
    types
        .signature_type_id(global_declaration_id)
        .and_then(
            |signature_type_id| match types.get_type(signature_type_id) {
                dir::Type::Function { return_type, .. } => *return_type,
                _ => None,
            },
        )
}

/// Resolve the textual return type string for a declaration signature.
fn resolve_return_type_text(
    declaration_id: dir::LocalNodeId<Declaration>,
    signature: &dir::FunctionSignature,
    context: &GeneratorContext,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    strings: &StringPool,
) -> Option<String> {
    // resolve the return type id when possible
    let return_type_id = resolve_return_type_id(declaration_id, signature, types);
    let return_text = return_type_id
        .and_then(|type_id| {
            if let Some(formatted) = type_text_for_signature(type_id, context, tree, types, strings)
            {
                return Some(format!(": {formatted}"));
            }

            if let dir::Type::Unevaluated(expression_id) = types.get_type(type_id) {
                return format_type_expression(*expression_id, tree, strings)
                    .map(|text| format!(": {text}"));
            }

            None
        })
        .or_else(|| {
            signature.return_type.and_then(|return_node| {
                let text = format_type_expression(return_node, tree, strings);
                text.map(|text| format!(": {text}"))
            })
        });

    return_text
}

/// Map a type id into a binding type for generated wrappers.
pub(crate) fn binding_type_from_type_id(
    compiler: &Compiler,
    context: &GeneratorContext,
    type_id: dir::LocalTypeId,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_table: &dir::SymbolTable,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    symbols: &BindingTypeSymbols,
    domain: &str,
) -> BindingType {
    let type_text = type_text_for_diagnostics(type_id, context, types, strings);
    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => {
            binding_type_from_literal(value, type_text.as_str(), strings)
        }
        dir::Type::Union { elements } => {
            if elements
                .iter()
                .all(|element| is_string_literal_type(*element, types))
            {
                BindingType::String
            } else if let Some(inner_type_id) = unwrap_optional_union_type(elements, types) {
                let inner = binding_type_from_type_id(
                    compiler,
                    context,
                    inner_type_id,
                    tree,
                    types,
                    symbol_table,
                    modules,
                    strings,
                    profile_id,
                    symbols,
                    domain,
                );
                BindingType::Optional(Box::new(inner))
            } else {
                unsupported_binding_type(
                    type_text.as_str(),
                    "unsupported union type in platform bindings",
                )
            }
        }
        dir::Type::Tuple {
            elements,
            is_readonly: _,
        } => binding_type_from_tuple(
            compiler,
            context,
            type_id,
            elements,
            tree,
            types,
            symbol_table,
            modules,
            strings,
            profile_id,
            symbols,
            domain,
        ),
        dir::Type::Unevaluated(expression_id) => {
            let dir = context.dir(types.module_id, profile_id);
            let tree = dir.tree();
            let expression_text =
                format_type_expression(*expression_id, tree, strings).unwrap_or(type_text.clone());

            unsupported_binding_type(
                expression_text.as_str(),
                "unevaluated type reached runtime binding generation",
            )
        }
        dir::Type::Reference {
            symbol,
            generic_arguments,
        } => {
            let symbol = compiler.canonical_declared_artifact_symbol_for_revision(
                context.revision(),
                profile_id,
                *symbol,
            );
            if symbols.is_result(symbol) || symbols.is_async_result(symbol) {
                if let Some(inner) = unwrap_first_type_argument(generic_arguments.as_ref()) {
                    return binding_type_from_type_id(
                        compiler,
                        context,
                        inner,
                        tree,
                        types,
                        symbol_table,
                        modules,
                        strings,
                        profile_id,
                        symbols,
                        domain,
                    );
                }
            }

            if let Some(kind) = symbols.slice_kind(symbol) {
                if let Some(inner) = unwrap_first_type_argument(generic_arguments.as_ref()) {
                    let inner_binding = binding_type_from_type_id(
                        compiler,
                        context,
                        inner,
                        tree,
                        types,
                        symbol_table,
                        modules,
                        strings,
                        profile_id,
                        symbols,
                        domain,
                    );
                    return match kind {
                        SliceKind::Slice => {
                            if inner_binding == BindingType::String {
                                BindingType::StringSlice
                            } else {
                                BindingType::Slice(Box::new(inner_binding))
                            }
                        }
                        SliceKind::Array | SliceKind::ReadonlyArray => {
                            BindingType::Array(Box::new(inner_binding))
                        }
                    };
                }

                let type_text = type_text_for_diagnostics(type_id, context, types, strings);
                unsupported_binding_type(
                    type_text.as_str(),
                    "slice-like binding type is missing its element type",
                )
            }

            binding_type_from_symbol(
                compiler, context, symbol, modules, strings, profile_id, symbols,
            )
        }
        dir::Type::Form(form) => binding_type_from_type_id(
            compiler,
            context,
            form.base,
            tree,
            types,
            symbol_table,
            modules,
            strings,
            profile_id,
            symbols,
            domain,
        ),
        dir::Type::Array { element, .. } => {
            let element = element.unwrap_or_else(|| {
                unsupported_binding_type(type_text.as_str(), "array element type is missing")
            });
            BindingType::Array(Box::new(binding_type_from_type_id(
                compiler,
                context,
                element,
                tree,
                types,
                symbol_table,
                modules,
                strings,
                profile_id,
                symbols,
                domain,
            )))
        }
        dir::Type::ArraySized { element, .. } => {
            BindingType::Array(Box::new(binding_type_from_type_id(
                compiler,
                context,
                *element,
                tree,
                types,
                symbol_table,
                modules,
                strings,
                profile_id,
                symbols,
                domain,
            )))
        }
        _ => unsupported_binding_type(type_text.as_str(), "unsupported type in platform bindings"),
    }
}

fn is_string_literal_type(type_id: dir::LocalTypeId, types: &dir::TypeTable) -> bool {
    match types.get_type(type_id) {
        dir::Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(_)),
        } => true,
        _ => false,
    }
}

/// Resolve one optional union payload type when union includes one sentinel branch.
fn unwrap_optional_union_type(
    elements: &[dir::LocalTypeId],
    types: &dir::TypeTable,
) -> Option<dir::LocalTypeId> {
    let mut payload = None;
    let mut sentinel_count = 0usize;
    for element in elements {
        if is_optional_union_sentinel(*element, types) {
            sentinel_count += 1;
            continue;
        }

        if payload.is_some() {
            return None;
        }
        payload = Some(*element);
    }

    if sentinel_count > 0 { payload } else { None }
}

/// Return true when a union branch is one optional sentinel type.
fn is_optional_union_sentinel(type_id: dir::LocalTypeId, types: &dir::TypeTable) -> bool {
    match types.get_type(type_id) {
        dir::Type::TypeLiteral {
            value: TypeLiteral::Void | TypeLiteral::Undefined | TypeLiteral::Null,
        } => true,
        _ => false,
    }
}

/// Return true if the type id is a Result wrapper.
fn is_result_type_id(
    type_id: dir::LocalTypeId,
    types: &dir::TypeTable,
    symbols: &BindingTypeSymbols,
) -> bool {
    match types.get_type(type_id) {
        dir::Type::Reference { symbol, .. } => symbols.is_result(*symbol),
        dir::Type::Form(form) => is_result_type_id(form.base, types, symbols),
        _ => false,
    }
}

/// Map a literal type into a binding type.
fn binding_type_from_literal(
    value: &TypeLiteral,
    type_text: &str,
    strings: &StringPool,
) -> BindingType {
    match value {
        TypeLiteral::Void => BindingType::Void,
        TypeLiteral::ScalarLiteral(literal) => binding_type_from_scalar_literal(literal, strings),
        TypeLiteral::Primitive(primitive) => binding_type_from_primitive(*primitive, type_text),
        _ => unsupported_binding_type(type_text, "unsupported literal type"),
    }
}

/// Map a scalar literal type into a binding type.
fn binding_type_from_scalar_literal(
    literal: &dir::ScalarLiteral,
    _strings: &StringPool,
) -> BindingType {
    match literal {
        dir::ScalarLiteral::Boolean(_) => BindingType::Bool,
        dir::ScalarLiteral::String(_) => BindingType::String,
        dir::ScalarLiteral::Integer(value) | dir::ScalarLiteral::Bigint(value) => {
            if *value < 0 {
                BindingType::Int(64)
            } else {
                BindingType::UInt(64)
            }
        }
        _ => unsupported_binding_type("<scalar literal>", "unsupported scalar literal type"),
    }
}

/// Map a primitive type into a binding type.
fn binding_type_from_primitive(primitive: PrimitiveType, type_text: &str) -> BindingType {
    match primitive {
        PrimitiveType::Boolean => BindingType::Bool,
        PrimitiveType::String => BindingType::String,
        PrimitiveType::Int(int_type) => binding_type_from_int(int_type, type_text),
        PrimitiveType::Float(float_type) => binding_type_from_float(float_type, type_text),
        PrimitiveType::Number => BindingType::Float(64),
        _ => unsupported_binding_type(type_text, "unsupported primitive type"),
    }
}

/// Map an integer primitive into a binding type.
fn binding_type_from_int(int_type: dir::IntType, type_text: &str) -> BindingType {
    let int_type = int_type.simplify();
    match int_type {
        dir::IntType::Int8 => BindingType::Int(8),
        dir::IntType::Int16 => BindingType::Int(16),
        dir::IntType::Int32 => BindingType::Int(32),
        dir::IntType::Int64 => BindingType::Int(64),
        dir::IntType::Uint8 => BindingType::UInt(8),
        dir::IntType::Uint16 => BindingType::UInt(16),
        dir::IntType::Uint32 => BindingType::UInt(32),
        dir::IntType::Uint64 => BindingType::UInt(64),
        _ => unsupported_binding_type(type_text, "unsupported integer width"),
    }
}

/// Map a float primitive into a binding type.
fn binding_type_from_float(float_type: dir::FloatType, type_text: &str) -> BindingType {
    let float_type = float_type.simplify();
    match float_type {
        dir::FloatType::Float32 => BindingType::Float(32),
        dir::FloatType::Float64 => BindingType::Float(64),
        _ => unsupported_binding_type(type_text, "unsupported float width"),
    }
}

/// Resolve the platform domain for a module path.
fn platform_domain_for_module(modules: &GeneratorContext, module_id: ModuleId) -> Option<String> {
    let module = modules.get(module_id);
    let module = module.as_ref();
    if let Some(path) = module.path.as_ref() {
        if let Some(domain) = platform_domain_from_path(path) {
            return Some(domain);
        }
    }

    platform_domain_from_uri(module.uri.as_ref())
}

/// Extract the platform domain from a filesystem path.
fn platform_domain_from_path(path: &std::path::Path) -> Option<String> {
    let mut components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str());
    while let Some(component) = components.next() {
        if component == "platform" {
            return components.next().map(|domain| domain.to_string());
        }
    }
    None
}

/// Extract the platform domain from a module URI.
fn platform_domain_from_uri(uri: &str) -> Option<String> {
    for marker in ["platform/", "platform\\"] {
        if let Some(index) = uri.find(marker) {
            let rest = &uri[index + marker.len()..];
            let domain = rest.split(&['/', '\\'][..]).next()?;
            if !domain.is_empty() {
                return Some(domain.to_string());
            }
        }
    }
    None
}

/// Resolve a binding type from a global symbol.
pub(crate) fn binding_type_from_symbol(
    compiler: &Compiler,
    context: &GeneratorContext,
    symbol_id: GlobalSymbolId,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    symbols: &BindingTypeSymbols,
) -> BindingType {
    let module = modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let dir = context.dir(module.id, profile_id);
    let tree = dir.tree();
    let types = dir.types();
    let symbol_table = dir.symbols();

    let symbol = symbol_table.get_symbol(symbol_id.local_id);
    if let Some(target_symbol) = symbol.canonical_symbol.or(symbol.target_symbol)
        && target_symbol != symbol_id
    {
        return binding_type_from_symbol(
            compiler,
            context,
            target_symbol,
            modules,
            strings,
            profile_id,
            symbols,
        );
    }
    if let Some(primary_declaration) = symbol.primary_declaration
        && primary_declaration.local_id.ty == dir::NodeType::DependencyItem
    {
        let dependency_id = primary_declaration
            .try_into_typed::<DependencyItem>()
            .unwrap_or_else(|error| unsupported_binding_type("<dependency item>", &error));
        let dependency = tree.get::<DependencyItem>(dependency_id.local_id);
        if let Some(target_symbol) = dependency.target_symbol()
            && target_symbol != symbol_id
        {
            return binding_type_from_symbol(
                compiler,
                context,
                target_symbol,
                modules,
                strings,
                profile_id,
                symbols,
            );
        }
    }

    let name = symbol
        .name()
        .map(|name| strings.get(name).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let declaration_id = symbol
        .primary_declaration
        .unwrap_or_else(|| unsupported_binding_type(&name, "missing declaration for binding type"));
    let declaration_id = declaration_id
        .local_id
        .try_into_typed::<Declaration>()
        .unwrap_or_else(|error| unsupported_binding_type(&name, &error));
    let declaration = tree.get::<Declaration>(declaration_id);
    let domain = if let Some(domain) = platform_domain_for_module(modules, symbol_id.module_id) {
        domain
    } else {
        // allow the intrinsic capability alias used by platform security bindings
        if name == "PlatformCapability" {
            return BindingType::String;
        }

        unsupported_binding_type(&name, "binding type must live under platform");
    };

    match declaration {
        Declaration::Struct { members, .. } => binding_type_from_struct(
            compiler,
            context,
            name,
            symbol_id,
            declaration_id,
            &members,
            &tree,
            &types,
            &symbol_table,
            domain.clone(),
            modules,
            strings,
            profile_id,
            symbols,
        ),
        Declaration::Enum { fields, .. } => binding_type_from_enum(
            name,
            fields.clone(),
            &tree,
            &types,
            symbol_id,
            domain,
            strings,
        ),
        Declaration::Type { kind, value, .. } => {
            let alias_target = types.get_alias_target_type_id(symbol_id).or_else(|| {
                let value_node = dir::GlobalNodeIdAny {
                    module_id: symbol_id.module_id,
                    local_id: (*value).into(),
                };
                types.get_declared_or_inferred_type_id(value_node)
            });

            // use lowered alias type info when available
            let inner = if let Some(alias_target) = alias_target {
                if let dir::Type::Union { elements } = types.get_type(alias_target) {
                    if !elements
                        .iter()
                        .all(|element| is_string_literal_type(*element, &types))
                        && unwrap_optional_union_type(&elements, &types).is_none()
                    {
                        return binding_type_from_tagged_union_alias(
                            compiler,
                            context,
                            name,
                            &elements,
                            tree,
                            &types,
                            &symbol_table,
                            modules,
                            strings,
                            profile_id,
                            symbols,
                            domain,
                        );
                    }
                }

                if let dir::Type::Object { fields, .. } = types.get_type(alias_target) {
                    return binding_type_from_object_type(
                        compiler,
                        context,
                        name.clone(),
                        &fields,
                        tree,
                        &types,
                        &symbol_table,
                        modules,
                        strings,
                        profile_id,
                        symbols,
                        domain,
                    );
                }

                binding_type_from_type_id(
                    compiler,
                    context,
                    alias_target,
                    tree,
                    &types,
                    &symbol_table,
                    modules,
                    strings,
                    profile_id,
                    symbols,
                    domain.as_str(),
                )
            } else {
                unsupported_binding_type(
                    &name,
                    "missing lowered type alias target for binding type",
                )
            };

            // preserve named aliases as newtypes for platform bindings
            // this keeps ABI/type names stable even when aliases are structural
            let _ = kind;
            BindingType::Newtype {
                name,
                domain,
                inner: Box::new(inner),
            }
        }
        _ => unsupported_binding_type(&name, "unsupported binding declaration"),
    }
}

/// Resolve a union type-alias into one tagged union binding type.
fn binding_type_from_tagged_union_alias(
    compiler: &Compiler,
    context: &GeneratorContext,
    name: String,
    elements: &[dir::LocalTypeId],
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_table: &dir::SymbolTable,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    symbols: &BindingTypeSymbols,
    domain: String,
) -> BindingType {
    let mut variants = Vec::with_capacity(elements.len());
    let mut variant_names = BTreeMap::<String, ()>::new();
    for element in elements {
        let element_binding = binding_type_from_type_id(
            compiler,
            context,
            *element,
            tree,
            types,
            symbol_table,
            modules,
            strings,
            profile_id,
            symbols,
            domain.as_str(),
        );
        let variant_name =
            binding_tagged_union_variant_name(&element_binding).unwrap_or_else(|| {
                let type_text = type_text_for_diagnostics(*element, context, types, strings);
                unsupported_binding_type(
                    type_text.as_str(),
                    "tagged union branches must reference named binding types",
                )
            });

        if variant_names.insert(variant_name.clone(), ()).is_some() {
            unsupported_binding_type(
                name.as_str(),
                "tagged union variants must have unique names",
            );
        }

        variants.push(BindingTaggedUnionVariant {
            name: variant_name,
            binding_type: element_binding,
        });
    }

    BindingType::TaggedUnion {
        name,
        domain,
        variants,
    }
}

/// Resolve one tagged union variant name from one branch binding type.
fn binding_tagged_union_variant_name(binding_type: &BindingType) -> Option<String> {
    match binding_type {
        BindingType::Newtype { name, .. }
        | BindingType::Struct { name, .. }
        | BindingType::Enum { name, .. }
        | BindingType::TaggedUnion { name, .. } => Some(name.clone()),
        _ => None,
    }
}

/// Resolve object fields into a binding struct type.
fn binding_type_from_object_type(
    compiler: &Compiler,
    context: &GeneratorContext,
    name: String,
    fields: &[dir::TypeField],
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_table: &dir::SymbolTable,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    symbols: &BindingTypeSymbols,
    domain: String,
) -> BindingType {
    let mut binding_fields = Vec::with_capacity(fields.len());
    for field in fields {
        let field_name = field_name_from_static_key(&field.key, strings).unwrap_or_else(|| {
            unsupported_binding_type(&name, "struct field name is not supported")
        });
        let field_binding = binding_type_from_type_id(
            compiler,
            context,
            field.ty,
            tree,
            types,
            symbol_table,
            modules,
            strings,
            profile_id,
            symbols,
            domain.as_str(),
        );
        let field_binding = if field.is_optional {
            BindingType::Optional(Box::new(field_binding))
        } else {
            field_binding
        };
        binding_fields.push(BindingField {
            name: field_name,
            documentation: None,
            binding_type: field_binding,
        });
    }

    BindingType::Struct {
        name,
        domain,
        fields: binding_fields,
    }
}

/// Resolve tuple elements into a binding struct type.
fn binding_type_from_tuple(
    compiler: &Compiler,
    context: &GeneratorContext,
    type_id: dir::LocalTypeId,
    elements: &[dir::TypeElement],
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_table: &dir::SymbolTable,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    symbols: &BindingTypeSymbols,
    domain: &str,
) -> BindingType {
    let type_text = type_text_for_diagnostics(type_id, context, types, strings);
    let name = tuple_struct_name(Some(type_text.as_str()), elements.len());
    let mut fields = Vec::with_capacity(elements.len());
    for (index, element) in elements.iter().enumerate() {
        if element.is_optional || element.is_rest || element.is_readonly {
            unsupported_binding_type(
                &type_text,
                "tuple elements cannot be optional, rest, or readonly in platform bindings",
            );
        }
        let field_name = format!("item{index}");
        let field_binding = binding_type_from_type_id(
            compiler,
            context,
            element.ty,
            tree,
            types,
            symbol_table,
            modules,
            strings,
            profile_id,
            symbols,
            domain,
        );
        fields.push(BindingField {
            name: field_name,
            documentation: None,
            binding_type: field_binding,
        });
    }

    BindingType::Struct {
        name,
        domain: domain.to_string(),
        fields,
    }
}

/// Resolve struct fields into a binding type.
fn binding_type_from_struct(
    compiler: &Compiler,
    context: &GeneratorContext,
    name: String,
    struct_symbol: GlobalSymbolId,
    _declaration_id: dir::LocalNodeId<Declaration>,
    members: &[dir::LocalNodeId<dir::Member>],
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbols: &dir::SymbolTable,
    domain: String,
    modules: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    binding_symbols: &BindingTypeSymbols,
) -> BindingType {
    let mut fields = Vec::new();
    for member_id in members {
        let member = tree.get::<dir::Member>(*member_id);
        let dir::Member::Field {
            modifiers,
            key,
            value: _,
            default: _,
            symbol,
            ..
        } = member
        else {
            continue;
        };
        let field_name = field_name_from_key(key.as_ref(), strings).unwrap_or_else(|| {
            unsupported_binding_type(&name, "struct field name is not supported")
        });
        let field_symbol = GlobalSymbolId::new(struct_symbol.module_id, *symbol);
        let field_type_id = types.get_value_type_id(field_symbol).unwrap_or_else(|| {
            let field_path = format!("{name}.{field_name}");
            unsupported_binding_type(
                &field_path,
                "committed struct binding is missing its field symbol type",
            )
        });
        let field_type = types.get_type(field_type_id);
        if matches!(field_type, dir::Type::Unevaluated(_)) {
            let field_path = format!("{name}.{field_name}");
            unsupported_binding_type(
                &field_path,
                "committed struct binding field type is still unevaluated",
            );
        }

        let field_binding = binding_type_from_type_id(
            compiler,
            context,
            field_type_id,
            tree,
            types,
            symbols,
            modules,
            strings,
            profile_id,
            binding_symbols,
            domain.as_str(),
        );
        let is_optional = modifiers
            .as_ref()
            .is_some_and(|modifiers| matches!(modifiers.kind, Some(dir::BindingKind::Maybe)));
        let field_binding = if is_optional {
            BindingType::Optional(Box::new(field_binding))
        } else {
            field_binding
        };
        fields.push(BindingField {
            name: field_name,
            documentation: node_documentation(
                compiler,
                artifacts,
                modules,
                struct_symbol.module_id,
                tree,
                member_id.id,
            ),
            binding_type: field_binding,
        });
    }

    BindingType::Struct {
        name,
        domain,
        fields,
    }
}

/// Collect documentation comments from one source-backed node.
fn node_documentation(
    compiler: &Compiler,
    _artifacts: &ArtifactStore,
    _modules: &ModuleRegistry,
    _module_id: ModuleId,
    tree: &dir::Tree,
    node_id: u32,
) -> Option<String> {
    let documentation = tree.get_documentation(node_id)?;

    Some(compiler.program.strings.get(documentation.text).to_string())
}

/// Resolve enum metadata into a binding type.
fn binding_type_from_enum(
    name: String,
    fields: Vec<dir::LocalNodeId<dir::EnumField>>,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_id: GlobalSymbolId,
    domain: String,
    strings: &StringPool,
) -> BindingType {
    let backing = types
        .get_enum_backing_type(symbol_id)
        .unwrap_or_else(|| infer_enum_backing(&name, &fields, tree, types, symbol_id, strings));
    if matches!(backing, dir::EnumBackingType::String) {
        unsupported_binding_type(&name, "string-backed enums are not supported");
    }
    let mut variants = Vec::new();
    for field_id in fields {
        let field = tree.get::<dir::EnumField>(field_id);
        let variant_symbol = GlobalSymbolId::new(symbol_id.module_id, field.symbol);
        let value = types
            .get_enum_field_value(variant_symbol)
            .map(|value| match value {
                dir::EnumFieldValue::Int(value) => BindingEnumValue::Int(value),
                dir::EnumFieldValue::String(value) => {
                    BindingEnumValue::String(strings.get(value).to_string())
                }
            })
            .or_else(|| {
                field
                    .value
                    .and_then(|expr| enum_field_value_from_expression(expr, tree, strings))
            })
            .unwrap_or_else(|| {
                unsupported_binding_type(&name, "missing enum field value for binding")
            });
        variants.push(BindingEnumVariant {
            name: strings.get(field.name).to_string(),
            value,
        });
    }

    BindingType::Enum {
        name,
        domain,
        backing,
        variants,
    }
}

fn tuple_struct_name(type_text: Option<&str>, arity: usize) -> String {
    let Some(type_text) = type_text else {
        return format!("Tuple{arity}");
    };
    if type_text.is_empty() {
        return format!("Tuple{arity}");
    }
    let hash = stable_hash_text(type_text);
    format!("Tuple_{hash:016x}")
}

/// Infer an enum backing type from its field values.
fn infer_enum_backing(
    name: &str,
    fields: &[dir::LocalNodeId<dir::EnumField>],
    tree: &dir::Tree,
    types: &dir::TypeTable,
    symbol_id: GlobalSymbolId,
    strings: &StringPool,
) -> dir::EnumBackingType {
    let mut min_value: i128 = 0;
    let mut max_value: i128 = 0;
    let mut has_value = false;

    for field_id in fields {
        let field = tree.get::<dir::EnumField>(*field_id);
        let variant_symbol = GlobalSymbolId::new(symbol_id.module_id, field.symbol);
        let value = types
            .get_enum_field_value(variant_symbol)
            .map(|value| match value {
                dir::EnumFieldValue::Int(value) => BindingEnumValue::Int(value),
                dir::EnumFieldValue::String(_) => {
                    unsupported_binding_type(name, "string-backed enums are not supported");
                }
            })
            .or_else(|| {
                field
                    .value
                    .and_then(|expr| enum_field_value_from_expression(expr, tree, strings))
            })
            .unwrap_or_else(|| {
                unsupported_binding_type(name, "missing enum field value for binding")
            });
        match value {
            BindingEnumValue::Int(value) => {
                let value = value as i128;
                if !has_value {
                    min_value = value;
                    max_value = value;
                    has_value = true;
                } else {
                    min_value = min_value.min(value);
                    max_value = max_value.max(value);
                }
            }
            BindingEnumValue::String(_) => {
                unsupported_binding_type(name, "string-backed enums are not supported");
            }
        }
    }

    if !has_value {
        unsupported_binding_type(name, "missing enum field values for binding");
    }

    if min_value < 0 {
        if min_value >= i8::MIN as i128 && max_value <= i8::MAX as i128 {
            return dir::EnumBackingType::Int(dir::IntType::Int8);
        }
        if min_value >= i16::MIN as i128 && max_value <= i16::MAX as i128 {
            return dir::EnumBackingType::Int(dir::IntType::Int16);
        }
        if min_value >= i32::MIN as i128 && max_value <= i32::MAX as i128 {
            return dir::EnumBackingType::Int(dir::IntType::Int32);
        }
        return dir::EnumBackingType::Int(dir::IntType::Int64);
    }

    if max_value <= u8::MAX as i128 {
        return dir::EnumBackingType::Int(dir::IntType::Uint8);
    }
    if max_value <= u16::MAX as i128 {
        return dir::EnumBackingType::Int(dir::IntType::Uint16);
    }
    if max_value <= u32::MAX as i128 {
        return dir::EnumBackingType::Int(dir::IntType::Uint32);
    }

    dir::EnumBackingType::Int(dir::IntType::Uint64)
}

/// Resolve enum field literal values directly from the AST.
fn enum_field_value_from_expression(
    expr_id: dir::LocalNodeId<Expression>,
    tree: &dir::Tree,
    strings: &StringPool,
) -> Option<BindingEnumValue> {
    let expression = tree.get::<Expression>(expr_id);
    match expression {
        Expression::ScalarLiteral { value } => match value {
            dir::ScalarLiteral::Integer(value) | dir::ScalarLiteral::Bigint(value) => {
                Some(BindingEnumValue::Int(*value))
            }
            dir::ScalarLiteral::String(value) => {
                Some(BindingEnumValue::String(strings.get(*value).to_string()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Resolve a field name from a declaration member key.
fn field_name_from_key(key: Option<&dir::DynamicKey>, strings: &StringPool) -> Option<String> {
    match key {
        Some(dir::DynamicKey::Name(name))
        | Some(dir::DynamicKey::Private(name))
        | Some(dir::DynamicKey::Number(name)) => Some(strings.get(*name).to_string()),
        Some(dir::DynamicKey::NamedExpression { name, .. }) => Some(strings.get(*name).to_string()),
        Some(dir::DynamicKey::Expression(_)) | None => None,
    }
}

/// Resolve a struct field name from a static key.
fn field_name_from_static_key(key: &dir::StaticKey, strings: &StringPool) -> Option<String> {
    key.name().map(|name| strings.get(name).to_string())
}

/// Report an unsupported binding type.
fn unsupported_binding_type(type_text: &str, reason: &str) -> ! {
    panic!("unsupported binding type {type_text}: {reason}")
}

/// Extract the first static type argument from committed artifact state.
fn unwrap_first_type_argument(
    generic_arguments: Option<&Vec<StaticArgument>>,
) -> Option<dir::LocalTypeId> {
    let generic_arguments = generic_arguments?;
    let first = generic_arguments.first()?;
    match first {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => Some(*ty),
            StaticExpression::TypeLiteral { .. } => None,
            _ => None,
        },
        StaticArgument::Unevaluated { .. } => None,
    }
}

/// Format a type expression node into a signature fragment.
fn format_type_expression(
    expression_id: dir::LocalNodeId<Expression>,
    tree: &dir::Tree,
    strings: &StringPool,
) -> Option<String> {
    // render the expression node based on its type
    match tree.get::<Expression>(expression_id) {
        Expression::TypeLiteral { value } => Some(format_type_literal(value, strings)),
        Expression::TypeUnary { operator, right } => {
            let right = format_type_expression(*right, tree, strings)?;
            let formatted = match operator {
                dir::TypeUnaryOperator::Not => format!("!{right}"),
                dir::TypeUnaryOperator::Must => format!("{right}!"),
                dir::TypeUnaryOperator::Newtype => format!("newtype {right}"),
                dir::TypeUnaryOperator::Type => format!("type {right}"),
                dir::TypeUnaryOperator::Readonly => format!("readonly {right}"),
                dir::TypeUnaryOperator::Typeof => format!("typeof {right}"),
                dir::TypeUnaryOperator::Keyof => format!("keyof {right}"),
                dir::TypeUnaryOperator::AsConst => format!("{right} as const"),
                dir::TypeUnaryOperator::AsComptime => format!("{right} as comptime"),
            };
            Some(formatted)
        }
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            let left = format_type_expression(*left, tree, strings)?;
            let right = format_type_expression(*right, tree, strings)?;
            let op = match operator {
                dir::TypeBinaryOperator::Cast => " as ",
                dir::TypeBinaryOperator::In => " in ",
                dir::TypeBinaryOperator::Is => " is ",
                dir::TypeBinaryOperator::InstanceOf => " instanceof ",
                dir::TypeBinaryOperator::Satisfies => " satisfies ",
                dir::TypeBinaryOperator::Extends => " extends ",
                dir::TypeBinaryOperator::Implements => " implements ",
            };
            Some(format!("{left}{op}{right}"))
        }
        Expression::TypeIndex { left, index } => {
            let left = format_type_expression(*left, tree, strings)?;
            let index = format_type_expression(*index, tree, strings)?;
            Some(format!("{left}[{index}]"))
        }
        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            let left = format_type_expression(*left, tree, strings)?;
            let right = format_type_expression(*right, tree, strings)?;
            let then_type = format_type_expression(*then_type, tree, strings)?;
            let else_type = format_type_expression(*else_type, tree, strings)?;
            Some(format!(
                "{left} extends {right} ? {then_type} : {else_type}"
            ))
        }
        Expression::TypeTemplateLiteral {
            strings: parts,
            spans,
        } => {
            let mut out = String::from("`");
            for (index, string_id) in parts.iter().enumerate() {
                out.push_str(&strings.get(*string_id));
                if let Some(span_id) = spans.get(index) {
                    let span = format_type_expression(*span_id, tree, strings)?;
                    out.push_str("${");
                    out.push_str(&span);
                    out.push('}');
                }
            }
            out.push('`');
            Some(out)
        }
        Expression::TypeImport {
            target,
            qualifier,
            generic_arguments,
            ..
        } => {
            let target = format_type_expression(*target, tree, strings)?;
            let mut out = format!("import({target})");
            if let Some(qualifier) = qualifier {
                out.push('.');
                out.push_str(&format_path_segments(qualifier, strings));
            }
            if let Some(arguments) = generic_arguments {
                let argument_text = format_argument_list(arguments, tree, strings)?;
                out.push('<');
                out.push_str(&argument_text);
                out.push('>');
            }
            Some(out)
        }
        Expression::Member {
            left,
            name,
            generic_arguments,
        } => {
            let left_text = format_type_expression(*left, tree, strings)?;
            let mut out = match name {
                Some(name) if left_text.is_empty() => strings.get(*name).to_string(),
                Some(name) => format!("{left_text}.{}", strings.get(*name).as_ref()),
                None => left_text,
            };
            if let Some(arguments) = generic_arguments {
                let argument_text = format_argument_list(arguments, tree, strings)?;
                out.push('<');
                out.push_str(&argument_text);
                out.push('>');
            }
            Some(out)
        }
        Expression::UnresolvedPath {
            path,
            generic_arguments,
            ..
        }
        | Expression::LocalReference {
            path,
            generic_arguments,
            ..
        }
        | Expression::ModuleReference {
            path,
            generic_arguments,
            ..
        }
        | Expression::GlobalReference {
            path,
            generic_arguments,
            ..
        } => {
            let mut out = format_path_segments(path, strings);
            if let Some(arguments) = generic_arguments {
                let argument_text = format_argument_list(arguments, tree, strings)?;
                out.push('<');
                out.push_str(&argument_text);
                out.push('>');
            }
            Some(out)
        }
        _ => None,
    }
}

/// Format a list of type arguments as source text.
fn format_argument_list(
    arguments: &[dir::LocalNodeId<Argument>],
    tree: &dir::Tree,
    strings: &StringPool,
) -> Option<String> {
    let mut formatted = Vec::new();
    for argument_id in arguments {
        formatted.push(format_argument_expression(*argument_id, tree, strings)?);
    }
    Some(formatted.join(", "))
}

/// Format a single type argument expression.
fn format_argument_expression(
    argument_id: dir::LocalNodeId<Argument>,
    tree: &dir::Tree,
    strings: &StringPool,
) -> Option<String> {
    let argument = tree.get::<Argument>(argument_id);
    let value_text = format_type_expression(argument.value(), tree, strings)?;
    match argument {
        Argument::Named { name, .. } => {
            let name = strings.get(*name);
            Some(format!("{}: {value_text}", name.as_ref()))
        }
        Argument::Labeled { label, .. } => {
            let label = strings.get(*label);
            Some(format!("{}: {value_text}", label.as_ref()))
        }
        Argument::Spread { .. } => Some(format!("...{value_text}")),
        Argument::Error { .. } => Some(value_text),
        Argument::Positional { .. } => Some(value_text),
    }
}

/// Format a path segment list into a source string.
fn format_path_segments(path: &dir::Path, strings: &StringPool) -> String {
    path.segments
        .iter()
        .map(|segment| strings.get(*segment).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Format a parameter declaration into a signature fragment.
fn format_parameter_declared(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    module_id: ModuleId,
    context: &GeneratorContext,
    dir_tree: &dir::Tree,
    types: &dir::TypeTable,
    strings: &StringPool,
) -> String {
    let parameter = dir_tree.get::<dir::Parameter>(parameter_id);
    let name = match parameter {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::Pattern { .. } => "_".to_string(),
        dir::Parameter::VariadicNamed { name, .. } => {
            format!("...{}", strings.get(*name).as_ref())
        }
        dir::Parameter::VariadicPattern { .. } => "..._".to_string(),
        dir::Parameter::Error { .. } => "_".to_string(),
    };

    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: parameter_id.into(),
    };
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        if let Some(type_text) = type_text_for_signature(type_id, context, dir_tree, types, strings)
        {
            return format!("{name}: {type_text}");
        }
    } else {
        return name;
    }

    name
}

/// Format one type for signature output without generator sentinel placeholders.
fn type_text_for_signature(
    type_id: dir::LocalTypeId,
    context: &GeneratorContext,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    strings: &StringPool,
) -> Option<String> {
    if let dir::Type::Unevaluated(expression_id) = types.get_type(type_id) {
        return format_type_expression(*expression_id, tree, strings);
    }

    Some(format_type_checked(type_id, context, types, strings))
}

/// Format one type for diagnostics with explicit unresolved labeling.
fn type_text_for_diagnostics(
    type_id: dir::LocalTypeId,
    context: &GeneratorContext,
    types: &dir::TypeTable,
    strings: &StringPool,
) -> String {
    if matches!(types.get_type(type_id), dir::Type::Unevaluated(_)) {
        return format!("<type {type_id:?}>");
    }

    format_type_checked(type_id, context, types, strings)
}

/// Format one non-unevaluated type and reject formatter placeholders.
fn format_type_checked(
    type_id: dir::LocalTypeId,
    context: &GeneratorContext,
    types: &dir::TypeTable,
    strings: &StringPool,
) -> String {
    let formatted = format_local_type(
        type_id,
        types,
        context.repository(),
        context.revision(),
        strings,
    );
    if formatted == "<unevaluated>" {
        panic!(
            "internal generator bug: format_local_type returned unevaluated for type {type_id:?}"
        );
    }

    formatted
}

#[cfg(test)]
mod tests {
    use destack_core::StringPool;
    use destack_dir as dir;

    use super::super::BindingType;

    use super::binding_type_from_scalar_literal;

    /// Lower scalar string literals to string binding types.
    #[test]
    fn test_binding_type_from_scalar_literal_string() {
        let strings = StringPool::new();
        let literal = dir::ScalarLiteral::String(strings.intern("value"));
        let binding_type = binding_type_from_scalar_literal(&literal, &strings);

        assert_eq!(binding_type, BindingType::String);
    }

    /// Lower scalar boolean literals to boolean binding types.
    #[test]
    fn test_binding_type_from_scalar_literal_boolean() {
        let strings = StringPool::new();
        let literal = dir::ScalarLiteral::Boolean(true);
        let binding_type = binding_type_from_scalar_literal(&literal, &strings);

        assert_eq!(binding_type, BindingType::Bool);
    }

    /// Lower scalar integer literals to signed and unsigned binding types.
    #[test]
    fn test_binding_type_from_scalar_literal_integer_sign() {
        let strings = StringPool::new();
        let positive = dir::ScalarLiteral::Integer(5);
        let negative = dir::ScalarLiteral::Integer(-5);
        let positive_type = binding_type_from_scalar_literal(&positive, &strings);
        let negative_type = binding_type_from_scalar_literal(&negative, &strings);

        assert_eq!(positive_type, BindingType::UInt(64));
        assert_eq!(negative_type, BindingType::Int(64));
    }
}
