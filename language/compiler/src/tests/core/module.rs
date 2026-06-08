use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactPathState, DirParsed, DirParsedFile};
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, FileContentId, FileId, LanguageType, ProfileId, Span};
use destack_repository::{Module, Repository, Revision};

/// One code module in a compiler test.
#[derive(Debug)]
pub(crate) struct TestModule {
    /// The original source.
    pub(crate) source: String,
    /// The repository module.
    pub(crate) module: Arc<Module>,
    /// The effective module profile.
    pub(crate) profile: ProfileId,
    /// The parsed DIR artifact.
    pub(crate) dir_parsed: DirParsed,
}

/// Parse one test module into a parsed DIR artifact.
pub(crate) fn parse_module(
    module: &Module,
    repository: &Repository,
    revision: Revision,
) -> DirParsed {
    let mut tree = dir::Tree::new(module.id);
    let mut files = Vec::with_capacity(module.files.len());

    // parse each contributing file into one module tree
    for module_file in &module.files {
        let source_file = repository
            .file(revision, module_file.file_id)
            .expect("test file lookup should work")
            .expect("test file should exist");
        let parsed_file = parse_module_file(
            module_file.aliases.clone(),
            source_file,
            repository,
            &mut tree,
        );

        files.push(parsed_file);
    }

    // preserve a stable module-level anchor
    let anchor_expression = tree.insert(
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
        Span::empty(module.file_id),
    );

    DirParsed::new(tree, files, anchor_expression)
}

/// Parse one physical module file into a shared parsed DIR tree.
fn parse_module_file(
    aliases: Vec<String>,
    source_file: Arc<File>,
    repository: &Repository,
    tree: &mut dir::Tree,
) -> DirParsedFile {
    let language_type =
        LanguageType::try_from(source_file.ty).expect("test code file should have a language type");

    // parse the file with the shared tree
    let tree_in = std::mem::replace(tree, dir::Tree::new(tree.module_id));
    let mut parser = Parser::lex_module_tree_with_options(
        source_file.clone(),
        language_type,
        ParserOptions::default(),
        repository.string_pool().clone(),
        tree_in,
    );
    let roots = parser.parse();
    let diagnostics = parser.diagnostics();
    assert!(
        diagnostics.is_empty(),
        "compiler source should parse cleanly: {diagnostics:?}"
    );

    // append token side data
    let (tokens, side_tokens) = parser.take_tokens();

    // restore the shared tree
    *tree = parser.tree;
    let anchor_expression = tree.insert(
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
        Span::empty(source_file.id),
    );

    DirParsedFile {
        file_id: source_file.id,
        aliases,
        roots,
        tokens,
        side_tokens,
        anchor_expression,
    }
}

/// Return source content dependencies for one parsed module.
pub(crate) fn parsed_dependencies(
    repository: &Repository,
    revision: Revision,
    module: &Module,
) -> Vec<ArtifactDependency> {
    module
        .files
        .iter()
        .flat_map(|file| {
            let content = file_content_id(repository, revision, file.file_id);

            [
                ArtifactDependency::path_state(file.file_id, ArtifactPathState::File),
                ArtifactDependency::file_content(file.file_id, content),
            ]
        })
        .collect()
}

/// Return one file content id from the repository.
fn file_content_id(repository: &Repository, revision: Revision, file: FileId) -> FileContentId {
    repository
        .file_content_id(revision, file)
        .expect("test file content should resolve")
        .expect("test file content should exist")
}
