#![allow(clippy::too_many_arguments)]

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::types::format_global_type;
use crate::core::ModuleQueryContext;
use crate::dir::declaration_display_name;

/// Formatted declaration signature.
#[derive(Debug, Clone)]
pub struct FormattedSignature {
    /// The full signature string.
    pub text: String,
    /// The declaration kind (function, struct, etc.).
    pub kind: &'static str,
}

/// Formatted call signature for functions and methods.
#[derive(Debug, Clone)]
pub struct FormattedCallSignature {
    /// The full signature label.
    pub label: String,
    /// Parameter labels in declared order.
    pub parameters: Vec<String>,
}

/// Format a declaration's signature with full type information.
pub fn format_declaration_signature(
    declaration: &dir::Declaration,
    ctx: &ModuleQueryContext<'_>,
) -> FormattedSignature {
    // resolve dir data for formatting
    let dir_tree = ctx.dir().view();
    let types = ctx.dir().types();
    let module_id = ctx.module_id();
    let strings = ctx.dir().strings();

    // resolve declaration metadata
    let kind = declaration.kind_name();
    let name = declaration_display_name(strings, declaration);

    // resolve the declaration prefix
    let declaration_prefix = format_declaration_prefix(declaration);

    // format the signature text by declaration kind
    let text = match declaration {
        dir::Declaration::Function(declaration) => format_function(
            &name,
            &declaration.signature,
            &declaration_prefix,
            module_id,
            dir_tree,
            types,
            ctx,
        ),
        dir::Declaration::Global(_) => format!("{declaration_prefix}global"),
        dir::Declaration::Module(_) => format!("{declaration_prefix}module"),
        dir::Declaration::Struct(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}interface {name}{generics_text}")
        }
        dir::Declaration::Enum(_) => {
            format!("{declaration_prefix}enum {name}")
        }
        dir::Declaration::Type(_) => {
            format!("{declaration_prefix}type {name}")
        }
        dir::Declaration::Extension(_) => {
            format!("{declaration_prefix}extension {name}")
        }
    };

    // return the formatted signature
    FormattedSignature { text, kind }
}

/// Format a function signature with parameters and return type.
fn format_function(
    name: &str,
    signature: &dir::FunctionSignature,
    declaration_prefix: &str,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    // resolve the phase prefix
    let phase_prefix = format_function_phase_prefix(signature.phase);

    // resolve the async prefix
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // format generic parameters
    let generics_text =
        format_generics(&signature.generic_parameters, dir_tree, ctx.dir().strings());

    // format parameters
    let parameters_text = format_parameters(
        signature.this_parameter,
        &signature.parameters,
        module_id,
        dir_tree,
        types,
        ctx,
    );

    // format return type
    let return_text = signature
        .return_type
        .and_then(|return_node| {
            let node_id = dir::GlobalNodeIdAny {
                module_id,
                local_id: return_node.into(),
            };
            types.get_node_type_id(node_id)
        })
        .map(|type_id| format!(": {}", format_global_type(type_id, ctx)))
        .unwrap_or_default();

    // return the formatted function signature
    format!(
        "{declaration_prefix}{phase_prefix}{async_prefix}function {name}{generics_text}({parameters_text}){return_text}"
    )
}

/// Format the phase prefix for one function.
fn format_function_phase_prefix(phase: dir::FunctionPhase) -> &'static str {
    match phase {
        dir::FunctionPhase::Normal => "",
        dir::FunctionPhase::Comptime => "comptime ",
    }
}

/// Format the declaration prefix keywords for a declaration.
fn format_declaration_prefix(declaration: &dir::Declaration) -> String {
    let export_prefix = if declaration.export().is_some() {
        "export "
    } else {
        ""
    };

    let declare_prefix = if declaration.is_ambient() {
        "declare "
    } else {
        ""
    };

    let abstract_prefix = if declaration.is_abstract() {
        "abstract "
    } else {
        ""
    };

    format!("{export_prefix}{declare_prefix}{abstract_prefix}")
}

/// Format a function or method call signature without declaration keywords.
pub fn format_call_signature(
    name: &str,
    signature: &dir::FunctionSignature,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
    include_this: bool,
) -> FormattedCallSignature {
    // resolve the phase prefix
    let phase_prefix = format_function_phase_prefix(signature.phase);

    // resolve the async prefix
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // format generic parameters
    let generics_text =
        format_generics(&signature.generic_parameters, dir_tree, ctx.dir().strings());

    // choose parameters
    let this_parameter = if include_this {
        signature.this_parameter
    } else {
        None
    };

    // format parameter labels
    let parameter_labels = format_parameter_labels(
        this_parameter,
        &signature.parameters,
        module_id,
        dir_tree,
        types,
        ctx,
    );
    let parameters_text = parameter_labels.join(", ");

    // format return type
    let return_text = signature
        .return_type
        .and_then(|return_node| {
            let node_id = dir::GlobalNodeIdAny {
                module_id,
                local_id: return_node.into(),
            };
            types.get_node_type_id(node_id)
        })
        .map(|type_id| format!(": {}", format_global_type(type_id, ctx)))
        .unwrap_or_default();

    // build the call signature label
    let label = format!(
        "{phase_prefix}{async_prefix}{name}{generics_text}({parameters_text}){return_text}"
    );

    // return the call signature details
    FormattedCallSignature {
        label,
        parameters: parameter_labels,
    }
}

/// Format generic type parameters.
fn format_generics(
    generics: &[dir::LocalNodeId<dir::GenericParameter>],
    dir_tree: dir::View<'_>,
    strings: &StringPool,
) -> String {
    // skip empty generic lists
    if generics.is_empty() {
        return String::new();
    }

    // format each generic parameter
    let formatted: Vec<_> = generics
        .iter()
        .map(|parameter_id| format_generic_parameter(*parameter_id, dir_tree, strings))
        .collect();

    // return the formatted generics list
    format!("<{}>", formatted.join(", "))
}

/// Format a single generic parameter.
fn format_generic_parameter(
    parameter_id: dir::LocalNodeId<dir::GenericParameter>,
    dir_tree: dir::View<'_>,
    strings: &StringPool,
) -> String {
    let parameter = dir_tree.get::<dir::GenericParameter>(parameter_id);

    match parameter {
        dir::GenericParameter::Type {
            name,
            is_const,
            variance,
            ..
        } => {
            let prefix = format_type_generic_parameter_prefix(*is_const, *variance, false);

            format!("{prefix}{}", strings.get(*name))
        }
        dir::GenericParameter::VariadicType {
            name,
            is_const,
            variance,
            ..
        } => {
            let prefix = format_type_generic_parameter_prefix(*is_const, *variance, true);

            format!("{prefix}{}", strings.get(*name))
        }
        dir::GenericParameter::Value {
            name, is_comptime, ..
        } => {
            let prefix = format_value_generic_parameter_prefix(*is_comptime, false);

            format!("{prefix}{}", strings.get(*name))
        }
        dir::GenericParameter::VariadicValue {
            name, is_comptime, ..
        } => {
            let prefix = format_value_generic_parameter_prefix(*is_comptime, true);

            format!("{prefix}{}", strings.get(*name))
        }
        dir::GenericParameter::Error => "<error>".to_string(),
    }
}

/// Format the prefix for one type generic parameter.
fn format_type_generic_parameter_prefix(
    is_const: bool,
    variance: Option<dir::VarianceModifier>,
    is_variadic: bool,
) -> String {
    let mut prefix = String::new();

    if is_const {
        prefix.push_str("const ");
    }

    match variance {
        Some(dir::VarianceModifier::In) => prefix.push_str("in "),
        Some(dir::VarianceModifier::Out) => prefix.push_str("out "),
        Some(dir::VarianceModifier::InOut) => prefix.push_str("in out "),
        None => {}
    }

    if is_variadic {
        prefix.push_str("...");
    }

    prefix
}

/// Format the prefix for one value generic parameter.
fn format_value_generic_parameter_prefix(is_comptime: bool, is_variadic: bool) -> String {
    let mut prefix = String::new();

    if is_comptime {
        prefix.push_str("comptime ");
    }

    if is_variadic {
        prefix.push_str("...");
    }

    prefix
}

/// Format function parameters with their types.
fn format_parameters(
    this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    // format labels for parameters
    let formatted =
        format_parameter_labels(this_parameter, parameters, module_id, dir_tree, types, ctx);

    // return the joined parameter labels
    formatted.join(", ")
}

/// Format parameter labels in declared order.
fn format_parameter_labels(
    this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> Vec<String> {
    // collect formatted parameter labels
    let mut formatted: Vec<String> = Vec::new();

    // add the explicit this parameter when present
    if let Some(this_parameter) = this_parameter {
        let this_text = format_parameter(this_parameter, module_id, dir_tree, types, ctx);
        formatted.push(format!("this: {this_text}"));
    }

    // add remaining parameters in order
    formatted.extend(
        parameters
            .iter()
            .map(|parameter_id| format_parameter(*parameter_id, module_id, dir_tree, types, ctx)),
    );

    // return the formatted labels
    formatted
}

/// Format a single parameter with its type annotation.
fn format_parameter(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    let strings = ctx.dir().strings();

    // read the parameter node
    let parameter = dir_tree.get::<dir::Parameter>(parameter_id);

    // resolve the parameter name
    let name = match parameter {
        dir::Parameter::Named { name, .. } => {
            format_parameter_name(strings.get(*name), parameter.is_comptime())
        }
        dir::Parameter::Pattern { .. } => format_parameter_name("_", parameter.is_comptime()),
        dir::Parameter::VariadicNamed { name, .. } => {
            let name = format!("...{}", strings.get(*name));
            format_parameter_name(&name, parameter.is_comptime())
        }
        dir::Parameter::VariadicPattern { .. } => {
            format_parameter_name("...<pattern>", parameter.is_comptime())
        }
        dir::Parameter::Error => "<error>".to_string(),
    };

    // try to get the inferred type for this parameter
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: parameter_id.into(),
    };

    if let Some(type_id) = types.get_node_type_id(node_id) {
        let type_text = format_global_type(type_id, ctx);
        format!("{name}: {type_text}")
    } else {
        name
    }
}

/// Format one parameter label with its phase prefix.
fn format_parameter_name(name: &str, is_comptime: bool) -> String {
    if is_comptime {
        format!("comptime {name}")
    } else {
        name.to_string()
    }
}

/// Format a symbol's signature for hover display.
pub fn format_symbol_signature(
    root_ctx: &ModuleQueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<FormattedSignature> {
    // resolve module dir data
    let ctx = root_ctx.module_context(symbol_id.module_id)?;
    let declaration_ref = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        symbol.declaration?
    };

    // read the declaration from the tree
    let dir_tree = ctx.dir().view();
    let declaration_id = declaration_ref.local_id.try_into().ok()?;
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);

    // resolve type table and metadata
    let types = ctx.dir().types();
    let kind = declaration.kind_name();
    let strings = ctx.dir().strings();
    let name = declaration_display_name(strings, declaration);

    // resolve the declaration prefix
    let declaration_prefix = format_declaration_prefix(declaration);

    // resolve the module id for global ids
    let module_id = ctx.module_id();
    // format the signature text by declaration kind
    let text = match declaration {
        dir::Declaration::Function(declaration) => format_function(
            &name,
            &declaration.signature,
            &declaration_prefix,
            module_id,
            dir_tree,
            types,
            &ctx,
        ),
        dir::Declaration::Global(_) => format!("{declaration_prefix}global"),
        dir::Declaration::Module(_) => format!("{declaration_prefix}module"),
        dir::Declaration::Struct(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface(declaration) => {
            let generics_text = format_generics(&declaration.generic_parameters, dir_tree, strings);
            format!("{declaration_prefix}interface {name}{generics_text}")
        }
        dir::Declaration::Enum(_) => format!("{declaration_prefix}enum {name}"),
        dir::Declaration::Type(_) => format!("{declaration_prefix}type {name}"),
        dir::Declaration::Extension(_) => format!("{declaration_prefix}extension {name}"),
    };

    // return the formatted signature
    Some(FormattedSignature { text, kind })
}
