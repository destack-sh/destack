use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactPayload, DirParsed, DirParsedFile};
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_repository::{Module, ModuleFile, ProviderContext};
use destack_source::{File, LanguageType, ModuleId, Span};

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
        let mut tree = dir::Tree::new(module_id);
        let span = Span::empty(file.id);
        let root_expression = tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Null),
            span,
        );

        let file = DirParsedFile {
            file_id: file.id,
            aliases: Vec::new(),
            roots: Vec::new(),
            tokens: Vec::new(),
            side_tokens: Vec::new(),
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
        let mut tree = dir::Tree::new(module_id);
        let mut files = Vec::with_capacity(module.files.len());

        // parse contributing source files into one module tree
        for module_file in &module.files {
            let file = self.source_file(attempt.revision(), module_file.file_id)?;
            let parsed_file = self.parse_code_file(file, module_file, &mut tree, attempt)?;

            files.push(parsed_file);
        }

        // preserve a stable module-level anchor
        let span = Span::empty(module.file_id);
        let anchor_expression = tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
            span,
        );
        let dir = DirParsed::new(tree, files, anchor_expression);

        Ok(dir)
    }

    /// Parse one physical code file into a shared module tree.
    fn parse_code_file(
        &self,
        file: Arc<File>,
        module_file: &ModuleFile,
        tree: &mut dir::Tree,
        attempt: &ProviderAttempt,
    ) -> Result<DirParsedFile, SessionError> {
        let repository = self.repository();

        // figure out language
        let language_type =
            LanguageType::try_from(file.ty).map_err(|_| SessionError::Internal {
                detail: format!("file type has no parser language: {:?}", file.ty),
            })?;

        // parse and forward parser diagnostics
        let tree_in = std::mem::replace(tree, dir::Tree::new(tree.module_id));
        let mut parser = Parser::lex_module_tree_with_options(
            file.clone(),
            language_type,
            ParserOptions::default(),
            repository.string_pool().clone(),
            tree_in,
        );
        let roots = parser.parse();
        attempt.emit_diagnostics(parser.diagnostics());

        // preserve parser side data in the artifact payload
        let (tokens, side_tokens) = parser.take_tokens();

        // restore the shared tree
        *tree = parser.tree;
        let span = Span::empty(file.id);
        let anchor_expression = tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
            span,
        );

        Ok(DirParsedFile {
            file_id: file.id,
            aliases: module_file.aliases.clone(),
            roots,
            tokens,
            side_tokens,
            anchor_expression,
        })
    }
}
