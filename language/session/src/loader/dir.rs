use std::sync::Arc;

use destack_artifact::{ArtifactPayload, DirParsed};
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, LanguageType, ModuleId, Span};
use destack_workspace::ProviderContext;

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
        let file = self.source_file(revision, module.file_id, attempt)?;

        // parse real code modules only
        let dir = if module.loader.is_code() && file.ty.is_code() {
            self.parse_code_dir(file.clone(), module_id, attempt)?
        } else {
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

        DirParsed::from_tree(tree, Vec::new(), Vec::new(), Vec::new(), root_expression)
    }

    /// Parse one code module into DIR.
    fn parse_code_dir(
        &self,
        file: Arc<File>,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<DirParsed, SessionError> {
        let repository = self.repository();

        // figure out language
        let language_type =
            LanguageType::try_from(file.ty).map_err(|_| SessionError::Internal {
                detail: format!("file type has no parser language: {:?}", file.ty),
            })?;

        // parse and forward parser diagnostics
        let mut parser = Parser::lex_module_with_options(
            module_id,
            file.clone(),
            language_type,
            ParserOptions::default(),
            repository.string_pool().clone(),
        );
        let expressions = parser.parse();
        attempt.emit_collection(parser.diagnostics.collect());

        // preserve parser side data in the artifact payload
        let (tokens, side_tokens) = parser.take_tokens();
        let span = Span::empty(file.id);
        let root_expression = parser.tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
            span,
        );
        let dir = DirParsed::from_tree(
            parser.tree,
            expressions,
            tokens,
            side_tokens,
            root_expression,
        );

        Ok(dir)
    }
}
