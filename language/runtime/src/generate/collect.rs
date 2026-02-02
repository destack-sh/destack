use destack_base::StringPool;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    self as dir, Annotation, Argument, Declaration, Expression, GlobalSymbolId, ScalarLiteral,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use crate::format::{
    binding_type_symbols, collect_binding_params, collect_binding_return, format_declared_signature,
};
use crate::model::{BindingCatalog, BindingEntry, BindingReturn};

/// Binding metadata extracted from a declaration node.
#[derive(Debug, Clone)]
struct BindingRecord {
    /// Fully qualified extern binding name.
    extern_name: String,
    /// Canonical signature string for stability checks.
    signature: String,
    /// Parameter metadata payload.
    params: Vec<crate::model::BindingParam>,
    /// Return binding type for generated wrappers.
    return_binding: BindingReturn,
}

/// Collect platform bindings from builtin modules.
pub(crate) fn collect_platform_bindings(
    program: &Program,
    strings: &StringPool,
    profile_id: ProfileId,
    platform_modules: &[ModuleId],
) -> BindingCatalog {
    // resolve the canonical binding decorator symbol
    let binding_decorator_symbol = binding_decorator_symbol_id(program, profile_id);
    let binding_symbols = binding_type_symbols(program, profile_id);

    // collect bindings by domain
    let mut domains: BindingCatalog = BindingCatalog::default();

    // visit builtin modules and extract binding annotations
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.read();
        let dir = module.dir(profile_id);
        let tree = dir.tree.read();
        let types = dir.types.read();
        let symbols = dir.symbols.read();

        // scan expressions for binding declarations
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let binding_argument = binding_decorator_value(
                &tree,
                expression_id.into_any(),
                strings,
                binding_decorator_symbol,
            );
            let Some(binding_argument) = binding_argument else {
                continue;
            };
            let declaration_id = declaration_from_expression(&tree, expression_id, expression);
            let Some(declaration_id) = declaration_id else {
                continue;
            };

            // filter to function declarations
            let declaration = tree.get::<Declaration>(declaration_id);
            let Declaration::Function { signature, .. } = declaration else {
                continue;
            };

            let symbol = symbols.get_symbol(declaration.symbol());
            let extern_name = binding_argument
                .or_else(|| symbol.name().map(|name| strings.get(name).to_string()));

            // format the canonical signature for the declaration
            let signature_text = format_declared_signature(
                declaration_id,
                declaration,
                &module,
                &program.modules,
                strings,
                profile_id,
            );

            // collect parameter and return metadata from the AST
            let domain = extern_name
                .as_deref()
                .map(binding_domain)
                .unwrap_or_else(|| "global".to_string());

            let params = collect_binding_params(
                signature,
                module.id,
                &tree,
                &types,
                &program.modules,
                strings,
                profile_id,
                &binding_symbols,
                &domain,
            );
            let return_binding = collect_binding_return(
                declaration_id,
                signature,
                module.id,
                &tree,
                &types,
                &program.modules,
                strings,
                profile_id,
                &binding_symbols,
                &domain,
            );

            // insert parsed binding metadata into the catalog
            if let Some(entry) =
                binding_from_node(extern_name, signature_text, params, return_binding)
            {
                insert_binding(&mut domains, entry);
            }
        }
    }

    domains
}

/// Resolve the canonical binding decorator symbol for the profile.
fn binding_decorator_symbol_id(program: &Program, profile_id: ProfileId) -> GlobalSymbolId {
    let builtins = program
        .builtins
        .as_ref()
        .expect("builtins must be loaded for binding generation");
    builtins
        .items
        .get(&(profile_id, LanguageSymbol::Binding))
        .map(|item| *item)
        .unwrap_or_else(|| panic!("missing binding decorator symbol for profile {profile_id:?}"))
}

/// Return the domain portion of a binding name.
fn binding_domain(extern_name: &str) -> String {
    let mut parts = extern_name.split('.');
    let _prefix = parts.next();
    parts.next().unwrap_or("global").to_string()
}

/// Build a binding record from parsed metadata.
fn binding_from_node(
    extern_name: Option<String>,
    signature: String,
    params: Vec<crate::model::BindingParam>,
    return_binding: BindingReturn,
) -> Option<BindingRecord> {
    let extern_name = extern_name?;
    if !extern_name.starts_with("destack.") {
        return None;
    }

    Some(BindingRecord {
        extern_name,
        signature,
        params,
        return_binding,
    })
}

/// Insert a binding record into the domain catalog.
fn insert_binding(domains: &mut BindingCatalog, record: BindingRecord) {
    let domain = binding_domain(&record.extern_name);
    let domain_bindings = domains.entry(domain).or_default();

    // build a canonical entry for comparisons
    let entry = BindingEntry {
        signature: record.signature,
        params: record.params,
        return_binding: record.return_binding.binding_type,
        return_is_result: record.return_binding.is_result,
    };

    if let Some(existing) = domain_bindings.insert(record.extern_name.clone(), entry.clone())
        && existing.signature != entry.signature
    {
        panic!(
            "binding signature mismatch for {}: {:?} vs {:?}",
            record.extern_name, existing.signature, entry.signature
        );
    }
}

/// Extract the binding decorator value from a declaration expression.
fn binding_decorator_value(
    tree: &dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
    strings: &StringPool,
    binding_decorator_symbol: GlobalSymbolId,
) -> Option<Option<String>> {
    // scan annotations for the binding decorator
    let annotations = tree.get_annotations(node_id.id);
    for annotation_id in annotations {
        let annotation = tree.get::<Annotation>(annotation_id);
        let Annotation::Decorator {
            left, arguments, ..
        } = annotation
        else {
            continue;
        };
        let decorator_symbol = decorator_symbol_from_expression(tree, *left)?;
        if decorator_symbol != binding_decorator_symbol {
            continue;
        }
        return Some(decorator_string_argument(tree, arguments.as_ref(), strings));
    }
    None
}

/// Resolve the decorator symbol from an expression node.
fn decorator_symbol_from_expression(
    tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let expression = tree.get::<Expression>(expr_id);
    expression.target_symbol()
}

/// Resolve a declaration node from a binding expression.
fn declaration_from_expression(
    tree: &dir::NodeTree,
    _expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
) -> Option<dir::LocalNodeId<Declaration>> {
    match expression {
        Expression::Declaration { declaration } => Some(*declaration),
        Expression::Statement { statement } => {
            let inner = tree.get::<Expression>(*statement);
            declaration_from_expression(tree, *statement, inner)
        }
        _ => None,
    }
}

/// Extract the first string argument from a decorator payload.
fn decorator_string_argument(
    tree: &dir::NodeTree,
    arguments: Option<&Vec<dir::LocalNodeId<Argument>>>,
    strings: &StringPool,
) -> Option<String> {
    let arguments = arguments?;
    let argument_id = arguments.first()?;
    let argument = tree.get::<Argument>(*argument_id);
    let value_id = argument.value();
    let value = tree.get::<Expression>(value_id);
    let Expression::ScalarLiteral { value } = value else {
        return None;
    };
    let ScalarLiteral::String(value_id) = value else {
        return None;
    };
    Some(strings.get(*value_id).to_string())
}
