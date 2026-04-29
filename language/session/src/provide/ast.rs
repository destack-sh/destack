use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactPayload, Ast, ProviderContext};
use destack_ast as ast;
use destack_core::StringPool;
use destack_parser::{Parser, ParserSettings};
use destack_source::{File, FileType, LanguageType, ModuleId, PackageId, Span};

use super::context::SessionContext;
use crate::{Session, SessionError};

impl Session {
    /// Provide one AST artifact from source.
    pub(super) fn provide_ast(
        &self,
        module_id: ModuleId,
        context: &SessionContext,
    ) -> Result<(), SessionError> {
        let revision = context.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleIdNotTracked { module_id })?;
        let file = self.file(revision, module.file_id, context)?;

        // parse real code modules only
        let ast = if module.loader.is_code() && file.ty.is_code() {
            self.parse_code_ast(file.clone(), module.package_id, context)?
        } else {
            self.anchor_ast(file.as_ref())
        };

        context
            .payload(ArtifactPayload::Ast(ast))
            .map_err(|error| SessionError::Internal {
                detail: error.to_string(),
            })?;

        Ok(())
    }

    /// Build one stable anchor AST for non-code source.
    fn anchor_ast(&self, file: &File) -> Ast {
        let mut tree = ast::Tree::new();
        let span = Span::empty(file.id);
        let anchor_expression = tree.insert(
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(false)),
            span,
        );

        Ast::from_tree(
            tree,
            Vec::new(),
            StringPool::new(),
            Vec::new(),
            Vec::new(),
            anchor_expression,
        )
    }

    /// Parse one code module into AST.
    fn parse_code_ast(
        &self,
        file: Arc<File>,
        package_id: PackageId,
        context: &SessionContext,
    ) -> Result<Ast, SessionError> {
        let language_type = self.source_language(file.ty, package_id, context)?;

        // configure parser from session compiler options
        let mut parser = Parser::lex_file_with_settings(
            file.clone(),
            language_type,
            ParserSettings {
                disallow_ambiguous_tree_literal: self
                    .compiler()
                    .options
                    .disallow_ambiguous_tree_literal,
                ..ParserSettings::default()
            },
        );

        // parse and forward parser diagnostics
        let expressions = parser.parse();
        context.diagnostics(parser.diagnostics.collect());

        // preserve parser side data in the artifact payload
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = StringPool::from_local(parser.strings);
        let span = Span::empty(file.id);
        let anchor_expression = parser.tree.insert(
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(false)),
            span,
        );
        let ast = Ast::from_tree(
            parser.tree,
            expressions,
            strings,
            tokens,
            side_tokens,
            anchor_expression,
        );

        Ok(ast)
    }

    /// Resolve the parser language for one source file type.
    fn source_language(
        &self,
        file_type: FileType,
        package_id: PackageId,
        context: &SessionContext,
    ) -> Result<LanguageType, SessionError> {
        let language_type =
            LanguageType::from_file_type(file_type).ok_or(SessionError::Internal {
                detail: format!("file type has no parser language: {file_type:?}"),
            })?;

        // only plain js can be promoted to jsx by package config
        if file_type != FileType::JavaScript {
            return Ok(language_type);
        }

        // record the package config dependency that controls js-as-jsx
        let Some(package) = self.repository().package(context.revision(), package_id)? else {
            return Ok(language_type);
        };
        if let Some(file_id) = package.destack_file_id {
            let content_id = self
                .repository()
                .file_content_id(context.revision(), file_id)?
                .ok_or(SessionError::FileIdNotTracked { file_id })?;

            context.dependency(ArtifactDependency::file_content(file_id, content_id));
        }

        let package_options = self
            .repository()
            .package_options(context.revision(), package_id)?;
        if package_options.is_some_and(|options| options.compiler.js_as_jsx) {
            return Ok(LanguageType::JavaScriptXml);
        }

        Ok(language_type)
    }
}
