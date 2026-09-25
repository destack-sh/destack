use std::path::PathBuf;

use tspp_query::{CodeActionKind, CompletionTrigger, FileRename, QueryMethod};

use super::{FixturePosition, FixtureRange};

/// One query call parsed from a fixture.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum QueryCall {
    /// Request completion items at one position.
    Completion {
        /// The queried source position.
        position: FixturePosition,
        /// The completion trigger.
        trigger: CompletionTrigger,
        /// Whether auto-import completions are included.
        include_auto_imports: bool,
    },
    /// Request expanded fields for one completion entry.
    CompletionDetails {
        /// The queried source position.
        position: FixturePosition,
        /// The selected completion label.
        entry: String,
        /// The completion trigger.
        trigger: CompletionTrigger,
        /// Whether auto-import completions are included.
        include_auto_imports: bool,
    },
    /// Request hover information at one position.
    Hover {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request signature help at one position.
    SignatureHelp {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request inlay hints for one range.
    InlayHints {
        /// The queried source range.
        range: FixtureRange,
        /// Whether inferred type hints are requested.
        type_hints: bool,
        /// Whether parameter name hints are requested.
        parameter_hints: bool,
    },
    /// Request code lenses for one module.
    CodeLenses {
        /// The queried source module.
        module: PathBuf,
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
        range: FixtureRange,
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
        position: FixturePosition,
    },
    /// Request selection range chains for source positions.
    SelectionRanges {
        /// The queried source positions.
        positions: Vec<FixturePosition>,
    },
    /// Request definitions at one position.
    GotoDefinition {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request declarations at one position.
    GotoDeclaration {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request type definitions at one position.
    GotoTypeDefinition {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request implementations at one position.
    GotoImplementation {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request references at one position.
    FindReferences {
        /// The queried source position.
        position: FixturePosition,
        /// Whether declarations are included.
        include_declaration: bool,
    },
    /// Request one call hierarchy item.
    CallItem {
        /// The queried callable position.
        position: FixturePosition,
    },
    /// Request incoming calls for the item at one position.
    IncomingCalls {
        /// The queried callable position.
        item: FixturePosition,
    },
    /// Request outgoing calls for the item at one position.
    OutgoingCalls {
        /// The queried callable position.
        item: FixturePosition,
    },
    /// Request one type hierarchy item.
    TypeItem {
        /// The queried nominal type position.
        position: FixturePosition,
    },
    /// Request supertypes for the item at one position.
    Supertypes {
        /// The queried nominal type position.
        item: FixturePosition,
    },
    /// Request subtypes for the item at one position.
    Subtypes {
        /// The queried nominal type position.
        item: FixturePosition,
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
        position: FixturePosition,
    },
    /// Rename the symbol at one position.
    Rename {
        /// The queried source position.
        position: FixturePosition,
        /// The new symbol name.
        new_name: String,
    },
    /// Rewrite module specifiers after file renames.
    RenameFiles {
        /// The renamed file paths.
        renames: Vec<FileRename>,
    },
    /// Extract one source range into a variable.
    ExtractVariable {
        /// The selected expression range.
        range: FixtureRange,
        /// The new variable name.
        new_name: String,
    },
    /// Inline the symbol at one position.
    Inline {
        /// The queried source position.
        position: FixturePosition,
    },
    /// Request code actions for one source range.
    CodeActions {
        /// The queried source range.
        range: FixtureRange,
        /// The requested code action kinds.
        only: Vec<CodeActionKind>,
        /// Exact client diagnostics, or every matching diagnostic when absent.
        diagnostics: Option<Vec<FixtureDiagnostic>>,
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

/// One exact diagnostic supplied by a query fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FixtureDiagnostic {
    /// The canonical diagnostic id.
    pub(super) id: String,
    /// The primary diagnostic range.
    pub(super) range: FixtureRange,
    /// The optional primary label message.
    pub(super) message: Option<String>,
}

impl QueryCall {
    /// Parse one query fence header and body.
    pub(super) fn parse(language: &str, body: &str) -> Result<Self, String> {
        let source = language
            .strip_prefix("query ")
            .ok_or_else(|| format!("query fence '{language}' must start with 'query '"))?;
        let mut parser = QueryCallParser::new(source)?;
        let method_name = parser.required("query method")?;
        let method = QueryMethod::from_name(&method_name)
            .ok_or_else(|| format!("unknown query method '{method_name}'"))?;

        // parse the exact request shape selected by the method
        let call = match method {
            QueryMethod::Completion => Self::Completion {
                position: parser.position()?,
                trigger: parser.completion_trigger()?,
                include_auto_imports: parser.boolean("include_auto_imports", false)?,
            },
            QueryMethod::CompletionDetails => Self::CompletionDetails {
                position: parser.position()?,
                entry: parser.required_value("entry")?,
                trigger: parser.completion_trigger()?,
                include_auto_imports: parser.boolean("include_auto_imports", false)?,
            },
            QueryMethod::Hover => Self::Hover {
                position: parser.position()?,
            },
            QueryMethod::SignatureHelp => Self::SignatureHelp {
                position: parser.position()?,
            },
            QueryMethod::InlayHints => Self::InlayHints {
                range: parser.range()?,
                type_hints: parser.boolean("type_hints", true)?,
                parameter_hints: parser.boolean("parameter_hints", true)?,
            },
            QueryMethod::CodeLenses => Self::CodeLenses {
                module: parser.module_path()?,
            },
            QueryMethod::FoldingRanges => Self::FoldingRanges {
                module: parser.module_path()?,
            },
            QueryMethod::SemanticTokens => Self::SemanticTokens {
                module: parser.module_path()?,
            },
            QueryMethod::SemanticTokensRange => Self::SemanticTokensRange {
                range: parser.range()?,
            },
            QueryMethod::Outline => Self::Outline {
                module: parser.module_path()?,
            },
            QueryMethod::SearchSymbols => Self::SearchSymbols {
                query: parser.required_value("query")?,
                max_results: parser.unsigned_integer("max_results")?,
            },
            QueryMethod::Links => Self::Links {
                module: parser.module_path()?,
            },
            QueryMethod::Highlight => Self::Highlight {
                position: parser.position()?,
            },
            QueryMethod::SelectionRanges => Self::SelectionRanges {
                positions: parser.remaining_positions()?,
            },
            QueryMethod::GotoDefinition => Self::GotoDefinition {
                position: parser.position()?,
            },
            QueryMethod::GotoDeclaration => Self::GotoDeclaration {
                position: parser.position()?,
            },
            QueryMethod::GotoTypeDefinition => Self::GotoTypeDefinition {
                position: parser.position()?,
            },
            QueryMethod::GotoImplementation => Self::GotoImplementation {
                position: parser.position()?,
            },
            QueryMethod::FindReferences => Self::FindReferences {
                position: parser.position()?,
                include_declaration: parser.boolean("include_declaration", false)?,
            },
            QueryMethod::CallItem => Self::CallItem {
                position: parser.position()?,
            },
            QueryMethod::IncomingCalls => Self::IncomingCalls {
                item: parser.position()?,
            },
            QueryMethod::OutgoingCalls => Self::OutgoingCalls {
                item: parser.position()?,
            },
            QueryMethod::TypeItem => Self::TypeItem {
                position: parser.position()?,
            },
            QueryMethod::Supertypes => Self::Supertypes {
                item: parser.position()?,
            },
            QueryMethod::Subtypes => Self::Subtypes {
                item: parser.position()?,
            },
            QueryMethod::Decorators => Self::Decorators {
                scope: parser.decorator_scope()?,
                name: parser.optional_value("name")?,
            },
            QueryMethod::RenameTarget => Self::RenameTarget {
                position: parser.position()?,
            },
            QueryMethod::Rename => Self::Rename {
                position: parser.position()?,
                new_name: parser.required_value("new_name")?,
            },
            QueryMethod::RenameFiles => Self::RenameFiles {
                renames: parse_file_renames(body)?,
            },
            QueryMethod::ExtractVariable => Self::ExtractVariable {
                range: parser.range()?,
                new_name: parser.required_value("new_name")?,
            },
            QueryMethod::Inline => Self::Inline {
                position: parser.position()?,
            },
            QueryMethod::CodeActions => Self::CodeActions {
                range: parser.range()?,
                only: parser.code_action_kinds()?,
                diagnostics: parse_code_action_diagnostics(body)?,
            },
        };
        parser.finish()?;

        Ok(call)
    }

    /// Return this call's query method.
    pub(super) fn method(&self) -> QueryMethod {
        match self {
            Self::Completion { .. } => QueryMethod::Completion,
            Self::CompletionDetails { .. } => QueryMethod::CompletionDetails,
            Self::Hover { .. } => QueryMethod::Hover,
            Self::SignatureHelp { .. } => QueryMethod::SignatureHelp,
            Self::InlayHints { .. } => QueryMethod::InlayHints,
            Self::CodeLenses { .. } => QueryMethod::CodeLenses,
            Self::FoldingRanges { .. } => QueryMethod::FoldingRanges,
            Self::SemanticTokens { .. } => QueryMethod::SemanticTokens,
            Self::SemanticTokensRange { .. } => QueryMethod::SemanticTokensRange,
            Self::Outline { .. } => QueryMethod::Outline,
            Self::SearchSymbols { .. } => QueryMethod::SearchSymbols,
            Self::Links { .. } => QueryMethod::Links,
            Self::Highlight { .. } => QueryMethod::Highlight,
            Self::SelectionRanges { .. } => QueryMethod::SelectionRanges,
            Self::GotoDefinition { .. } => QueryMethod::GotoDefinition,
            Self::GotoDeclaration { .. } => QueryMethod::GotoDeclaration,
            Self::GotoTypeDefinition { .. } => QueryMethod::GotoTypeDefinition,
            Self::GotoImplementation { .. } => QueryMethod::GotoImplementation,
            Self::FindReferences { .. } => QueryMethod::FindReferences,
            Self::CallItem { .. } => QueryMethod::CallItem,
            Self::IncomingCalls { .. } => QueryMethod::IncomingCalls,
            Self::OutgoingCalls { .. } => QueryMethod::OutgoingCalls,
            Self::TypeItem { .. } => QueryMethod::TypeItem,
            Self::Supertypes { .. } => QueryMethod::Supertypes,
            Self::Subtypes { .. } => QueryMethod::Subtypes,
            Self::Decorators { .. } => QueryMethod::Decorators,
            Self::RenameTarget { .. } => QueryMethod::RenameTarget,
            Self::Rename { .. } => QueryMethod::Rename,
            Self::RenameFiles { .. } => QueryMethod::RenameFiles,
            Self::ExtractVariable { .. } => QueryMethod::ExtractVariable,
            Self::Inline { .. } => QueryMethod::Inline,
            Self::CodeActions { .. } => QueryMethod::CodeActions,
        }
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
    fn position(&mut self) -> Result<FixturePosition, String> {
        FixturePosition::parse(&self.required("position")?)
    }

    /// Parse one required range.
    fn range(&mut self) -> Result<FixtureRange, String> {
        FixtureRange::parse(&self.required("range")?)
    }

    /// Parse one required path.
    fn module_path(&mut self) -> Result<PathBuf, String> {
        Ok(PathBuf::from(self.required("module path")?))
    }

    /// Parse all remaining positional positions.
    fn remaining_positions(&mut self) -> Result<Vec<FixturePosition>, String> {
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
    fn boolean(&mut self, name: &str, default: bool) -> Result<bool, String> {
        let Some(value) = self.optional_value(name)? else {
            return Ok(default);
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

    /// Parse requested code action kinds.
    fn code_action_kinds(&mut self) -> Result<Vec<CodeActionKind>, String> {
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

        Ok(only)
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
        "refactor_extract" => Ok(CodeActionKind::RefactorExtract),
        "refactor_inline" => Ok(CodeActionKind::RefactorInline),
        _ => Err(format!("unknown code action kind '{value}'")),
    }
}

/// Parse exact diagnostics supplied with one code action request.
fn parse_code_action_diagnostics(source: &str) -> Result<Option<Vec<FixtureDiagnostic>>, String> {
    let lines = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(None);
    }
    if lines.as_slice() == ["diagnostics none"] {
        return Ok(Some(Vec::new()));
    }
    let mut diagnostics = Vec::new();

    // parse every explicit client diagnostic
    for line in lines {
        let words = parse_words(line)?;
        if words.first().map(String::as_str) != Some("diagnostic") {
            return Err(format!(
                "code action request row '{line}' must start with 'diagnostic'"
            ));
        }
        let Some(id) = words.get(1) else {
            return Err(format!("code action diagnostic '{line}' has no id"));
        };
        let Some(range) = words.get(2) else {
            return Err(format!("code action diagnostic '{line}' has no range"));
        };
        let message = match words.get(3) {
            Some(value) => Some(
                value
                    .strip_prefix("message=")
                    .ok_or_else(|| {
                        format!("code action diagnostic option '{value}' must be 'message=<text>'")
                    })?
                    .to_string(),
            ),
            None => None,
        };
        if words.len() > 4 {
            return Err(format!(
                "code action diagnostic '{line}' has unexpected arguments"
            ));
        }
        diagnostics.push(FixtureDiagnostic {
            id: id.clone(),
            range: FixtureRange::parse(range)?,
            message,
        });
    }

    Ok(Some(diagnostics))
}

/// Parse exact file rename lines.
fn parse_file_renames(source: &str) -> Result<Vec<FileRename>, String> {
    let mut renames = Vec::new();

    // parse every non-empty old-to-new path pair
    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let Some((old_path, new_path)) = line.split_once(" -> ") else {
            return Err(format!(
                "file rename '{line}' must be '<old path> -> <new path>'"
            ));
        };
        renames.push(FileRename {
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
