use dyst_ast::{
    Definition, DefinitionMeta, ModuleFormat, ModuleStyle, Name, NodeId, NodeParentIndex, NodeTree,
    TokenSpan, TokenType,
};
use dyst_fir::format;
use dyst_formatter::{DystFormatContext, DystFormatOptions};
use dyst_parser::Parser;
use dyst_session::Session;
use dyst_source::{LanguageOptions, MultiSpan, Source, SourceFormat, SourceId, StringPool, Uri};

use crate::PackageId;

/// Generic source file (`ds`)
pub const SOURCE_FILE_NAME: &str = "ds";
/// Generic source file extension (`.ds`)
pub const SOURCE_FILE_EXTENSION: &str = ".ds";

/// Generic source declaration file (`d.ds`)
pub const SOURCE_DECLARATION_FILE_NAME: &str = "d.ds";
/// Generic source declaration file extension (`.d.ds`)
pub const SOURCE_DECLARATION_FILE_EXTENSION: &str = ".d.ds";

/// Generic data file (`dst`)
pub const DATA_FILE_NAME: &str = "dst";
/// Generic data file extension (`.dst`)
pub const DATA_FILE_EXTENSION: &str = ".dst";

/// Generic binary file (`dsb`)
pub const BINARY_FILE_NAME: &str = "dsb";
/// Generic binary file extension (`.dsb`)
pub const BINARY_FILE_EXTENSION: &str = ".dsb";

/// Package file (`.dst`)
pub const PACKAGE_FILE_NAME: &str = "package.dst";
/// Module file (`.ds`)
pub const MODULE_FILE_NAME: &str = "module.ds";

/// The special intent of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIntent {
    /// Generic source file (`.ds`)
    Source,
    /// Generic source declaration file (`.d.ds`)
    SourceDeclaration,
    /// Generic data file (`.dst`, `.dsb`, `.dsx`)
    Data,
    /// Package file (`package.dst`)
    Package,
    /// Module file (`module.ds`)
    Module,
}

/// A source File (might be on disk, might also be virtual or in-memory).
#[derive(Debug, Clone)]
pub struct File {
    /// The ID of this source.
    pub id: SourceId,
    /// The ID of the containing package.
    pub package_id: PackageId,
    /// The name of the file.
    pub name: String,
    /// The URI of the SourceFile.
    pub uri: Uri,
    /// The format of the file.
    pub format: SourceFormat,
    /// The special intent of the file.
    pub intent: FileIntent,
    /// Whether the file is currently open (in editor context).
    pub is_open: bool,
    /// The content of the file.
    pub content: FileContent,
}

/// The content of a File.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum FileContent {
    /// The content of a (text) SourceFile (e.g., `.ds`, `.d.ds`, or `.dst`)
    Source(SourceFile),
    /// The content of a BinaryFile (e.g., `.dsb` or `.dsx`)
    Binary(BinaryFile),
}

/// The content of a (text) SourceFile.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The text source of the file.
    pub source: Source,
    /// The main tokens of the file.
    pub tokens: Vec<TokenSpan>,
    /// The side tokens of the file.
    pub side_tokens: Vec<TokenSpan>,
    /// The side span of the file.
    pub side_span: MultiSpan,
    /// All tokens of the file (including side tokens).
    pub all_tokens: Vec<TokenSpan>,
    /// The AST of the file.
    pub ast: NodeTree,
    /// The parent index of the AST.
    pub parents: NodeParentIndex,
    /// The root definition ID of the file.
    /// (In case of irrecoverable errors, this is an empty module.)
    pub root_definition_id: NodeId<Definition>,
    /// The string pool.
    pub strings: StringPool,
}

impl SourceFile {
    pub fn parse(source: Source, language: LanguageOptions, session: &mut Session) -> Self {
        // module name
        let module_name = source.uri.last_segment().unwrap_or("<string>");

        // parse the file AST
        let mut parser = Parser::prepare(&source, language, session);
        let module_name_id = parser.intern_string(module_name);
        let root_definition_id = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(
                        DefinitionMeta::new(Name::Identifier(module_name_id)),
                        ModuleFormat::Source,
                        ModuleStyle::Module,
                    )
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        // default to empty module if no root definition is found
        let root_definition_id = root_definition_id.unwrap_or_else(|| {
            parser.tree.insert(
                Definition::Module {
                    meta: DefinitionMeta::new(Name::Identifier(module_name_id)),
                    format: ModuleFormat::Source,
                    style: ModuleStyle::Module,
                    with_clauses: None,
                    where_clauses: None,
                    expressions: Vec::new(),
                },
                source.whole_span(),
            )
        });
        parser.finalize();

        // combine tokens and AST into file body
        let side_span = parser.get_side_span();
        let side_tokens = parser.side_tokens;
        let tokens = parser.tokens;
        let mut all_tokens: Vec<TokenSpan> = Vec::with_capacity(tokens.len() + side_tokens.len());
        all_tokens.extend(tokens.iter());
        all_tokens.extend(side_tokens.iter());
        all_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;
        let parents = NodeParentIndex::from_tree(&ast);
        let strings = parser.strings;

        SourceFile {
            source,
            tokens,
            side_tokens,
            side_span,
            all_tokens,
            ast,
            parents,
            root_definition_id,
            strings,
        }
    }
}

/// The content of a BinaryFile.
#[derive(Debug, Clone)]
pub struct BinaryFile {
    /// The binary content of the file.
    pub content: Vec<u8>,
}

impl BinaryFile {
    pub(crate) fn from(content: Vec<u8>) -> Self {
        BinaryFile { content }
    }
}

impl FileContent {
    /// Build a file for the provided text source.
    pub(crate) fn parse_text(
        source: Source,
        language: LanguageOptions,
        session: &mut Session,
    ) -> Self {
        FileContent::Source(SourceFile::parse(source, language, session))
    }

    /// Build a file for the provided binary source.
    pub(crate) fn wrap_binary(content: Vec<u8>) -> Self {
        FileContent::Binary(BinaryFile::from(content))
    }
}

impl File {
    /// Build a parsed File for the provided text source.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse_text(
        id: SourceId,
        package_id: PackageId,
        name: String,
        uri: Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
        language: LanguageOptions,
        session: &mut Session,
    ) -> Self {
        let source = Source::from_string(id, name.clone(), uri.clone(), format, content);
        let content = FileContent::parse_text(source, language, session);
        let intent = match format {
            SourceFormat::Dyst => {
                if name.eq(MODULE_FILE_NAME) {
                    FileIntent::Module
                } else if name.ends_with(SOURCE_DECLARATION_FILE_EXTENSION) {
                    FileIntent::SourceDeclaration
                } else {
                    FileIntent::Source
                }
            }
            SourceFormat::DystDeclaration => FileIntent::SourceDeclaration,
            SourceFormat::DystText => {
                if name.eq(PACKAGE_FILE_NAME) {
                    FileIntent::Package
                } else {
                    FileIntent::Data
                }
            }
            SourceFormat::DystBinary => FileIntent::Data,
            SourceFormat::DystExecutable => FileIntent::Data,
        };

        File {
            id,
            package_id,
            name,
            uri,
            format,
            intent,
            is_open,
            content,
        }
    }

    /// Build a parsed File for the provided binary content.
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
        File {
            id,
            package_id,
            name,
            uri,
            format,
            intent: FileIntent::Data,
            is_open,
            content: FileContent::wrap_binary(content),
        }
    }

    /// Format a (text) File.
    /// If the file couldn't be formatted, returns `None`.
    pub fn format(&self, session: &Session) -> Option<String> {
        let FileContent::Source(SourceFile {
            source,
            tokens,
            side_tokens,
            side_span,
            ast,
            root_definition_id: module_id,
            strings,
            ..
        }) = &self.content
        else {
            return None;
        };

        // format with default options
        // NOTE #Incomplete: configure LSP formatting options from Workspace/Package
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
