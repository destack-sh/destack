use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{DirBound, DirParsed, MemoryCacheStore};
use destack_core::StringPool;
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{
    File, FileId, FileType, LanguageType, Loader, MemoryFileSystem, ModuleId, PackageId, Span, Uri,
};
use destack_workspace::{HostEnvironment, Module, Repository};

use crate::Compiler;

/// One parsed and bound module fixture.
#[derive(Debug)]
struct BindFixture {
    /// The shared string pool used by the parsed module.
    strings: Arc<StringPool>,
    /// The source module under test.
    module: Module,
    /// The parsed DIR artifact.
    parsed: DirParsed,
    /// The bound DIR artifact.
    bound: DirBound,
}

/// Parse and bind one in-memory Destack module.
fn bind_source(source: &str) -> BindFixture {
    let strings = Arc::new(StringPool::new());
    let package_id = PackageId::from_path(Path::new("bind-test"));
    let module_id = ModuleId::from_relative_path(package_id, PathBuf::from("main.ds").as_path());
    let file_id = FileId::from_logical_str("main.ds");
    let uri = Uri::from_string("memory:///main.ds");
    let file = Arc::new(File::from_text(
        file_id,
        "main.ds".to_string(),
        uri.clone(),
        None,
        FileType::Destack,
        source.to_string(),
    ));
    let mut parser = Parser::lex_file_with_options(
        file,
        LanguageType::Destack,
        ParserOptions::default(),
        strings.clone(),
    );
    let roots = parser.parse();
    let diagnostics = parser.diagnostics.collect();
    assert!(
        diagnostics.is_empty(),
        "bind test source should parse cleanly: {diagnostics:?}"
    );

    let (tokens, side_tokens) = parser.take_tokens();
    let anchor_expression = parser.tree.insert(
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
        Span::empty(file_id),
    );
    let parsed = DirParsed::from_tree(parser.tree, roots, tokens, side_tokens, anchor_expression);
    let module = Module::blank(
        module_id,
        file_id,
        uri,
        None,
        package_id,
        Some(LanguageType::Destack),
        Loader::Destack,
    );
    let repository = Arc::new(Repository::new(
        PathBuf::new(),
        Arc::new(MemoryCacheStore::new()),
        Arc::new(MemoryFileSystem::new()),
        HostEnvironment::default(),
    ));
    let compiler = Compiler::new(repository);
    let bound = compiler.bind_dir_parsed(&module, &parsed);

    BindFixture {
        strings,
        module,
        parsed,
        bound,
    }
}

/// Return symbols keyed by one textual name.
fn symbols_named(fixture: &BindFixture, name: &str) -> Vec<dir::LocalSymbolId> {
    fixture
        .bound
        .bindings
        .symbol_ids()
        .filter(|symbol_id| {
            let symbol = fixture.bound.bindings.get_symbol(*symbol_id);
            let Some(symbol_name) = symbol.name() else {
                return false;
            };

            &*fixture.strings.get(symbol_name) == name
        })
        .collect()
}

/// Return one symbol keyed by one textual name.
fn symbol_named(fixture: &BindFixture, name: &str) -> dir::LocalSymbolId {
    let symbols = symbols_named(fixture, name);
    assert_eq!(symbols.len(), 1, "expected exactly one symbol named {name}");

    symbols[0]
}

/// Return one scope binding list filtered to one textual key.
fn scope_bindings_named(
    fixture: &BindFixture,
    scope_id: dir::LocalScopeId,
    name: &str,
) -> Vec<dir::ScopeBinding> {
    let scope = fixture.bound.bindings.get_scope_by_id(scope_id);
    scope
        .bindings
        .iter()
        .copied()
        .filter(|binding| {
            let Some(name_id) = binding.key.and_then(|key| key.name()) else {
                return false;
            };

            &*fixture.strings.get(name_id) == name
        })
        .collect()
}

/// Return one node's bound scope.
fn scope_for_node<T: dir::Node>(
    fixture: &BindFixture,
    node_id: dir::LocalNodeId<T>,
) -> dir::LocalScope {
    let node_id = node_id.into_any();

    fixture
        .bound
        .bindings
        .scope_for_node(node_id.into_global(fixture.module.id))
        .unwrap_or_else(|| panic!("missing scope for {node_id:?}"))
}

/// Return one node's declared symbol.
fn symbol_for_node<T: dir::Node>(
    fixture: &BindFixture,
    node_id: dir::LocalNodeId<T>,
) -> dir::LocalSymbolId {
    let node_id = node_id.into_any();

    fixture
        .bound
        .bindings
        .symbol_for_declaration(node_id.into_global(fixture.module.id))
        .unwrap_or_else(|| panic!("missing symbol for {node_id:?}"))
}

/// Return the scope owned by one symbol.
fn scope_owned_by(fixture: &BindFixture, owner: dir::LocalSymbolId) -> dir::LocalScopeId {
    fixture
        .bound
        .bindings
        .scope_ids()
        .find(|scope_id| fixture.bound.bindings.get_scope_by_id(*scope_id).owner == Some(owner))
        .unwrap_or_else(|| panic!("missing scope owned by {owner:?}"))
}

#[test]
fn test_bind_module_surface_and_rebinding() {
    let fixture = bind_source(
        r#"
import { dep as local, type TypeDep } from "dep"

let x: int32 = 1
let x = x + 1

function wrap<T>(value: T): T {
    let value = value
    return value
}

type Element<T> = T extends Array<infer U> ? U : T
"#,
    );

    // module bindings
    let x_symbols = symbols_named(&fixture, "x");
    assert_eq!(x_symbols.len(), 2);
    let x_bindings = scope_bindings_named(&fixture, fixture.bound.namespace_scope, "x");
    assert_eq!(x_bindings.len(), 2);
    assert_eq!(x_bindings[0].symbol, x_symbols[0]);
    assert_eq!(x_bindings[1].symbol, x_symbols[1]);

    // import bindings
    let local_symbol = fixture
        .bound
        .bindings
        .get_symbol(symbol_named(&fixture, "local"));
    assert_eq!(local_symbol.space, dir::SymbolSpace::Value);
    let type_dep_symbol = fixture
        .bound
        .bindings
        .get_symbol(symbol_named(&fixture, "TypeDep"));
    assert_eq!(type_dep_symbol.space, dir::SymbolSpace::Type);

    // synthetic exports
    let has_synthetic_default = fixture
        .bound
        .bindings
        .symbols()
        .any(|symbol| symbol.export_kind == Some(dir::ExportKind::Default));
    assert!(!has_synthetic_default);
}

#[test]
fn test_bind_generics_parameters_and_infer() {
    let fixture = bind_source(
        r#"
function wrap<T>(value: T): T {
    let value = value
    return value
}

type Element<T> = T extends Array<infer U> ? U : T
"#,
    );

    // generic parameter symbols
    let generic_parameters = fixture
        .parsed
        .tree
        .iter_nodes::<dir::GenericParameter>()
        .map(|parameter_id| symbol_for_node(&fixture, parameter_id))
        .collect::<Vec<_>>();
    assert_eq!(generic_parameters.len(), 2);
    for symbol_id in generic_parameters {
        let symbol = fixture.bound.bindings.get_symbol(symbol_id);
        assert!(symbol.is_generic_parameter());
    }

    // function parameter scope
    let wrap_symbol = symbol_named(&fixture, "wrap");
    let wrap_scope = scope_owned_by(&fixture, wrap_symbol);
    let value_symbols = symbols_named(&fixture, "value");
    assert_eq!(value_symbols.len(), 2);
    let parameter = value_symbols
        .iter()
        .map(|symbol_id| fixture.bound.bindings.get_symbol(*symbol_id))
        .find(|symbol| symbol.scope.id == wrap_scope)
        .expect("expected parameter in function scope");
    assert_eq!(parameter.form, dir::SymbolForm::Variable);

    // infer declarations
    let infer_symbols = symbols_named(&fixture, "U");
    assert_eq!(infer_symbols.len(), 1);
    let infer_symbol = fixture.bound.bindings.get_symbol(infer_symbols[0]);
    assert_eq!(infer_symbol.form, dir::SymbolForm::TypeAlias);
    assert!(infer_symbol.declaration.is_some());
}

#[test]
fn test_bind_loop_scopes() {
    let fixture = bind_source(
        r#"
for (let index = 0; index < 10; index = index + 1) {
    let index = index
}
"#,
    );
    let for_id = fixture
        .parsed
        .tree
        .iter_nodes_of_type::<dir::Expression>()
        .find_map(|(expression_id, expression)| {
            matches!(expression, dir::Expression::For { .. }).then_some(expression_id)
        })
        .expect("expected for expression");
    let for_scope = scope_for_node(&fixture, for_id);
    let index_bindings = scope_bindings_named(&fixture, for_scope.id, "index");

    assert_eq!(index_bindings.len(), 1);
}

#[test]
fn test_bind_if_let_scope() {
    let fixture = bind_source(
        r#"
let output = if (let Some(value) = maybe) {
    value
} else {
    0
}
"#,
    );
    let if_id = fixture
        .parsed
        .tree
        .iter_nodes_of_type::<dir::Expression>()
        .find_map(|(expression_id, expression)| {
            matches!(
                expression,
                dir::Expression::If {
                    condition: dir::IfCondition::Let { .. },
                    ..
                }
            )
            .then_some(expression_id)
        })
        .expect("expected if let expression");
    let if_scope = scope_for_node(&fixture, if_id);
    let value_bindings = scope_bindings_named(&fixture, if_scope.id, "value");

    assert_eq!(value_bindings.len(), 1);
}
