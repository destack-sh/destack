use dyst_ast::{Definition, ModuleFormat, NodeId, NodeParentIndex, NodeTree, TokenSpan, TokenType};
use dyst_fir::format;
use dyst_format::{DystFormatContext, DystFormatOptions};
use dyst_parser::Parser;
use dyst_session::Session;
use dyst_source::{MultiSpan, Source, SourceFormat, SourceId, StringPool, Uri};

use crate::PackageId;

pub const PACKAGE_FILE_NAME: &str = "package.dst";
pub const MODULE_FILE_NAME: &str = "module.ds";

/// The special intent of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentIntent {
    /// Generic source file (`.ds`)
    Source,
    /// Generic data file (`.dst`, `.dsb`, `.dsx`)
    Data,
    /// Package file (`package.dst`)
    Package,
    /// Module file (`module.ds`)
    Module,
}

/// A source Document (pre-parsed).
#[derive(Debug, Clone)]
pub struct Document {
    /// The ID of the source.
    pub id: SourceId,
    /// The ID of the containing package.
    pub package_id: PackageId,
    /// The name of the document.
    pub name: String,
    /// The URI of the SourceFile.
    pub uri: Uri,
    /// The format of the document.
    pub format: SourceFormat,
    /// The special intent of the document.
    pub intent: DocumentIntent,
    /// Whether the document is currently open.
    pub is_open: bool,
    /// The content of the document.
    pub body: DocumentBody,
}

/// The content of a Document.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentBody {
    Text {
        /// The text source of the document.
        source: Source,
        /// The main tokens of the document.
        tokens: Vec<TokenSpan>,
        /// The side tokens of the document.
        side_tokens: Vec<TokenSpan>,
        /// The side span of the document.
        side_span: MultiSpan,
        /// All tokens of the document (including side tokens).
        all_tokens: Vec<TokenSpan>,
        /// The AST of the document.
        ast: NodeTree,
        /// The root definition ID of the document.
        /// (In case of irrecoverable errors, this is an empty module.)
        root_definition_id: NodeId<Definition>,
        /// The string pool.
        strings: StringPool,
    },
    Binary {
        /// The binary content of the document.
        content: Vec<u8>,
    },
}

impl DocumentBody {
    /// Build a document for the provided text source.
    pub(crate) fn parse_text(source: Source, session: &mut Session) -> Self {
        // module name
        let module_name = source.uri.last_segment().unwrap_or("<string>");

        // parse the document AST
        let mut parser = Parser::prepare(&source, session);
        let module_name_id = parser.intern_string(module_name);
        let root_definition_id = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(None, Some(module_name_id), ModuleFormat::Source)
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        // default to empty module if no root definition is found
        let root_definition_id = root_definition_id.unwrap_or_else(|| {
            parser.tree.allocate(
                Definition::Module {
                    name: Some(module_name_id),
                    format: ModuleFormat::Source,
                    visibility: None,
                    with_clauses: None,
                    where_clauses: None,
                    expressions: vec![],
                },
                source.whole_span(),
            )
        });
        parser.finalize();

        // combine tokens and AST into document body
        let side_span = parser.get_side_span();
        let side_tokens = parser.side_tokens;
        let tokens = parser.tokens;
        let mut all_tokens: Vec<TokenSpan> = Vec::with_capacity(tokens.len() + side_tokens.len());
        all_tokens.extend(tokens.iter());
        all_tokens.extend(side_tokens.iter());
        all_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;
        let strings = parser.strings;

        DocumentBody::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            all_tokens,
            ast,
            root_definition_id,
            strings,
        }
    }

    /// Build a document for the provided binary source.
    pub(crate) fn wrap_binary(content: Vec<u8>) -> Self {
        DocumentBody::Binary { content }
    }
}

impl Document {
    /// Build a parsed Document for the provided text source.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse_text(
        id: SourceId,
        package_id: PackageId,
        name: String,
        uri: Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
        session: &mut Session,
    ) -> Self {
        let source = Source::from_string(id, name.clone(), uri.clone(), format, content);
        let content = DocumentBody::parse_text(source, session);
        let intent = match format {
            SourceFormat::Dyst => {
                if name.eq(MODULE_FILE_NAME) {
                    DocumentIntent::Module
                } else {
                    DocumentIntent::Source
                }
            }
            SourceFormat::DystText => {
                if name.eq(PACKAGE_FILE_NAME) {
                    DocumentIntent::Package
                } else {
                    DocumentIntent::Data
                }
            }
            SourceFormat::DystBinary => DocumentIntent::Data,
            SourceFormat::DystExecutable => DocumentIntent::Data,
        };

        Document {
            id,
            package_id,
            name,
            uri,
            format,
            intent,
            is_open,
            body: content,
        }
    }

    /// Build a parsed Document for the provided binary content.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn wrap_binary(
        id: SourceId,
        package_id: PackageId,
        name: String,
        uri: Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> Self {
        Document {
            id,
            package_id,
            name,
            uri,
            format,
            intent: DocumentIntent::Data,
            is_open,
            body: DocumentBody::wrap_binary(content),
        }
    }

    /// Format a (text) Document.
    /// If the document couldn't be formatted, returns `None`.
    pub fn format(&self, session: &Session) -> Option<String> {
        let DocumentBody::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            ast,
            root_definition_id: module_id,
            strings,
            ..
        } = &self.body
        else {
            return None;
        };

        // format with default options
        // NOTE #Incomplete: configure LSP formatting options from Workspace
        let options = DystFormatOptions::default();
        let context = DystFormatContext {
            options,
            source,
            tokens,
            side_tokens,
            side_span,
            tree: ast,
            spans: &ast.spans,
            parents: NodeParentIndex::from_tree(ast),
            session,
            strings,
        };
        let formatted = format!(context, [module_id]).unwrap();
        let printed = formatted.print();
        Some(printed.unwrap().as_str().to_string())
    }
}

/// Infer a source format from a URI.
pub fn infer_source_format_from_uri(uri: &Uri) -> Option<SourceFormat> {
    infer_source_format_from_str(uri.as_ref())
}

/// Infer a source format from a string.
fn infer_source_format_from_str(value: &str) -> Option<SourceFormat> {
    let trimmed = value.split(['?', '#']).next().unwrap_or(value);
    let extension = trimmed.rsplit('.').next()?;
    if extension.contains('/') || extension.contains('\\') {
        return None;
    }
    SourceFormat::from_extension(&extension.to_ascii_lowercase())
}
