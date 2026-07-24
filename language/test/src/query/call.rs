use std::path::PathBuf;

use destack_query::{
    CodeActionContext, CodeActionKind, CodeLensAction, CompletionTrigger, FileRenameEntry,
    QueryMethodId, query_method,
};

use super::{QueryPosition, QueryRange};

/// One query call parsed from a fixture.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum QueryCall {
    /// Request completion items at one position.
    Completion {
        /// The queried source position.
        position: QueryPosition,
        /// The completion trigger.
        trigger: CompletionTrigger,
        /// Whether auto-import completions are included.
        include_imports: bool,
    },
    /// Request hover information at one position.
    Hover {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request signature help at one position.
    SignatureHelp {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request inlay hints for one range.
    InlayHints {
        /// The queried source range.
        range: QueryRange,
    },
    /// Request code lenses for one module.
    CodeLenses {
        /// The queried source module.
        module: PathBuf,
    },
    /// Resolve the code lens at one source range.
    ResolveCodeLens {
        /// The exact code lens range.
        lens: QueryRange,
        /// The code lens action to resolve.
        action: CodeLensKind,
    },
    /// Request folding ranges for one module.
    FoldingRanges {
        /// The queried source module.
        module: PathBuf,
    },
    /// Request semantic tokens for one module.
    SemanticTokens {
        /// The queried source module.
        module: PathBuf,
    },
    /// Request semantic tokens for one range.
    SemanticTokensRange {
        /// The queried source range.
        range: QueryRange,
    },
    /// Request the symbol outline for one module.
    Outline {
        /// The queried source module.
        module: PathBuf,
    },
    /// Search symbols across the program.
    SearchSymbols {
        /// The symbol search text.
        query: String,
        /// The maximum number of returned symbols.
        max_results: u32,
    },
    /// Request links for one module.
    Links {
        /// The queried source module.
        module: PathBuf,
    },
    /// Request document highlights at one position.
    Highlight {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request selection range chains for source positions.
    SelectionRanges {
        /// The queried source positions.
        positions: Vec<QueryPosition>,
    },
    /// Request definitions at one position.
    GotoDefinition {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request declarations at one position.
    GotoDeclaration {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request type definitions at one position.
    GotoTypeDefinition {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request implementations at one position.
    GotoImplementation {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request references at one position.
    FindReferences {
        /// The queried source position.
        position: QueryPosition,
        /// Whether declarations are included.
        include_declaration: bool,
    },
    /// Request one call hierarchy item.
    CallItem {
        /// The queried callable position.
        position: QueryPosition,
    },
    /// Request incoming calls for the item at one position.
    IncomingCalls {
        /// The queried callable position.
        item: QueryPosition,
    },
    /// Request outgoing calls for the item at one position.
    OutgoingCalls {
        /// The queried callable position.
        item: QueryPosition,
    },
    /// Request one type hierarchy item.
    TypeItem {
        /// The queried nominal type position.
        position: QueryPosition,
    },
    /// Request supertypes for the item at one position.
    Supertypes {
        /// The queried nominal type position.
        item: QueryPosition,
    },
    /// Request subtypes for the item at one position.
    Subtypes {
        /// The queried nominal type position.
        item: QueryPosition,
    },
    /// Request decorators from one scope.
    Decorators {
        /// The decorator query scope.
        scope: QueryDecoratorScope,
        /// The optional decorator name filter.
        name: Option<String>,
    },
    /// Request the rename target at one position.
    RenameTarget {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Rename the symbol at one position.
    Rename {
        /// The queried source position.
        position: QueryPosition,
        /// The new symbol name.
        new_name: String,
    },
    /// Rewrite module specifiers after file renames.
    RenameFiles {
        /// The renamed file paths.
        renames: Vec<FileRenameEntry>,
    },
    /// Extract one source range into a variable.
    ExtractVariable {
        /// The selected expression range.
        range: QueryRange,
        /// The new variable name.
        new_name: String,
    },
    /// Inline the symbol at one position.
    Inline {
        /// The queried source position.
        position: QueryPosition,
    },
    /// Request code actions for one source range.
    CodeActions {
        /// The queried source range.
        range: QueryRange,
        /// The requested code action context.
        context: CodeActionContext,
    },
}

/// One decorator query scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum QueryDecoratorScope {
    /// One source module.
    Module(PathBuf),
    /// Every program profile.
    Program,
}

/// One code lens action selected by a query fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodeLensKind {
    /// A reference count lens.
    References,
    /// An implementation count lens.
    Implementations,
    /// A test run lens.
    RunTest,
    /// A test debug lens.
    DebugTest,
    /// A custom command lens.
    Custom,
}

impl CodeLensKind {
    /// Return whether a code lens has this action.
    pub(super) const fn matches(self, action: &CodeLensAction) -> bool {
        matches!(
            (self, action),
            (Self::References, CodeLensAction::References { .. })
                | (
                    Self::Implementations,
                    CodeLensAction::Implementations { .. }
                )
                | (Self::RunTest, CodeLensAction::RunTest { .. })
                | (Self::DebugTest, CodeLensAction::DebugTest { .. })
                | (Self::Custom, CodeLensAction::Custom { .. })
        )
    }
}

impl QueryCall {
    /// Parse one query fence header and body.
    pub(super) fn parse(
        language: &str,
        body: &str,
        expected_method: QueryMethodId,
    ) -> Result<Self, String> {
        let source = language
            .strip_prefix("query ")
            .ok_or_else(|| format!("query fence '{language}' must start with 'query '"))?;
        let mut parser = QueryCallParser::new(source)?;
        let method_name = parser.required("query method")?;
        let method = query_method(&method_name)
            .ok_or_else(|| format!("unknown query method '{method_name}'"))?
            .id;
        if method != expected_method {
            return Err(format!(
                "query fixture for {expected_method:?} contains a {method:?} call"
            ));
        }

        // parse the exact request shape selected by the method
        let call = match method {
            QueryMethodId::Completion => Self::Completion {
                position: parser.position()?,
                trigger: parser.completion_trigger()?,
                include_imports: parser.boolean("include_imports")?,
            },
            QueryMethodId::Hover => Self::Hover {
                position: parser.position()?,
            },
            QueryMethodId::SignatureHelp => Self::SignatureHelp {
                position: parser.position()?,
            },
            QueryMethodId::InlayHints => Self::InlayHints {
                range: parser.range()?,
            },
            QueryMethodId::CodeLenses => Self::CodeLenses {
                module: parser.module_path()?,
            },
            QueryMethodId::ResolveCodeLens => Self::ResolveCodeLens {
                lens: parser.range()?,
                action: parser.code_lens_action()?,
            },
            QueryMethodId::FoldingRanges => Self::FoldingRanges {
                module: parser.module_path()?,
            },
            QueryMethodId::SemanticTokens => Self::SemanticTokens {
                module: parser.module_path()?,
            },
            QueryMethodId::SemanticTokensRange => Self::SemanticTokensRange {
                range: parser.range()?,
            },
            QueryMethodId::Outline => Self::Outline {
                module: parser.module_path()?,
            },
            QueryMethodId::SearchSymbols => Self::SearchSymbols {
                query: parser.required_value("query")?,
                max_results: parser.unsigned_integer("max_results")?,
            },
            QueryMethodId::Links => Self::Links {
                module: parser.module_path()?,
            },
            QueryMethodId::Highlight => Self::Highlight {
                position: parser.position()?,
            },
            QueryMethodId::SelectionRanges => Self::SelectionRanges {
                positions: parser.remaining_positions()?,
            },
            QueryMethodId::GotoDefinition => Self::GotoDefinition {
                position: parser.position()?,
            },
            QueryMethodId::GotoDeclaration => Self::GotoDeclaration {
                position: parser.position()?,
            },
            QueryMethodId::GotoTypeDefinition => Self::GotoTypeDefinition {
                position: parser.position()?,
            },
            QueryMethodId::GotoImplementation => Self::GotoImplementation {
                position: parser.position()?,
            },
            QueryMethodId::FindReferences => Self::FindReferences {
                position: parser.position()?,
                include_declaration: parser.boolean("include_declaration")?,
            },
            QueryMethodId::CallItem => Self::CallItem {
                position: parser.position()?,
            },
            QueryMethodId::IncomingCalls => Self::IncomingCalls {
                item: parser.position()?,
            },
            QueryMethodId::OutgoingCalls => Self::OutgoingCalls {
                item: parser.position()?,
            },
            QueryMethodId::TypeItem => Self::TypeItem {
                position: parser.position()?,
            },
            QueryMethodId::Supertypes => Self::Supertypes {
                item: parser.position()?,
            },
            QueryMethodId::Subtypes => Self::Subtypes {
                item: parser.position()?,
            },
            QueryMethodId::Decorators => Self::Decorators {
                scope: parser.decorator_scope()?,
                name: parser.optional_value("name")?,
            },
            QueryMethodId::RenameTarget => Self::RenameTarget {
                position: parser.position()?,
            },
            QueryMethodId::Rename => Self::Rename {
                position: parser.position()?,
                new_name: parser.required_value("new_name")?,
            },
            QueryMethodId::RenameFiles => Self::RenameFiles {
                renames: parse_file_renames(body)?,
            },
            QueryMethodId::ExtractVariable => Self::ExtractVariable {
                range: parser.range()?,
                new_name: parser.required_value("new_name")?,
            },
            QueryMethodId::Inline => Self::Inline {
                position: parser.position()?,
            },
            QueryMethodId::CodeActions => Self::CodeActions {
                range: parser.range()?,
                context: parser.code_action_context()?,
            },
        };
        parser.finish()?;

        Ok(call)
    }

    /// Return whether this call directly produces an edit.
    pub(super) fn is_edit(&self) -> bool {
        matches!(
            self,
            Self::Rename { .. }
                | Self::RenameFiles { .. }
                | Self::ExtractVariable { .. }
                | Self::Inline { .. }
        )
    }
}

/// One cursor over a query fence header.
struct QueryCallParser {
    /// The parsed header words.
    words: Vec<String>,
    /// The next unread word.
    index: usize,
}

impl QueryCallParser {
    /// Parse one query fence header.
    fn new(source: &str) -> Result<Self, String> {
        Ok(Self {
            words: parse_words(source)?,
            index: 0,
        })
    }

    /// Return the next required positional word.
    fn required(&mut self, noun: &str) -> Result<String, String> {
        let word = self
            .words
            .get(self.index)
            .ok_or_else(|| format!("query call has no {noun}"))?
            .clone();
        if word.contains('=') {
            return Err(format!("query call expected {noun}, found option '{word}'"));
        }
        self.index += 1;

        Ok(word)
    }

    /// Parse one required position.
    fn position(&mut self) -> Result<QueryPosition, String> {
        QueryPosition::parse(&self.required("position")?)
    }

    /// Parse one required range.
    fn range(&mut self) -> Result<QueryRange, String> {
        QueryRange::parse(&self.required("range")?)
    }

    /// Parse one required path.
    fn module_path(&mut self) -> Result<PathBuf, String> {
        Ok(PathBuf::from(self.required("module path")?))
    }

    /// Parse all remaining positional positions.
    fn remaining_positions(&mut self) -> Result<Vec<QueryPosition>, String> {
        let mut positions = Vec::new();

        // consume every remaining positional position
        while self.index < self.words.len() && !self.words[self.index].contains('=') {
            positions.push(self.position()?);
        }
        if positions.is_empty() {
            return Err("selection_ranges query has no positions".to_string());
        }

        Ok(positions)
    }

    /// Remove one named option.
    fn optional_value(&mut self, name: &str) -> Result<Option<String>, String> {
        let prefix = format!("{name}=");
        let mut found = None;
        let mut index = self.index;

        // remove the option while preserving the order of every other option
        while index < self.words.len() {
            if let Some(value) = self.words[index].strip_prefix(&prefix) {
                if found.is_some() {
                    return Err(format!("query call repeats option '{name}'"));
                }
                found = Some(value.to_string());
                self.words.remove(index);
            } else {
                index += 1;
            }
        }

        Ok(found)
    }

    /// Remove one required named option.
    fn required_value(&mut self, name: &str) -> Result<String, String> {
        self.optional_value(name)?
            .ok_or_else(|| format!("query call has no '{name}' option"))
    }

    /// Parse one required unsigned integer option.
    fn unsigned_integer(&mut self, name: &str) -> Result<u32, String> {
        let value = self.required_value(name)?;

        value
            .parse()
            .map_err(|_| format!("query option '{name}' is not a u32: '{value}'"))
    }

    /// Parse one optional boolean option.
    fn boolean(&mut self, name: &str) -> Result<bool, String> {
        let Some(value) = self.optional_value(name)? else {
            return Ok(false);
        };

        match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!(
                "query option '{name}' must be 'true' or 'false', found '{value}'"
            )),
        }
    }

    /// Parse one completion trigger.
    fn completion_trigger(&mut self) -> Result<CompletionTrigger, String> {
        let Some(value) = self.optional_value("trigger")? else {
            return Ok(CompletionTrigger::Invoked);
        };

        match value.as_str() {
            "invoked" => Ok(CompletionTrigger::Invoked),
            "incomplete" => Ok(CompletionTrigger::Incomplete),
            value => {
                let mut characters = value.chars();
                let character = characters
                    .next()
                    .ok_or_else(|| "completion trigger has no character".to_string())?;
                if characters.next().is_some() {
                    return Err(format!(
                        "completion trigger must be one character, found '{value}'"
                    ));
                }

                Ok(CompletionTrigger::Character(character))
            }
        }
    }

    /// Parse one decorator scope.
    fn decorator_scope(&mut self) -> Result<QueryDecoratorScope, String> {
        let value = self.required_value("scope")?;
        if value == "program" {
            Ok(QueryDecoratorScope::Program)
        } else {
            Ok(QueryDecoratorScope::Module(PathBuf::from(value)))
        }
    }

    /// Parse one required code lens action.
    fn code_lens_action(&mut self) -> Result<CodeLensKind, String> {
        let value = self.required_value("action")?;

        match value.as_str() {
            "references" => Ok(CodeLensKind::References),
            "implementations" => Ok(CodeLensKind::Implementations),
            "run_test" => Ok(CodeLensKind::RunTest),
            "debug_test" => Ok(CodeLensKind::DebugTest),
            "custom" => Ok(CodeLensKind::Custom),
            _ => Err(format!("unknown code lens action '{value}'")),
        }
    }

    /// Parse one code action context.
    fn code_action_context(&mut self) -> Result<CodeActionContext, String> {
        let only = self.optional_value("only")?;
        let only = only
            .map(|value| {
                value
                    .split(',')
                    .map(parse_code_action_kind)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let include_disabled = self.boolean("include_disabled")?;

        Ok(CodeActionContext {
            only,
            include_disabled,
        })
    }

    /// Require every header word to be consumed.
    fn finish(self) -> Result<(), String> {
        if let Some(word) = self.words.get(self.index) {
            return Err(format!("query call has unknown argument '{word}'"));
        }

        Ok(())
    }
}

/// Parse one code action kind.
fn parse_code_action_kind(value: &str) -> Result<CodeActionKind, String> {
    match value {
        "quick_fix" => Ok(CodeActionKind::QuickFix),
        "refactor" => Ok(CodeActionKind::Refactor),
        "refactor_extract" => Ok(CodeActionKind::RefactorExtract),
        "refactor_inline" => Ok(CodeActionKind::RefactorInline),
        "refactor_rewrite" => Ok(CodeActionKind::RefactorRewrite),
        "source" => Ok(CodeActionKind::Source),
        "source_fix_all" => Ok(CodeActionKind::SourceFixAll),
        _ => Err(format!("unknown code action kind '{value}'")),
    }
}

/// Parse exact file rename lines.
fn parse_file_renames(source: &str) -> Result<Vec<FileRenameEntry>, String> {
    let mut renames = Vec::new();

    // parse every non-empty old-to-new path pair
    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let Some((old_path, new_path)) = line.split_once(" -> ") else {
            return Err(format!(
                "file rename '{line}' must be '<old path> -> <new path>'"
            ));
        };
        renames.push(FileRenameEntry {
            old_path: PathBuf::from(old_path),
            new_path: PathBuf::from(new_path),
        });
    }
    if renames.is_empty() {
        return Err("rename_files query has no file renames".to_string());
    }

    Ok(renames)
}

/// Split one query header or row into quoted words.
pub(super) fn parse_words(source: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut characters = source.chars();
    let mut is_quoted = false;

    // collect words while retaining whitespace inside quotes
    while let Some(character) = characters.next() {
        match character {
            '"' => is_quoted = !is_quoted,
            '\\' if is_quoted => {
                let escaped = characters
                    .next()
                    .ok_or_else(|| "query text ends with an escape".to_string())?;
                let escaped = match escaped {
                    '0' => '\0',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    'u' => parse_unicode_escape(&mut characters)?,
                    _ => return Err(format!("query text has unknown escape '\\{escaped}'")),
                };
                word.push(escaped);
            }
            character if character.is_whitespace() && !is_quoted => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            character => word.push(character),
        }
    }
    if is_quoted {
        return Err("query text has an unterminated quote".to_string());
    }
    if !word.is_empty() {
        words.push(word);
    }

    Ok(words)
}

/// Parse one Rust-style Unicode scalar escape.
fn parse_unicode_escape(characters: &mut impl Iterator<Item = char>) -> Result<char, String> {
    if characters.next() != Some('{') {
        return Err("query Unicode escape must start with '\\u{'".to_string());
    }
    let mut digits = String::new();

    // collect hexadecimal digits through the closing brace
    loop {
        let character = characters
            .next()
            .ok_or_else(|| "query Unicode escape has no closing brace".to_string())?;
        if character == '}' {
            break;
        }
        if !character.is_ascii_hexdigit() || digits.len() == 6 {
            return Err(
                "query Unicode escape must contain one to six hexadecimal digits".to_string(),
            );
        }
        digits.push(character);
    }
    if digits.is_empty() {
        return Err("query Unicode escape has no digits".to_string());
    }
    let value = u32::from_str_radix(&digits, 16)
        .map_err(|_| format!("query Unicode escape '\\u{{{digits}}}' is invalid"))?;

    char::from_u32(value)
        .ok_or_else(|| format!("query Unicode escape '\\u{{{digits}}}' is not a scalar"))
}
