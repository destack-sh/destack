use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::types::format_local_type;
use destack_source::ProfileId;
use destack_workspace::{ArtifactRegistry, Module, ModuleRegistry};

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
///
/// # Panics
/// Panics if the module's DIR for the profile is not available.
pub fn format_declaration_signature(
    declaration: &dir::Declaration,
    module: &Module,
    artifacts: &ArtifactRegistry,
    modules: &ModuleRegistry,
    strings: &StringPool,
    profile: ProfileId,
) -> FormattedSignature {
    // resolve dir data for formatting
    let dir = artifacts
        .dir_snapshot(module.id, profile)
        .unwrap_or_else(|| panic!("no DIR artifact for profile {profile:?}"));
    let dir_tree = &dir.tree;
    let types = &dir.types;
    let module_id = module.id;

    // resolve declaration metadata
    let kind = declaration.kind_name();
    let descriptor = declaration.descriptor();

    // resolve the declaration name
    let name = descriptor
        .name
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());

    // resolve the export prefix
    let export_prefix = if descriptor.export.is_some() {
        "export "
    } else {
        ""
    };

    // format the signature text by declaration kind
    let text = match declaration {
        dir::Declaration::Function { signature, .. } => format_function(
            &name,
            signature,
            export_prefix,
            artifacts,
            module_id,
            &dir_tree,
            &types,
            modules,
            strings,
        ),
        dir::Declaration::Global { .. } => {
            let declare_prefix = if descriptor.kind == dir::DeclarationKind::Declaration {
                "declare "
            } else {
                ""
            };
            format!("{export_prefix}{declare_prefix}global")
        }
        dir::Declaration::ImportAlias { kind, .. } => {
            format_import_alias(&name, *kind, export_prefix)
        }
        dir::Declaration::Struct { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
            format!("{export_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
            format!("{export_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
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

    // return the formatted signature
    FormattedSignature { text, kind }
}

/// Format a function signature with parameters and return type.
#[allow(clippy::too_many_arguments)]
fn format_function(
    name: &str,
    signature: &dir::FunctionSignature,
    export_prefix: &str,
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    // resolve the async prefix
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // format generic parameters
    let generics_text = signature
        .generics
        .as_ref()
        .map(|g| format_generics(g, artifacts, module_id, dir_tree, types, modules, strings))
        .unwrap_or_default();

    // format dynamic parameters
    let parameters_text = format_parameters(
        signature.this_parameter,
        &signature.dynamic_parameters,
        artifacts,
        module_id,
        dir_tree,
        types,
        modules,
        strings,
    );

    // format return type
    let return_text = signature
        .return_type
        .and_then(|return_node| {
            let node_id = dir::GlobalNodeIdAny {
                module_id,
                local_id: return_node.into(),
            };
            types.get_declared_or_inferred_type_id(node_id)
        })
        .map(|type_id| {
            format!(
                ": {}",
                format_local_type(type_id, artifacts, types, modules, strings)
            )
        })
        .unwrap_or_default();

    // return the formatted function signature
    format!(
        "{export_prefix}{async_prefix}function {name}{generics_text}({parameters_text}){return_text}"
    )
}

fn format_import_alias(name: &str, kind: dir::DependencyKind, export_prefix: &str) -> String {
    let import_prefix = if kind == dir::DependencyKind::Type {
        "import type "
    } else {
        "import "
    };
    format!("{export_prefix}{import_prefix}{name}")
}

/// Format a function or method call signature without declaration keywords.
#[allow(clippy::too_many_arguments)]
pub fn format_call_signature(
    name: &str,
    signature: &dir::FunctionSignature,
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
    include_this: bool,
) -> FormattedCallSignature {
    // resolve the async prefix
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // format generic parameters
    let generics_text = signature
        .generics
        .as_ref()
        .map(|g| format_generics(g, artifacts, module_id, dir_tree, types, modules, strings))
        .unwrap_or_default();

    // choose dynamic parameters
    let this_parameter = if include_this {
        signature.this_parameter
    } else {
        None
    };

    // format parameter labels
    let parameter_labels = format_parameter_labels(
        this_parameter,
        &signature.dynamic_parameters,
        artifacts,
        module_id,
        dir_tree,
        types,
        modules,
        strings,
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
            types.get_declared_or_inferred_type_id(node_id)
        })
        .map(|type_id| {
            format!(
                ": {}",
                format_local_type(type_id, artifacts, types, modules, strings)
            )
        })
        .unwrap_or_default();

    // build the call signature label
    let label = format!("{async_prefix}{name}{generics_text}({parameters_text}){return_text}");

    // return the call signature details
    FormattedCallSignature {
        label,
        parameters: parameter_labels,
    }
}

/// Format generic type parameters.
fn format_generics(
    generics: &dir::Generics,
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    // read generic parameters
    let Some(parameters) = &generics.static_parameters else {
        return String::new();
    };
    if parameters.is_empty() {
        return String::new();
    }

    // format each generic parameter
    let formatted: Vec<_> = parameters
        .iter()
        .map(|parameter_id| {
            format_parameter(
                *parameter_id,
                artifacts,
                module_id,
                dir_tree,
                types,
                modules,
                strings,
            )
        })
        .collect();

    // return the formatted generics list
    format!("<{}>", formatted.join(", "))
}

/// Format function parameters with their types.
fn format_parameters(
    this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    // format labels for parameters
    let formatted = format_parameter_labels(
        this_parameter,
        parameters,
        artifacts,
        module_id,
        dir_tree,
        types,
        modules,
        strings,
    );

    // return the joined parameter labels
    formatted.join(", ")
}

/// Format parameter labels in declared order.
fn format_parameter_labels(
    this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> Vec<String> {
    // collect formatted parameter labels
    let mut formatted: Vec<String> = Vec::new();

    // add the explicit this parameter when present
    if let Some(this_parameter) = this_parameter {
        let this_text = format_parameter(
            this_parameter,
            artifacts,
            module_id,
            dir_tree,
            types,
            modules,
            strings,
        );
        formatted.push(format!("this: {this_text}"));
    }

    // add remaining parameters in order
    formatted.extend(parameters.iter().map(|parameter_id| {
        format_parameter(
            *parameter_id,
            artifacts,
            module_id,
            dir_tree,
            types,
            modules,
            strings,
        )
    }));

    // return the formatted labels
    formatted
}

/// Format a single parameter with its type annotation.
fn format_parameter(
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    artifacts: &ArtifactRegistry,
    module_id: ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    // read the parameter node
    let parameter = dir_tree.get::<dir::Parameter>(parameter_id);

    // resolve the parameter name
    let name = match parameter {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::Pattern { .. } => "_".to_string(),
        dir::Parameter::VariadicNamed { name, .. } => {
            format!("...{}", &*strings.get(*name))
        }
        dir::Parameter::VariadicPattern { .. } => "...<pattern>".to_string(),
    };

    // try to get the inferred type for this parameter
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: parameter_id.into(),
    };

    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, artifacts, types, modules, strings);
        format!("{name}: {type_text}")
    } else {
        name
    }
}

/// Format a symbol's signature for hover display.
pub fn format_symbol_signature(
    symbol_id: dir::GlobalSymbolId,
    artifacts: &ArtifactRegistry,
    modules: &ModuleRegistry,
    strings: &StringPool,
    profile: ProfileId,
) -> Option<FormattedSignature> {
    // resolve module dir data
    let module = modules.get(symbol_id.module_id);
    let module = module.read();
    let dir = artifacts.dir_snapshot(symbol_id.module_id, profile)?;
    let declaration_ref = {
        let symbols = &dir.symbols;
        let symbol = symbols.get_symbol(symbol_id.into_local());
        symbol.primary_declaration?
    };

    // read the declaration from the tree
    let dir_tree = &dir.tree;
    let declaration_id = declaration_ref.local_id.try_into().ok()?;
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);

    // resolve type table and metadata
    let types = &dir.types;
    let kind = declaration.kind_name();
    let descriptor = declaration.descriptor();

    // resolve the declaration name
    let name = descriptor
        .name
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());

    // resolve the export prefix
    let export_prefix = if descriptor.export.is_some() {
        "export "
    } else {
        ""
    };

    // resolve the module id for global ids
    let module_id = module.id;

    // format the signature text by declaration kind
    let text = match declaration {
        dir::Declaration::Function { signature, .. } => format_function(
            &name,
            signature,
            export_prefix,
            artifacts,
            module_id,
            &dir_tree,
            &types,
            modules,
            strings,
        ),
        dir::Declaration::Global { .. } => {
            let declare_prefix = if descriptor.kind == dir::DeclarationKind::Declaration {
                "declare "
            } else {
                ""
            };
            format!("{export_prefix}{declare_prefix}global")
        }
        dir::Declaration::ImportAlias { kind, .. } => {
            format_import_alias(&name, *kind, export_prefix)
        }
        dir::Declaration::Struct { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
            format!("{export_prefix}struct {name}{generics_text}")
        }
        dir::Declaration::Class { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
            format!("{export_prefix}class {name}{generics_text}")
        }
        dir::Declaration::Interface { generics, .. } => {
            let generics_text = format_generics(
                generics, artifacts, module_id, &dir_tree, &types, modules, strings,
            );
            format!("{export_prefix}interface {name}{generics_text}")
        }
        dir::Declaration::Enum { .. } => format!("{export_prefix}enum {name}"),
        dir::Declaration::Type { .. } => format!("{export_prefix}type {name}"),
        dir::Declaration::Namespace { .. } => format!("{export_prefix}namespace {name}"),
        dir::Declaration::Extension { .. } => format!("{export_prefix}extension {name}"),
    };

    // return the formatted signature
    Some(FormattedSignature { text, kind })
}
