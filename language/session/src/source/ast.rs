use std::sync::Arc;

use destack_artifact::{ArtifactPayload, Ast};
use destack_ast as ast;
use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, LanguageType, ModuleId, PackageId, Span};
use destack_workspace::ProviderContext;

use crate::{SessionError, SessionProviderContext, SessionState};

impl SessionState {
    /// Provide one AST artifact from source.
    pub(crate) fn provide_ast(
        &self,
        module_id: ModuleId,
        context: &SessionProviderContext,
    ) -> Result<ArtifactPayload, SessionError> {
        let revision = context.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleNotTracked { module_id })?;
        let file = self.file(revision, module.file_id, context)?;

        // parse real code modules only
        let ast = if module.loader.is_code() && file.ty.is_code() {
            self.parse_code_ast(file.clone(), module.package_id, context)?
        } else {
            self.empty_ast(file.as_ref())
        };

        Ok(ArtifactPayload::Ast(ast))
    }

    /// Build one empty AST for non-code source.
    fn empty_ast(&self, file: &File) -> Ast {
        let mut tree = ast::Tree::new();
        let span = Span::empty(file.id);
        let root_expression = tree.insert(
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::Null),
            span,
        );

        Ast::from_tree(
            tree,
            Vec::new(),
            StringPool::new(),
            Vec::new(),
            Vec::new(),
            root_expression,
        )
    }

    /// Parse one code module into AST.
    fn parse_code_ast(
        &self,
        file: Arc<File>,
        _package_id: PackageId,
        context: &SessionProviderContext,
    ) -> Result<Ast, SessionError> {
        // figure out language
        let language_type =
            LanguageType::try_from(file.ty).map_err(|_| SessionError::Internal {
                detail: format!("file type has no parser language: {:?}", file.ty),
            })?;
        // parse and forward parser diagnostics
        let mut parser =
            Parser::lex_file_with_options(file.clone(), language_type, ParserOptions::default());
        let expressions = parser.parse();
        context.emit_collection(parser.diagnostics.collect());

        // preserve parser side data in the artifact payload
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = StringPool::from_local(parser.strings);
        let span = Span::empty(file.id);
        let root_expression = parser.tree.insert(
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(false)),
            span,
        );
        let ast = Ast::from_tree(
            parser.tree,
            expressions,
            strings,
            tokens,
            side_tokens,
            root_expression,
        );

        Ok(ast)
    }
}
