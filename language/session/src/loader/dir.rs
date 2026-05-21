use std::sync::Arc;

use destack_artifact::{ArtifactPayload, DirParsed, DirParsedFile};
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, LanguageType, ModuleId, Span};
use destack_workspace::{Module, ModuleFile, ProviderContext};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
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
            let file = self.source_file(revision, module.file_id, attempt)?;

            Self::make_empty_dir(file.as_ref(), module_id)
        };

        Ok(ArtifactPayload::DirParsed(dir))
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
            token_range: 0..0,
            side_token_range: 0..0,
            anchor_expression: root_expression,
        };

        DirParsed::new(tree, vec![file], Vec::new(), Vec::new(), root_expression)
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
        let mut tokens = Vec::new();
        let mut side_tokens = Vec::new();

        // parse contributing source files into one module tree
        for module_file in &module.files {
            let file = self.source_file(attempt.revision(), module_file.file_id, attempt)?;
            let parsed_file = self.parse_code_file(
                file,
                module_file,
                &mut tree,
                &mut tokens,
                &mut side_tokens,
                attempt,
            )?;

            files.push(parsed_file);
        }

        // preserve a stable module-level anchor
        let span = Span::empty(module.file_id);
        let anchor_expression = tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
            span,
        );
        let dir = DirParsed::new(tree, files, tokens, side_tokens, anchor_expression);

        Ok(dir)
    }

    /// Parse one physical code file into a shared module tree.
    fn parse_code_file(
        &self,
        file: Arc<File>,
        module_file: &ModuleFile,
        tree: &mut dir::Tree,
        tokens: &mut Vec<dir::TokenSpan>,
        side_tokens: &mut Vec<dir::TokenSpan>,
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
        attempt.emit_collection(parser.diagnostics());

        // preserve parser side data in the artifact payload
        let token_start = tokens.len() as u32;
        let side_token_start = side_tokens.len() as u32;
        let (mut file_tokens, mut file_side_tokens) = parser.take_tokens();
        tokens.append(&mut file_tokens);
        side_tokens.append(&mut file_side_tokens);
        let token_end = tokens.len() as u32;
        let side_token_end = side_tokens.len() as u32;

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
            token_range: token_start..token_end,
            side_token_range: side_token_start..side_token_end,
            anchor_expression,
        })
    }
}
