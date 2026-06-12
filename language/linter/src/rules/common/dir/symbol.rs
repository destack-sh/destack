use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::LintModuleContext;

/// A matched decorator attached to a symbol declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDecorator {
    /// The string arguments supplied to the decorator.
    pub arguments: Vec<Option<String>>,
}

/// Return candidate symbols for an expression usage site.
pub fn expression_candidate_symbols(
    local_module_id: ModuleId,
    local_resolutions: &dir::ResolutionTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::GlobalSymbolId> {
    let mut symbols = Vec::new();

    // include lexical reference targets
    let global_expression_id = expression_id.into_global_any(local_module_id);
    if let Some(symbol_id) = local_resolutions.symbol_resolution(global_expression_id) {
        push_unique_symbol(&mut symbols, symbol_id);
    }

    // include member and call target symbols
    for symbol_id in resolution_target_symbols(local_resolutions, global_expression_id) {
        push_unique_symbol(&mut symbols, symbol_id);
    }

    symbols
}

/// Map decorators found on expression candidate symbols.
pub fn expression_symbol_decorator_map<T>(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    decorator_symbol: dir::GlobalSymbolId,
    mut map: impl FnMut(&SymbolDecorator) -> Option<T>,
) -> Option<T> {
    let symbols = expression_candidate_symbols(ctx.module_id(), ctx.resolutions, expression_id);

    for symbol_id in symbols {
        let decorators = symbol_decorators_for(ctx, symbol_id, decorator_symbol);

        for decorator in decorators {
            if let Some(value) = map(&decorator) {
                return Some(value);
            }
        }
    }

    None
}

/// Return true when an expression candidate symbol has a matching decorator.
pub fn expression_has_symbol_decorator(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    decorator_symbol: dir::GlobalSymbolId,
) -> bool {
    expression_symbol_decorator_map(ctx, expression_id, decorator_symbol, |_| Some(())).is_some()
}

/// Collect member, call, and construct target symbols for one resolved node.
pub fn resolution_target_symbols(
    resolutions: &dir::ResolutionTable<'_>,
    node_id: dir::GlobalNodeIdAny,
) -> Vec<dir::GlobalSymbolId> {
    let mut symbols = Vec::new();

    if let Some(resolution) = resolutions.member_resolution(node_id) {
        match &resolution.target {
            dir::MemberTarget::Symbol(candidate) => {
                push_unique_symbol(&mut symbols, candidate.symbol);
            }
            dir::MemberTarget::Overloaded(candidates) | dir::MemberTarget::Union(candidates) => {
                for candidate in candidates {
                    push_unique_symbol(&mut symbols, candidate.symbol);
                }
            }
            dir::MemberTarget::Builtin(_) | dir::MemberTarget::Field(_) => {}
        }
    }

    if let Some(resolution) = resolutions.call_resolution(node_id) {
        match &resolution.target {
            dir::CallTarget::Symbol(candidate) => {
                push_unique_symbol(&mut symbols, candidate.symbol);
            }
            dir::CallTarget::Union(candidates) => {
                for candidate in candidates {
                    push_unique_symbol(&mut symbols, candidate.symbol);
                }
            }
            dir::CallTarget::Builtin(_) | dir::CallTarget::Expression { .. } => {}
        }
    }

    if let Some(resolution) = resolutions.construct_resolution(node_id) {
        let symbol = resolution.target.symbol();

        push_unique_symbol(&mut symbols, symbol);
    }

    symbols
}

/// Insert a symbol if it is not already present.
fn push_unique_symbol(symbols: &mut Vec<dir::GlobalSymbolId>, symbol: dir::GlobalSymbolId) {
    if symbols.contains(&symbol) {
        return;
    }
    symbols.push(symbol);
}

/// Read matching decorators in one module-local DIR snapshot.
fn symbol_decorators_in_module(
    module_id: ModuleId,
    tree: &dir::Tree,
    strings: &StringPool,
    symbols: &dir::BindingTable<'_>,
    resolutions: &dir::ResolutionTable<'_>,
    symbol_id: dir::LocalSymbolId,
    decorator_symbol: dir::GlobalSymbolId,
) -> Vec<SymbolDecorator> {
    let symbol = symbols.get_symbol(symbol_id);
    let Some(declaration) = symbol.declaration else {
        return Vec::new();
    };
    if declaration.module_id != module_id {
        return Vec::new();
    }

    let mut decorators = Vec::new();
    for decorator_id in tree.get_decorators(declaration.local_id.id) {
        let decorator = tree.get(decorator_id);
        let Some(decorator) = symbol_decorator_from_expression(
            module_id,
            tree,
            strings,
            resolutions,
            decorator,
            decorator_symbol,
        ) else {
            continue;
        };

        decorators.push(decorator);
    }

    decorators
}

/// Read one decorator when its callee resolves to the requested symbol.
fn symbol_decorator_from_expression(
    module_id: ModuleId,
    tree: &dir::Tree,
    strings: &StringPool,
    resolutions: &dir::ResolutionTable<'_>,
    decorator: &dir::Decorator,
    decorator_symbol: dir::GlobalSymbolId,
) -> Option<SymbolDecorator> {
    let expression_id = unwrap_parenthesized_expression(tree, decorator.expression);
    let expression = tree.get(expression_id);
    let (callee_id, arguments) = match expression {
        dir::Expression::Call {
            left, arguments, ..
        } => (
            unwrap_parenthesized_expression(tree, *left),
            Some(arguments.as_slice()),
        ),
        _ => (expression_id, None),
    };

    if !decorator_expression_matches(
        module_id,
        resolutions,
        expression_id,
        callee_id,
        decorator_symbol,
    ) {
        return None;
    }

    let arguments = decorator_string_arguments(tree, strings, arguments);

    Some(SymbolDecorator { arguments })
}

/// Unwrap parenthesized decorator expressions.
fn unwrap_parenthesized_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let mut current = expression_id;
    loop {
        let expression = tree.get(current);
        let dir::Expression::Parenthesized { expression } = expression else {
            return current;
        };
        current = *expression;
    }
}

/// Check both the decorator call and callee for a resolved decorator symbol.
fn decorator_expression_matches(
    module_id: ModuleId,
    resolutions: &dir::ResolutionTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    callee_id: dir::LocalNodeId<dir::Expression>,
    decorator_symbol: dir::GlobalSymbolId,
) -> bool {
    let expression_node = expression_id.into_global_any(module_id);
    if resolutions
        .symbol_resolution(expression_node)
        .is_some_and(|symbol| symbol == decorator_symbol)
    {
        return true;
    }

    let callee_node = callee_id.into_global_any(module_id);
    resolutions
        .symbol_resolution(callee_node)
        .is_some_and(|symbol| symbol == decorator_symbol)
}

/// Read string arguments, using None as the unlabeled marker.
fn decorator_string_arguments(
    tree: &dir::Tree,
    strings: &StringPool,
    arguments: Option<&[dir::LocalNodeId<dir::Argument>]>,
) -> Vec<Option<String>> {
    let Some(arguments) = arguments else {
        return vec![None];
    };
    if arguments.is_empty() {
        return vec![None];
    }

    let mut values = Vec::new();
    for argument_id in arguments {
        let argument = tree.get(*argument_id);
        let dir::Argument::Positional { value, .. } = argument else {
            continue;
        };
        let expression = tree.get(*value);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = expression else {
            continue;
        };

        values.push(Some(strings.get(*value).to_string()));
    }

    values
}

/// Read one symbol entry from local or remote module tables.
pub fn symbol_for(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::Symbol> {
    if symbol_id.module_id == ctx.module_id() {
        return Some(ctx.symbols.get_symbol(symbol_id.local_id).clone());
    }

    let module = ctx.session.checked_module(symbol_id.module_id)?;

    Some(module.symbols.get_symbol(symbol_id.local_id).clone())
}

/// Read matching decorators for a symbol.
pub fn symbol_decorators_for(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    decorator_symbol: dir::GlobalSymbolId,
) -> Vec<SymbolDecorator> {
    if symbol_id.module_id == ctx.module_id() {
        return symbol_decorators_in_module(
            ctx.module_id(),
            ctx.dir.tree(),
            ctx.strings,
            &ctx.symbols,
            ctx.resolutions,
            symbol_id.local_id,
            decorator_symbol,
        );
    }

    let Some(module) = ctx.session.checked_module(symbol_id.module_id) else {
        return Vec::new();
    };

    symbol_decorators_in_module(
        symbol_id.module_id,
        &module.parsed.tree,
        &module.strings,
        &module.symbols,
        &module.resolutions,
        symbol_id.local_id,
        decorator_symbol,
    )
}

/// Read the declaration id for a symbol.
pub fn symbol_declaration_for(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalNodeIdAny> {
    let symbol = symbol_for(ctx, symbol_id)?;
    symbol.declaration
}

/// Resolve one local initializer expression for a symbol when available.
pub fn symbol_initializer_expression(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // resolve the declaration for this symbol
    let declaration_id = symbol_declaration_for(ctx, symbol_id)?;
    if declaration_id.module_id != ctx.module_id() {
        return None;
    }

    // resolve the declaration initializer in the local tree
    declaration_initializer_expression(
        &ctx.symbols,
        ctx.dir.tree(),
        declaration_id,
        symbol_id.local_id,
    )
}

/// Resolve one initializer expression from a symbol declaration node.
pub fn declaration_initializer_expression(
    symbols: &dir::BindingTable<'_>,
    tree: &dir::Tree,
    declaration_id: dir::GlobalNodeIdAny,
    symbol_id: dir::LocalSymbolId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match declaration_id.local_id.ty {
        // read direct declarator initializers
        dir::NodeType::Declarator => {
            let declarator = tree.get(declaration_id.into_local_typed::<dir::Declarator>());
            declarator.value
        }

        // resolve pattern and pattern-field declarations through their declarator
        dir::NodeType::Pattern | dir::NodeType::PatternField => {
            let declarator_id = enclosing_declarator(tree, declaration_id.local_id.id)?;
            let declarator = tree.get(declarator_id);
            declarator.value
        }

        // map property declarations to value or default initializers
        dir::NodeType::Property => {
            let property = tree.get(declaration_id.into_local_typed::<dir::Property>());
            match property {
                dir::Property::Field { value, .. } => Some(*value),
                dir::Property::Method { .. } => None,
                dir::Property::Spread { value, .. } => Some(*value),
                dir::Property::Error => None,
            }
        }

        // map member declarations to value-like initializers
        dir::NodeType::Member => {
            let member = tree.get(declaration_id.into_local_typed::<dir::Member>());
            match member {
                dir::Member::Field { default, .. } => *default,
                dir::Member::AssociatedConst { value, .. } => *value,
                dir::Member::Method { .. }
                | dir::Member::AssociatedType { .. }
                | dir::Member::StaticBlock { .. }
                | dir::Member::ComptimeBlock { .. }
                | dir::Member::Error => None,
            }
        }

        // map parameters to default value expressions
        dir::NodeType::Parameter => {
            let parameter = tree.get(declaration_id.into_local_typed::<dir::Parameter>());
            match parameter {
                dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => {
                    *default
                }
                dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
                    None
                }
                dir::Parameter::Error => None,
            }
        }

        // handle let and using declarations that point at the root expression node
        dir::NodeType::Expression => {
            let expression = tree.get(declaration_id.into_local_typed::<dir::Expression>());
            match expression {
                dir::Expression::Let { declarators, .. }
                | dir::Expression::Using { declarators, .. } => declarators.iter().find_map(|id| {
                    let declarator = tree.get(*id);
                    (symbols
                        .declaration_symbol(declarator.pattern.into_global_any(symbols.module_id))
                        == Some(symbol_id))
                    .then_some(declarator.value)
                    .flatten()
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Find the nearest declarator parent for one node id.
fn enclosing_declarator(
    tree: &dir::Tree,
    mut node_id: u32,
) -> Option<dir::LocalNodeId<dir::Declarator>> {
    loop {
        let parent = tree.get_parent(node_id)?;
        if parent.ty == dir::NodeType::Declarator {
            return Some(parent.into_typed());
        }

        node_id = parent.id;
    }
}
