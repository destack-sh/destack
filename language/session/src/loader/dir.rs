use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependencySet, ArtifactPayload, DiagnosticRecord, DirParsed, DirParsedFile,
};
use tspp_dir::{Expression, Literal, Tree};
use tspp_parser::{CommentRetention, ParseOptions, Parser};
use tspp_repository::{Module, ModuleFile, ProviderContext};
use tspp_source::{File, LanguageType, ModuleId, Span};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
    /// Collect the source closure for one parsed DIR artifact.
    pub(crate) fn collect_dir_parsed(
        &self,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactDependencySet, SessionError> {
        let revision = attempt.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleNotTracked { module_id })?;
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.observe(self.repository().module_dependency(revision, module_id)?);

        // code modules observe every contributing source file
        if module.loader.is_code() {
            for module_file in &module.files {
                self.observe_source(revision, module_file.file_id, &mut dependencies)?;
            }
        } else {
            self.observe_source(revision, module.file_id, &mut dependencies)?;
        }

        Ok(dependencies)
    }

    /// Provide one parsed DIR artifact through the selected loader.
    pub(crate) fn provide_dir_parsed(
        &self,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactPayload, SessionError> {
        let revision = attempt.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleNotTracked { module_id })?;

        // parse real code modules only
        let dir = if module.loader.is_code() {
            self.parse_code_dir(module.as_ref(), module_id, attempt)?
        } else {
            let file = self.source_file(revision, module.file_id)?;

            Self::make_empty_dir(file.as_ref(), module_id)
        };

        Ok(ArtifactPayload::DirParsed(Arc::new(dir)))
    }

    /// Build one empty parsed DIR for non-code source.
    fn make_empty_dir(file: &File, module_id: ModuleId) -> DirParsed {
        let mut tree = Tree::new(module_id);
        let span = Span::empty(file.id);
        let root_expression = tree.insert(Expression::Literal(Literal::Null), span);

        let file = DirParsedFile {
            file_id: file.id,
            aliases: Vec::new(),
            roots: Vec::new(),
            tokens: Vec::new(),
            comments: Vec::new(),
            anchor_expression: root_expression,
        };

        DirParsed::new(tree, vec![file], root_expression)
    }

    /// Parse one code module into DIR.
    fn parse_code_dir(
        &self,
        module: &Module,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<DirParsed, SessionError> {
        let mut tree = Tree::new(module_id);
        let mut files = Vec::with_capacity(module.files.len());

        // parse contributing source files into one module tree
        for module_file in &module.files {
            let file = self.source_file(attempt.revision(), module_file.file_id)?;
            let parsed_file = self.parse_code_file(file, module_file, &mut tree, attempt)?;

            files.push(parsed_file);
        }

        // preserve a stable module-level anchor
        let span = Span::empty(module.file_id);
        let anchor_expression = tree.insert(Expression::Literal(Literal::Boolean(false)), span);
        let dir = DirParsed::new(tree, files, anchor_expression);

        Ok(dir)
    }

    /// Parse one physical code file into a shared module tree.
    fn parse_code_file(
        &self,
        file: Arc<File>,
        module_file: &ModuleFile,
        tree: &mut Tree,
        attempt: &ProviderAttempt,
    ) -> Result<DirParsedFile, SessionError> {
        let repository = self.repository();

        // figure out language
        let language_type =
            LanguageType::try_from(file.ty).map_err(|_| SessionError::Internal {
                detail: format!("file type has no parser language: {:?}", file.ty),
            })?;

        // parse the source into the shared module tree
        let tree_in = std::mem::replace(tree, Tree::new(tree.module_id));
        let parser = Parser::new(
            file.clone(),
            language_type,
            tree_in,
            ParseOptions {
                comment_retention: CommentRetention::All,
                ..ParseOptions::default()
            },
        );
        let parse = parser.parse();

        // forward parser diagnostics
        let records = parse
            .diagnostics()
            .diagnostics
            .iter()
            .map(DiagnosticRecord::from)
            .collect();
        attempt.emit_diagnostics(records);

        // intern parsed strings in the repository
        repository.string_pool().extend(&parse.strings);

        // preserve semantic tokens in the artifact payload
        let tokens = parse.tokens;
        let comments = parse.comments;
        let roots = parse.roots;

        // restore the shared tree
        *tree = parse.tree;
        let span = Span::empty(file.id);
        let anchor_expression = tree.insert(Expression::Literal(Literal::Boolean(false)), span);

        Ok(DirParsedFile {
            file_id: file.id,
            aliases: module_file.aliases.clone(),
            roots,
            tokens,
            comments,
            anchor_expression,
        })
    }
}
