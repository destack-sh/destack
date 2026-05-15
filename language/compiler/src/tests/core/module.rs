use std::path::Path;
use std::sync::Arc;

use destack_artifact::{ArtifactDependency, DirParsed};
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{FileContentId, FileId, FileType, ProfileId, Span};
use destack_workspace::{Module, Repository, Revision};

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
pub(crate) fn parse_module(module: &Module, source: &str, repository: &Repository) -> DirParsed {
    let source_file = Arc::new(destack_source::File::from_text(
        module.file_id,
        module
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| module.uri.to_string()),
        module.uri.clone(),
        module.path.clone(),
        FileType::from_path_or_unknown(
            module
                .path
                .as_deref()
                .unwrap_or_else(|| Path::new("main.ds")),
        ),
        source.to_string(),
    ));

    // parse source
    let mut parser = Parser::lex_file_with_options(
        source_file,
        module.code_language_type(),
        ParserOptions::default(),
        repository.string_pool().clone(),
    );
    let roots = parser.parse();
    let diagnostics = parser.diagnostics.collect();
    assert!(
        diagnostics.is_empty(),
        "compiler source should parse cleanly: {diagnostics:?}"
    );

    // build parsed artifact
    let (tokens, side_tokens) = parser.take_tokens();
    parser.tree.module_id = module.id;
    let anchor_expression = parser.tree.insert(
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
        Span::empty(module.file_id),
    );

    DirParsed::from_tree(parser.tree, roots, tokens, side_tokens, anchor_expression)
}

/// Return the source content dependency for one parsed module.
pub(crate) fn parsed_dependency(
    repository: &Repository,
    revision: Revision,
    module: &Module,
) -> ArtifactDependency {
    let content = file_content_id(repository, revision, module.file_id);

    ArtifactDependency::FileContent {
        file: module.file_id,
        content,
    }
}

/// Return one file content id from the repository.
fn file_content_id(repository: &Repository, revision: Revision, file: FileId) -> FileContentId {
    repository
        .file_content_id(revision, file)
        .expect("test file content should resolve")
        .expect("test file content should exist")
}
