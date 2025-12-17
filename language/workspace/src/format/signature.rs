use destack_base::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::types::format_local_type;
use crate::{Module, ModuleRegistry};

/// Formatted declaration signature.
#[derive(Debug, Clone)]
pub struct FormattedSignature {
    /// The full signature string.
    pub text: String,
    /// The declaration kind (function, struct, etc.).
    pub kind: &'static str,
}

/// Format a declaration's signature with full type information.
pub fn format_declaration_signature(
    declaration: &dir::Declaration,
    module: &Module,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> FormattedSignature {
    let dir_tree = module.dir.tree.read();
    let types = module.dir.types.read();
    let module_id = module.id;

    let kind = declaration.kind_name();
    let descriptor = declaration.descriptor();

    let name = descriptor
        .name
        .map(|id| strings.get(id).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());

    let export_prefix = if descriptor.export.is_some() {
        "export "
    } else {
        ""
    };

    let text = match declaration {
        dir::Declaration::Function { signature, .. } => format_function(
            &name,
            signature,
            export_prefix,
            module_id,
            &dir_tree,
            &types,
            modules,
            strings,
        ),
        dir::Declaration::Struct { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}interface {name}{generics_text}")
        }
        dir::Declaration::Enum { .. } => {
            format!("{export_prefix}enum {name}")
        }
        dir::Declaration::Type { .. } => {
            format!("{export_prefix}type {name}")
        }
        dir::Declaration::Namespace { .. } => {
            format!("{export_prefix}namespace {name}")
        }
        dir::Declaration::Extension { .. } => {
            format!("{export_prefix}extension {name}")
        }
    };

    FormattedSignature { text, kind }
}

/// Format a function signature with parameters and return type.
#[allow(clippy::too_many_arguments)]
fn format_function(
    name: &str,
    signature: &dir::FunctionSignature,
    export_prefix: &str,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    // async
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // static parameters
    let generics_text = signature
        .generics
        .as_ref()
        .map(|g| format_generics(g, module_id, dir_tree, types, modules, strings))
        .unwrap_or_default();

    // dynamic parameters
    let parameters_text = format_parameters(
        &signature.dynamic_parameters,
        module_id,
        dir_tree,
        types,
        modules,
        strings,
    );

    // return type
    let return_text = signature
        .return_type
        .and_then(|return_node| {
            let node_id = dir::GlobalNodeIdAny {
                module_id,
                local_id: return_node.into(),
            };
            types.get_inferred_type_id(node_id)
        })
        .map(|type_id| format!(": {}", format_local_type(type_id, types, modules, strings)))
        .unwrap_or_default();

    format!(
        "{export_prefix}{async_prefix}function {name}{generics_text}({parameters_text}){return_text}"
    )
}

/// Format generic type parameters.
fn format_generics(
    generics: &dir::Generics,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let Some(parameters) = &generics.static_parameters else {
        return String::new();
    };
    if parameters.is_empty() {
        return String::new();
    }

    let formatted: Vec<_> = parameters
        .iter()
        .map(|parameter_id| {
            format_parameter(*parameter_id, module_id, dir_tree, types, modules, strings)
        })
        .collect();

    format!("<{}>", formatted.join(", "))
}

/// Format function parameters with their types.
fn format_parameters(
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let formatted: Vec<_> = parameters
        .iter()
        .map(|parameter_id| {
            format_parameter(*parameter_id, module_id, dir_tree, types, modules, strings)
        })
        .collect();

    formatted.join(", ")
}

/// Format a single parameter with its type annotation.
fn format_parameter(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let parameter = dir_tree.get::<dir::Parameter>(parameter_id);

    let name = match parameter {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::Pattern { .. } => "_".to_string(),
        dir::Parameter::Variadic { name, .. } => format!("...{}", strings.get(*name).as_str()),
    };

    // try to get the inferred type for this parameter
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: parameter_id.into(),
    };

    if let Some(type_id) = types.get_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, modules, strings);
        format!("{name}: {type_text}")
    } else {
        name
    }
}

/// Format a symbol's signature for hover display.
pub fn format_symbol_signature(
    symbol_id: dir::GlobalSymbolId,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> Option<FormattedSignature> {
    let module = modules.get(symbol_id.module_id);
    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.into_local());

    // only format symbols with declarations
    let declaration_ref = symbol.primary_declaration?;
    drop(symbols);

    let dir_tree = module_guard.dir.tree.read();
    let declaration_id = declaration_ref.local_id.try_into().ok()?;
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);

    let types = module_guard.dir.types.read();
    let kind = declaration.kind_name();
    let descriptor = declaration.descriptor();

    let name = descriptor
        .name
        .map(|id| strings.get(id).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());

    let export_prefix = if descriptor.export.is_some() {
        "export "
    } else {
        ""
    };

    let module_id = module_guard.id;

    let text = match declaration {
        dir::Declaration::Function { signature, .. } => format_function(
            &name,
            signature,
            export_prefix,
            module_id,
            &dir_tree,
            &types,
            modules,
            strings,
        ),
        dir::Declaration::Struct { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface { generics, .. } => {
            let generics_text =
                format_generics(generics, module_id, &dir_tree, &types, modules, strings);
            format!("{export_prefix}interface {name}{generics_text}")
        }
        dir::Declaration::Enum { .. } => format!("{export_prefix}enum {name}"),
        dir::Declaration::Type { .. } => format!("{export_prefix}type {name}"),
        dir::Declaration::Namespace { .. } => format!("{export_prefix}namespace {name}"),
        dir::Declaration::Extension { .. } => format!("{export_prefix}extension {name}"),
    };

    Some(FormattedSignature { text, kind })
}
