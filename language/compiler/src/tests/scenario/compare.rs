use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{Dumper, DumperOptions, NodeVisitor};
use destack_source::{FileId, ModuleId, ModuleVersion};
use destack_workspace::{Ast, DirPrepared, DirResolved};

/// Build one stable profile key for scenario cache tests.
pub(crate) fn test_profile_key() -> destack_workspace::ProfileKey {
    use destack_workspace::{
        EnvSnapshot, OutputFormat, Platform, ProfileFlags, ProfileKey, Runtime,
    };

    ProfileKey::new(
        OutputFormat::Js,
        Runtime::Node,
        Platform::Web,
        None,
        None,
        None,
        Vec::new(),
        false,
        false,
        false,
        EnvSnapshot::Whitelist {
            keys: Vec::new(),
            hash: 0,
        },
        ProfileFlags::default(),
    )
}

/// Append text to one loaded file and bump its version.
pub(crate) fn append_file_text(
    program: &destack_workspace::Program,
    file_id: destack_source::FileId,
    suffix: &str,
) {
    use destack_source::{File, FileContent};

    let file = program.files.get(file_id);
    let FileContent::Text { content } = &file.content else {
        panic!("expected text file");
    };
    let content = format!("{content}{suffix}");
    let file = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        content,
    )
    .with_version(file.version.next());

    program.files.replace(file);
}

/// Normalize one AST for stable cross-session comparison.
pub(crate) fn normalize_ast(mut ast: Ast) -> Ast {
    ast.id = ModuleId::EPHEMERAL;
    ast.version = ModuleVersion::INITIAL;
    ast.tree.source_map.rebind_file(FileId::new(0));

    for token in &mut ast.tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    for token in &mut ast.side_tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    ast
}

/// Dump one prepared DIR node surface deterministically.
pub(crate) fn dump_dir_prepared_nodes(strings: &StringPool, dir: &DirPrepared) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());

    for expression_id in dir.roots.iter().copied() {
        let expression = dir.tree.get(expression_id);
        dumper.visit_expression(&dir.tree, expression_id, expression);
    }

    dumper.finish()
}

/// Dump one prepared DIR symbol surface deterministically.
pub(crate) fn dump_dir_prepared_symbols(strings: &StringPool, dir: &DirPrepared) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());
    let scope = dir.symbols.get_scope_by_id(dir.namespace_scope);
    dumper.visit_scope(&dir.tree, &dir.symbols, dir.namespace_scope, scope);
    dumper.finish()
}

/// Dump one resolved DIR node surface deterministically.
pub(crate) fn dump_dir_resolved_nodes(strings: &StringPool, dir: &DirResolved) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());

    for expression_id in dir.roots.iter().copied() {
        let expression = dir.tree.get(expression_id);
        dumper.visit_expression(&dir.tree, expression_id, expression);
    }

    dumper.finish()
}

/// Dump one resolved DIR symbol surface deterministically.
pub(crate) fn dump_dir_resolved_symbols(strings: &StringPool, dir: &DirResolved) -> String {
    let strings = strings.clone().into_immutable();
    let mut dumper = Dumper::new(&strings, &dir.tree, DumperOptions::default());
    let scope = dir.symbols.get_scope_by_id(dir.namespace_scope);
    dumper.visit_scope(&dir.tree, &dir.symbols, dir.namespace_scope, scope);
    dumper.finish()
}

/// Encode one AST deterministically for equality assertions.
pub(crate) fn encode_ast(ast: Ast) -> Vec<u8> {
    postcard::to_allocvec(&normalize_ast(ast))
        .unwrap_or_else(|error| panic!("failed to encode ast: {error}"))
}

/// Assert two AST values are equivalent.
pub(crate) fn assert_ast_eq(expected: Ast, actual: Ast) {
    let expected = encode_ast(expected);
    let actual = encode_ast(actual);
    assert_eq!(actual, expected);
}

/// Assert two prepared DIR values are equivalent.
pub(crate) fn assert_dir_prepared_eq(
    strings: &Arc<StringPool>,
    expected: &DirPrepared,
    actual: &DirPrepared,
) {
    let expected_nodes = dump_dir_prepared_nodes(strings, expected);
    let expected_symbols = dump_dir_prepared_symbols(strings, expected);
    let actual_nodes = dump_dir_prepared_nodes(strings, actual);
    let actual_symbols = dump_dir_prepared_symbols(strings, actual);

    assert_eq!(actual_nodes, expected_nodes);
    assert_eq!(actual_symbols, expected_symbols);
}

/// Assert two resolved DIR values are equivalent.
pub(crate) fn assert_dir_resolved_eq(
    strings: &Arc<StringPool>,
    expected: &DirResolved,
    actual: &DirResolved,
) {
    let expected_nodes = dump_dir_resolved_nodes(strings, expected);
    let expected_symbols = dump_dir_resolved_symbols(strings, expected);
    let actual_nodes = dump_dir_resolved_nodes(strings, actual);
    let actual_symbols = dump_dir_resolved_symbols(strings, actual);

    assert_eq!(actual_nodes, expected_nodes);
    assert_eq!(actual_symbols, expected_symbols);
}
