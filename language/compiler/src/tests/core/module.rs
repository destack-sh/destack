use std::sync::Arc;

use tspp_artifact::{ArtifactDependency, DirParsed, DirParsedFile, SourceDependency};
use tspp_dir as dir;
use tspp_parser::{CommentRetention, ParseOptions, Parser};
use tspp_repository::{Module, Repository, Revision};
use tspp_source::{File, LanguageType, ProfileId, Span};

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
        dir::Expression::Literal(dir::Literal::Boolean(false)),
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
    let parser = Parser::new(
        source_file.clone(),
        language_type,
        tree_in,
        ParseOptions {
            comment_retention: CommentRetention::Documentation,
            ..ParseOptions::default()
        },
    );
    let parse = parser.parse();

    // require the source to parse cleanly
    let diagnostics = parse.diagnostics();
    assert!(
        diagnostics.is_empty(),
        "compiler source should parse cleanly: {diagnostics:?}"
    );

    // intern parsed strings in the test repository
    repository.string_pool().extend(&parse.strings);

    // preserve semantic tokens
    let tokens = parse.tokens;
    let comments = parse.comments;
    let roots = parse.roots;

    // restore the shared tree
    *tree = parse.tree;
    let anchor_expression = tree.insert(
        dir::Expression::Literal(dir::Literal::Boolean(false)),
        Span::empty(source_file.id),
    );

    DirParsedFile {
        file_id: source_file.id,
        aliases,
        roots,
        tokens,
        comments,
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
        .map(|file| {
            let blob = repository
                .file_blob(revision, file.file_id)
                .expect("test file Blob should resolve")
                .expect("test file Blob should exist");

            ArtifactDependency::Source(SourceDependency::file(file.file_id, blob.id))
        })
        .collect()
}
