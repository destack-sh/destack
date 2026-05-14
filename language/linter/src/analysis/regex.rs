use std::sync::Arc;

use regex_syntax::ast::ErrorKind as RegexAstErrorKind;
use regex_syntax::hir::{ErrorKind as HirErrorKind, Hir};
use regex_syntax::{Error as RegexError, Parser, ParserBuilder};

/// The kind of error produced while parsing a regex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintRegexErrorKind {
    /// The regex failed while parsing into the regex AST.
    Parse(RegexAstErrorKind),
    /// The regex failed while translating the regex AST into HIR.
    Translate(HirErrorKind),
    /// The regex failed with an unclassified error.
    Other,
}

/// Describes a regex parse error for lint rules.
#[derive(Debug, Clone)]
pub struct LintRegexError {
    /// The error kind reported by the regex parser.
    pub kind: LintRegexErrorKind,
    /// A formatted error message for diagnostics.
    pub message: String,
}

/// Describe the parsed regex data for lint rules.
#[derive(Debug, Clone)]
pub struct LintRegexParse {
    /// The parsed regex HIR when available.
    pub hir: Option<Arc<Hir>>,
    /// The parse error when the pattern is invalid.
    pub error: Option<LintRegexError>,
}

impl LintRegexParse {
    /// Parse a pattern and return cached regex data.
    pub fn parse(pattern: &str) -> Self {
        Self::parse_with_flags(pattern, None)
    }

    /// Parse a pattern with optional flags and return cached regex data.
    pub fn parse_with_flags(pattern: &str, flags: Option<&str>) -> Self {
        let parse_result = regex_parser(flags).parse(pattern);
        let (hir, error) = match parse_result {
            Ok(hir) => (Some(Arc::new(hir)), None),
            Err(err) => {
                let message = err.to_string();
                let kind = match err {
                    RegexError::Parse(parse_error) => {
                        LintRegexErrorKind::Parse(parse_error.kind().clone())
                    }
                    RegexError::Translate(translate_error) => {
                        LintRegexErrorKind::Translate(translate_error.kind().clone())
                    }
                    _ => LintRegexErrorKind::Other,
                };
                (None, Some(LintRegexError { kind, message }))
            }
        };

        Self { hir, error }
    }
}

/// Build one regex parser configured from optional flags.
fn regex_parser(flags: Option<&str>) -> Parser {
    // use defaults when no flags are known
    let Some(flags) = flags else {
        return Parser::new();
    };

    // align parser mode with unicode-related JS flags
    let mut parser_builder = ParserBuilder::new();
    let has_unicode = flags.contains('u') || flags.contains('v');
    parser_builder.unicode(has_unicode);

    parser_builder.build()
}

/// Find a control character in a raw pattern string.
pub(crate) fn find_control_character(s: &str) -> Option<char> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            match next {
                'c' => {
                    i += 3;
                    continue;
                }
                'x' => {
                    i += 4;
                    continue;
                }
                'u' => {
                    i += 6;
                    continue;
                }
                '0' => {
                    i += 2;
                    continue;
                }
                'n' | 'r' | 't' | 'f' | 'v' => {
                    i += 2;
                    continue;
                }
                _ => {
                    i += 2;
                    continue;
                }
            }
        }

        if c.is_ascii_control() && c != '\t' && c != '\n' && c != '\r' {
            return Some(c);
        }

        i += 1;
    }

    None
}

/// Find control characters in a regex pattern with optional flag semantics.
pub(crate) fn find_control_characters(pattern: &str, flags: Option<&str>) -> Vec<String> {
    let unicode_mode = flags.is_some_and(|flags| flags.contains('u') || flags.contains('v'));
    let bytes = pattern.as_bytes();
    let mut control_characters = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += collect_control_escape(&mut control_characters, bytes, index, unicode_mode);
            continue;
        }

        if let Some(character) = pattern[index..].chars().next() {
            if character.is_ascii_control() {
                push_control_character(&mut control_characters, character as u32);
            }
            index += character.len_utf8();
            continue;
        }

        break;
    }

    control_characters
}

/// Parse one regex escape sequence and collect control characters.
fn collect_control_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
    unicode_mode: bool,
) -> usize {
    if index + 1 >= bytes.len() {
        return 1;
    }

    match bytes[index + 1] {
        b'x' => collect_hex_escape(control_characters, bytes, index),
        b'u' => collect_unicode_escape(control_characters, bytes, index, unicode_mode),
        _ => 2,
    }
}

/// Parse `\xNN` escape and collect a control character when present.
fn collect_hex_escape(control_characters: &mut Vec<String>, bytes: &[u8], index: usize) -> usize {
    if index + 3 >= bytes.len() {
        return 2;
    }

    let high = hex_nibble(bytes[index + 2]);
    let low = hex_nibble(bytes[index + 3]);
    let (Some(high), Some(low)) = (high, low) else {
        return 2;
    };

    let value = (high << 4) | low;
    push_control_character(control_characters, value as u32);
    4
}

/// Parse `\uNNNN` or `\u{...}` escape and collect a control character when present.
fn collect_unicode_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
    unicode_mode: bool,
) -> usize {
    if index + 2 >= bytes.len() {
        return 2;
    }

    if bytes[index + 2] == b'{' {
        if !unicode_mode {
            return 2;
        }
        return collect_unicode_braced_escape(control_characters, bytes, index);
    }

    if index + 5 >= bytes.len() {
        return 2;
    }

    let mut value = 0u32;
    for offset in 0..4 {
        let Some(nibble) = hex_nibble(bytes[index + 2 + offset]) else {
            return 2;
        };
        value = (value << 4) | nibble as u32;
    }

    push_control_character(control_characters, value);
    6
}

/// Parse `\u{...}` escape and collect a control character when present.
fn collect_unicode_braced_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
) -> usize {
    let mut cursor = index + 3;
    let mut value = 0u32;
    let mut has_digit = false;

    while cursor < bytes.len() {
        if bytes[cursor] == b'}' {
            if has_digit {
                push_control_character(control_characters, value);
                return cursor - index + 1;
            }
            return 2;
        }

        let Some(nibble) = hex_nibble(bytes[cursor]) else {
            return 2;
        };
        has_digit = true;
        value = (value << 4) | nibble as u32;
        cursor += 1;
    }

    2
}

/// Convert one ASCII hex byte to its numeric nibble value.
fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Add one control character to the collected diagnostics list.
fn push_control_character(control_characters: &mut Vec<String>, value: u32) {
    if value <= 0x1f {
        control_characters.push(format!("\\x{value:02x}"));
    }
}

/// Find a misleading character class in a pattern string.
pub(crate) fn find_misleading_character_class(regex: &str) -> Option<&'static str> {
    let mut in_class = false;
    let mut chars = regex.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '[' if !in_class => {
                in_class = true;
            }
            ']' if in_class => {
                in_class = false;
            }
            _ if in_class => {
                if is_combining_mark(c) {
                    return Some("combining character");
                }

                if c >= '\u{1F1E6}' && c <= '\u{1F1FF}' {
                    return Some("regional indicator symbol");
                }

                if is_zero_width(c) {
                    return Some("zero-width character");
                }
            }
            _ => {}
        }
    }

    None
}

/// Return whether the character is a combining mark.
fn is_combining_mark(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' |
        '\u{1AB0}'..='\u{1AFF}' |
        '\u{1DC0}'..='\u{1DFF}' |
        '\u{20D0}'..='\u{20FF}' |
        '\u{FE20}'..='\u{FE2F}'
    )
}

/// Return whether the character is zero width.
fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}')
}

/// Find a backreference that cannot resolve to a prior group.
pub(crate) fn find_useless_backreference(
    regex: &str,
    flags: Option<&str>,
    error_kind: Option<&LintRegexErrorKind>,
) -> Option<String> {
    // keep parse errors that are not backreference related out of this rule path
    if let Some(kind) = error_kind {
        let is_named_backreference_escape = regex.contains("\\k<");
        let is_backreference_related_error = matches!(
            kind,
            LintRegexErrorKind::Parse(RegexAstErrorKind::UnsupportedBackreference)
                | LintRegexErrorKind::Parse(RegexAstErrorKind::UnsupportedLookAround)
        ) || matches!(
            kind,
            LintRegexErrorKind::Parse(RegexAstErrorKind::EscapeUnrecognized)
        ) && is_named_backreference_escape;
        if !is_backreference_related_error {
            return None;
        }

        // skip reports when a separate syntax error is masked by backreference parsing
        if has_masked_non_backreference_syntax_error(regex, flags) {
            return None;
        }
    };

    analyze_backreferences(regex, flags)
}

/// Return true when sanitizing backreferences still leaves a syntax error.
fn has_masked_non_backreference_syntax_error(regex: &str, flags: Option<&str>) -> bool {
    let sanitized_pattern = sanitize_backreferences_for_parse(regex);
    let parse = LintRegexParse::parse_with_flags(&sanitized_pattern, flags);
    let Some(error) = parse.error else {
        return false;
    };

    // only suppress known repetition-count syntax errors that must fail in JS too
    matches!(
        error.kind,
        LintRegexErrorKind::Parse(
            RegexAstErrorKind::RepetitionCountInvalid
                | RegexAstErrorKind::RepetitionCountDecimalEmpty
                | RegexAstErrorKind::RepetitionCountUnclosed
        )
    )
}

/// Replace backreference tokens with literals for conservative syntax probing.
fn sanitize_backreferences_for_parse(regex: &str) -> String {
    let indexed_characters: Vec<(usize, char)> = regex.char_indices().collect();
    let mut sanitized = String::with_capacity(regex.len());
    let mut index = 0usize;
    let mut in_character_class = false;

    while index < indexed_characters.len() {
        let (character_start, character) = indexed_characters[index];

        // preserve character class contents and class escape structure
        if in_character_class {
            if character == '\\' {
                let escape_end = character_end_offset(regex, &indexed_characters, index + 1);
                sanitized.push_str(&regex[character_start..escape_end]);
                index += 2;
                continue;
            }
            if character == ']' {
                in_character_class = false;
            }

            let character_end = character_end_offset(regex, &indexed_characters, index + 1);
            sanitized.push_str(&regex[character_start..character_end]);
            index += 1;
            continue;
        }

        // mark the start of one character class
        if character == '[' {
            in_character_class = true;
            let character_end = character_end_offset(regex, &indexed_characters, index + 1);
            sanitized.push_str(&regex[character_start..character_end]);
            index += 1;
            continue;
        }

        // replace numeric and named backreferences with one literal
        if character == '\\'
            && let Some((_, next_character)) = indexed_characters.get(index + 1).copied()
        {
            if matches!(next_character, '1'..='9') {
                sanitized.push('a');
                index += 2;
                while indexed_characters
                    .get(index)
                    .is_some_and(|(_, digit)| digit.is_ascii_digit())
                {
                    index += 1;
                }
                continue;
            }

            if next_character == 'k'
                && indexed_characters
                    .get(index + 2)
                    .is_some_and(|(_, ch)| *ch == '<')
            {
                let mut cursor = index + 3;
                while let Some((_, name_character)) = indexed_characters.get(cursor) {
                    cursor += 1;
                    if *name_character == '>' {
                        break;
                    }
                }

                sanitized.push('a');
                index = cursor;
                continue;
            }
        }

        // preserve all other tokens
        let character_end = character_end_offset(regex, &indexed_characters, index + 1);
        sanitized.push_str(&regex[character_start..character_end]);
        index += 1;
    }

    sanitized
}

/// Return one end byte offset for an indexed character position.
fn character_end_offset(regex: &str, indexed_characters: &[(usize, char)], index: usize) -> usize {
    indexed_characters
        .get(index)
        .map_or(regex.len(), |(start, _)| *start)
}

/// One lookaround kind in a parsed regex group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedLookaroundKind {
    /// One lookahead assertion.
    Lookahead,
    /// One lookbehind assertion.
    Lookbehind,
}

/// One regex parser node kind used for backreference analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedRegexNodeKind {
    /// The synthetic pattern root node.
    Root,
    /// One alternative branch node.
    Alternative,
    /// One group node with optional capture metadata.
    Group {
        capture_index: Option<usize>,
        capture_name: Option<String>,
        lookaround: Option<ParsedLookaroundKind>,
        is_negative_lookaround: bool,
    },
}

/// One parsed regex node entry for backreference analysis.
#[derive(Debug, Clone)]
struct ParsedRegexNode {
    /// The kind of this node.
    kind: ParsedRegexNodeKind,
    /// The parent node id in `ParsedRegexStructure.nodes`.
    parent: Option<usize>,
    /// The byte start offset in the pattern.
    start: usize,
    /// The byte end offset in the pattern.
    end: usize,
}

/// One backreference target.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedBackreferenceTarget {
    /// One positional numeric backreference.
    Index(usize),
    /// One named backreference.
    Name(String),
}

/// One backreference occurrence in a parsed regex.
#[derive(Debug, Clone)]
struct ParsedBackreference {
    /// The raw backreference text such as `\1` or `\k<name>`.
    raw: String,
    /// The byte start offset in the pattern.
    start: usize,
    /// The byte end offset in the pattern.
    end: usize,
    /// The parent alternative node id.
    parent_alternative_id: usize,
    /// The resolved target shape.
    target: ParsedBackreferenceTarget,
}

/// Parsed structure data needed for backreference diagnostics.
#[derive(Debug, Clone)]
struct ParsedRegexStructure {
    /// All parsed nodes.
    nodes: Vec<ParsedRegexNode>,
    /// All parsed backreference occurrences in source order.
    backreferences: Vec<ParsedBackreference>,
    /// Capture group node ids indexed by capture index minus one.
    capture_group_ids_by_index: Vec<usize>,
    /// Capture group node ids grouped by capture name.
    capture_group_ids_by_name: Vec<(String, Vec<usize>)>,
}

/// One problem kind for one backreference and one target group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackreferenceProblemKind {
    /// The backreference appears inside its target capture group.
    Nested,
    /// The backreference appears before its target capture group.
    Forward,
    /// The backreference appears after its target in one lookbehind context.
    Backward,
    /// The backreference and target live in sibling alternatives.
    Disjunctive,
    /// The target group is inside one negative lookaround.
    IntoNegativeLookaround,
}

/// One analyzed problem for one backreference and one target group.
#[derive(Debug, Clone)]
struct BackreferenceProblem {
    /// The selected problem kind.
    kind: BackreferenceProblemKind,
    /// The target capture group node id.
    group_id: usize,
}

/// One active parser group frame.
#[derive(Debug, Clone, Copy)]
struct ParserGroupFrame {
    /// The current group node id.
    group_id: usize,
    /// The current alternative node id for this group.
    alternative_id: usize,
}

/// Analyze backreference usage for invalid references.
fn analyze_backreferences(regex: &str, flags: Option<&str>) -> Option<String> {
    // parse one lightweight structure for group and alternative relationships
    let parsed = parse_regex_structure(regex)?;
    let has_unicode_mode =
        flags.is_some_and(|flag_text| flag_text.contains('u') || flag_text.contains('v'));

    // inspect backreferences in source order
    for backreference in &parsed.backreferences {
        let Some(problem) =
            first_problem_for_backreference(&parsed, backreference, has_unicode_mode)
        else {
            continue;
        };

        return Some(format_backreference_problem(
            regex,
            &parsed,
            backreference,
            &problem,
        ));
    }

    None
}

/// Return one first reportable problem for one backreference.
fn first_problem_for_backreference(
    parsed: &ParsedRegexStructure,
    backreference: &ParsedBackreference,
    has_unicode_mode: bool,
) -> Option<BackreferenceProblem> {
    // resolve all target capture groups for this backreference
    let target_group_ids =
        resolve_backreference_target_groups(parsed, backreference, has_unicode_mode);
    if target_group_ids.is_empty() {
        return None;
    }

    // collect one problem per target group, or stop when one target is valid
    let mut problems = Vec::new();
    for group_id in target_group_ids {
        let problem_kind = classify_backreference_problem(parsed, backreference, group_id)?;
        problems.push(BackreferenceProblem {
            kind: problem_kind,
            group_id,
        });
    }

    // report non disjunctive problems first, otherwise report disjunctive
    for problem in &problems {
        if problem.kind != BackreferenceProblemKind::Disjunctive {
            return Some(problem.clone());
        }
    }

    problems.into_iter().next()
}

/// Resolve target capture group ids for one backreference.
fn resolve_backreference_target_groups(
    parsed: &ParsedRegexStructure,
    backreference: &ParsedBackreference,
    has_unicode_mode: bool,
) -> Vec<usize> {
    match &backreference.target {
        ParsedBackreferenceTarget::Index(backreference_index) => {
            // keep octal escapes out of this lint unless unicode mode forces backreferences
            if !has_unicode_mode && *backreference_index > parsed.capture_group_ids_by_index.len() {
                return Vec::new();
            }

            parsed
                .capture_group_ids_by_index
                .get(backreference_index.saturating_sub(1))
                .copied()
                .into_iter()
                .collect()
        }
        ParsedBackreferenceTarget::Name(backreference_name) => parsed
            .capture_group_ids_by_name
            .iter()
            .find(|(name, _)| name == backreference_name)
            .map(|(_, ids)| ids.clone())
            .unwrap_or_default(),
    }
}

/// Classify one problem for one backreference and one target group.
fn classify_backreference_problem(
    parsed: &ParsedRegexStructure,
    backreference: &ParsedBackreference,
    group_id: usize,
) -> Option<BackreferenceProblemKind> {
    // collect node paths from self to root
    let backreference_path = node_path_to_root(parsed, backreference.parent_alternative_id);
    let group_path = node_path_to_root(parsed, group_id);

    // group ancestor references are nested
    if backreference_path.contains(&group_id) {
        return Some(BackreferenceProblemKind::Nested);
    }

    // resolve one lowest common ancestor between both paths
    let (group_lca_index, group_common_path) =
        path_lowest_common_ancestor_split(&group_path, &backreference_path)?;

    // groups in sibling alternatives are disjunctive
    let group_cut = &group_path[..group_lca_index];
    if group_cut.last().is_some_and(|node_id| {
        matches!(
            parsed.nodes[*node_id].kind,
            ParsedRegexNodeKind::Alternative
        )
    }) {
        return Some(BackreferenceProblemKind::Disjunctive);
    }

    // keep the lowest common lookaround semantics
    let lowest_common_lookaround = group_common_path
        .iter()
        .find_map(|node_id| parsed_group_lookaround_kind(parsed, *node_id));
    let is_matching_backward = lowest_common_lookaround == Some(ParsedLookaroundKind::Lookbehind);

    // forward references are invalid in forward matching contexts
    let group = &parsed.nodes[group_id];
    if !is_matching_backward && backreference.end <= group.start {
        return Some(BackreferenceProblemKind::Forward);
    }

    // backward references are invalid in the same lookbehind context
    if is_matching_backward && group.end <= backreference.start {
        return Some(BackreferenceProblemKind::Backward);
    }

    // groups inside negative lookarounds are unreachable for this reference
    if group_cut
        .iter()
        .copied()
        .any(|node_id| parsed_node_is_negative_lookaround(parsed, node_id))
    {
        return Some(BackreferenceProblemKind::IntoNegativeLookaround);
    }

    None
}

/// Format one diagnostic message for a classified backreference problem.
fn format_backreference_problem(
    regex: &str,
    parsed: &ParsedRegexStructure,
    backreference: &ParsedBackreference,
    problem: &BackreferenceProblem,
) -> String {
    let group = &parsed.nodes[problem.group_id];
    let group_text = regex.get(group.start..group.end).unwrap_or("<group>");

    match problem.kind {
        BackreferenceProblemKind::Nested => format!(
            "backreference {} references group {} from within that group",
            backreference.raw, group_text
        ),
        BackreferenceProblemKind::Forward => format!(
            "backreference {} references group {} which appears later in the pattern",
            backreference.raw, group_text
        ),
        BackreferenceProblemKind::Backward => format!(
            "backreference {} references group {} which appears earlier in the same lookbehind",
            backreference.raw, group_text
        ),
        BackreferenceProblemKind::Disjunctive => format!(
            "backreference {} references group {} in another alternative",
            backreference.raw, group_text
        ),
        BackreferenceProblemKind::IntoNegativeLookaround => format!(
            "backreference {} references group {} inside a negative lookaround",
            backreference.raw, group_text
        ),
    }
}

/// Return one path from a node to the root.
fn node_path_to_root(parsed: &ParsedRegexStructure, node_id: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = Some(node_id);

    // push current nodes until the root has no parent
    while let Some(id) = current {
        path.push(id);
        current = parsed.nodes[id].parent;
    }

    path
}

/// Split one path by the lowest common ancestor with another path.
fn path_lowest_common_ancestor_split<'path>(
    group_path: &'path [usize],
    reference_path: &[usize],
) -> Option<(usize, &'path [usize])> {
    let mut group_cursor = group_path.len();
    let mut reference_cursor = reference_path.len();

    // walk back from root while both paths match
    while group_cursor > 0
        && reference_cursor > 0
        && group_path[group_cursor - 1] == reference_path[reference_cursor - 1]
    {
        group_cursor -= 1;
        reference_cursor -= 1;
    }

    if group_cursor >= group_path.len() {
        return None;
    }

    Some((group_cursor, &group_path[group_cursor..]))
}

/// Return the lookaround kind for one group node.
fn parsed_group_lookaround_kind(
    parsed: &ParsedRegexStructure,
    node_id: usize,
) -> Option<ParsedLookaroundKind> {
    let node = &parsed.nodes[node_id];
    let ParsedRegexNodeKind::Group { lookaround, .. } = &node.kind else {
        return None;
    };

    *lookaround
}

/// Return true when one node is a negative lookaround group.
fn parsed_node_is_negative_lookaround(parsed: &ParsedRegexStructure, node_id: usize) -> bool {
    let node = &parsed.nodes[node_id];
    let ParsedRegexNodeKind::Group {
        is_negative_lookaround,
        ..
    } = &node.kind
    else {
        return false;
    };

    *is_negative_lookaround
}

/// Parse one regex pattern into groups, alternatives, and backreferences.
fn parse_regex_structure(regex: &str) -> Option<ParsedRegexStructure> {
    let mut nodes = Vec::new();
    nodes.push(ParsedRegexNode {
        kind: ParsedRegexNodeKind::Root,
        parent: None,
        start: 0,
        end: regex.len(),
    });

    let mut capture_group_ids_by_index = Vec::new();
    let mut capture_group_ids_by_name: Vec<(String, Vec<usize>)> = Vec::new();
    let mut backreferences = Vec::new();
    let mut group_frames = Vec::new();
    let mut capture_count = 0usize;

    // initialize one root alternative for top level parsing
    let root_alternative_id = parsed_add_alternative_node(&mut nodes, 0, 0);
    group_frames.push(ParserGroupFrame {
        group_id: 0,
        alternative_id: root_alternative_id,
    });

    // walk pattern characters with one character class state
    let indexed_characters: Vec<(usize, char)> = regex.char_indices().collect();
    let mut index = 0;
    let mut in_character_class = false;
    while index < indexed_characters.len() {
        let (character_start, character) = indexed_characters[index];

        // close one character class when `]` is reached
        if in_character_class {
            if character == '\\' {
                index += 2;
                continue;
            }
            if character == ']' {
                in_character_class = false;
            }
            index += 1;
            continue;
        }

        // parse escapes and backreferences
        if character == '\\' {
            let Some((next_start, next_character)) = indexed_characters.get(index + 1).copied()
            else {
                index += 1;
                continue;
            };

            if let Some(backreference) = parsed_numeric_backreference(
                regex,
                &indexed_characters,
                index,
                group_frames.last().copied()?.alternative_id,
            ) {
                backreferences.push(backreference.0);
                index = backreference.1;
                continue;
            }

            if next_character == 'k'
                && let Some(backreference) = parsed_named_backreference(
                    regex,
                    &indexed_characters,
                    index,
                    group_frames.last().copied()?.alternative_id,
                )
            {
                backreferences.push(backreference.0);
                index = backreference.1;
                continue;
            }

            if next_start > character_start {
                index += 2;
                continue;
            }

            index += 1;
            continue;
        }

        // enter one character class
        if character == '[' {
            in_character_class = true;
            index += 1;
            continue;
        }

        // split one alternative with `|`
        if character == '|' {
            let group_frame = group_frames.last_mut()?;
            nodes[group_frame.alternative_id].end = character_start;

            let alternative_start = character_start + character.len_utf8();
            group_frame.alternative_id =
                parsed_add_alternative_node(&mut nodes, group_frame.group_id, alternative_start);

            index += 1;
            continue;
        }

        // parse one opening group
        if character == '(' {
            let group_frame = group_frames.last().copied()?;

            let (group_kind, next_index) =
                parsed_group_kind(&indexed_characters, index, &mut capture_count)?;
            let group_id = parsed_add_group_node(
                &mut nodes,
                group_frame.alternative_id,
                character_start,
                group_kind.clone(),
            );
            let group_alternative_start = indexed_characters
                .get(next_index)
                .map(|(start, _)| *start)
                .unwrap_or(regex.len());
            let alternative_id =
                parsed_add_alternative_node(&mut nodes, group_id, group_alternative_start);
            group_frames.push(ParserGroupFrame {
                group_id,
                alternative_id,
            });

            // track capture groups by index and name
            if let ParsedRegexNodeKind::Group {
                capture_index: Some(capture_index),
                capture_name,
                ..
            } = group_kind
            {
                if capture_group_ids_by_index.len() < capture_index {
                    capture_group_ids_by_index.push(group_id);
                } else {
                    capture_group_ids_by_index[capture_index - 1] = group_id;
                }

                if let Some(name) = capture_name {
                    if let Some((_, ids)) = capture_group_ids_by_name
                        .iter_mut()
                        .find(|(existing_name, _)| existing_name == &name)
                    {
                        ids.push(group_id);
                    } else {
                        capture_group_ids_by_name.push((name, vec![group_id]));
                    }
                }
            }

            index = next_index;
            continue;
        }

        // parse one closing group
        if character == ')' {
            if group_frames.len() <= 1 {
                return None;
            }

            let group_frame = group_frames.pop()?;
            nodes[group_frame.alternative_id].end = character_start;
            nodes[group_frame.group_id].end = character_start + character.len_utf8();
            index += 1;
            continue;
        }

        index += 1;
    }

    // reject unclosed groups and close the current alternative
    if group_frames.len() != 1 || in_character_class {
        return None;
    }
    if let Some(root_frame) = group_frames.last() {
        nodes[root_frame.alternative_id].end = regex.len();
    }

    Some(ParsedRegexStructure {
        nodes,
        backreferences,
        capture_group_ids_by_index,
        capture_group_ids_by_name,
    })
}

/// Parse one numeric backreference at one escape position.
fn parsed_numeric_backreference(
    regex: &str,
    indexed_characters: &[(usize, char)],
    backslash_index: usize,
    parent_alternative_id: usize,
) -> Option<(ParsedBackreference, usize)> {
    let (_, next_character) = indexed_characters.get(backslash_index + 1).copied()?;
    if !next_character.is_ascii_digit() || next_character == '0' {
        return None;
    }

    // consume one decimal backreference index
    let mut cursor = backslash_index + 1;
    let mut digits = String::new();
    while let Some((_, digit_character)) = indexed_characters.get(cursor) {
        if !digit_character.is_ascii_digit() {
            break;
        }
        digits.push(*digit_character);
        cursor += 1;
    }
    let backreference_index = digits.parse::<usize>().ok()?;

    // resolve one raw text slice
    let backslash_start = indexed_characters[backslash_index].0;
    let backreference_end = indexed_characters
        .get(cursor)
        .map(|(start, _)| *start)
        .unwrap_or(regex.len());
    let raw = regex.get(backslash_start..backreference_end)?.to_string();

    Some((
        ParsedBackreference {
            raw,
            start: backslash_start,
            end: backreference_end,
            parent_alternative_id,
            target: ParsedBackreferenceTarget::Index(backreference_index),
        },
        cursor,
    ))
}

/// Parse one named backreference at one escape position.
fn parsed_named_backreference(
    regex: &str,
    indexed_characters: &[(usize, char)],
    backslash_index: usize,
    parent_alternative_id: usize,
) -> Option<(ParsedBackreference, usize)> {
    if indexed_characters.get(backslash_index + 1).map(|(_, c)| *c) != Some('k')
        || indexed_characters.get(backslash_index + 2).map(|(_, c)| *c) != Some('<')
    {
        return None;
    }

    // consume one named backreference payload
    let mut cursor = backslash_index + 3;
    let mut name = String::new();
    while let Some((_, character)) = indexed_characters.get(cursor).copied() {
        if character == '>' {
            let backslash_start = indexed_characters[backslash_index].0;
            let backreference_end = indexed_characters
                .get(cursor + 1)
                .map(|(start, _)| *start)
                .unwrap_or(regex.len());
            let raw = regex.get(backslash_start..backreference_end)?.to_string();

            return Some((
                ParsedBackreference {
                    raw,
                    start: backslash_start,
                    end: backreference_end,
                    parent_alternative_id,
                    target: ParsedBackreferenceTarget::Name(name),
                },
                cursor + 1,
            ));
        }

        name.push(character);
        cursor += 1;
    }

    None
}

/// Parse one opening group header and return its group kind and next content index.
fn parsed_group_kind(
    indexed_characters: &[(usize, char)],
    open_group_index: usize,
    capture_count: &mut usize,
) -> Option<(ParsedRegexNodeKind, usize)> {
    let next_character = indexed_characters
        .get(open_group_index + 1)
        .map(|(_, c)| *c);
    if next_character != Some('?') {
        *capture_count += 1;
        let capture_index = *capture_count;
        return Some((
            ParsedRegexNodeKind::Group {
                capture_index: Some(capture_index),
                capture_name: None,
                lookaround: None,
                is_negative_lookaround: false,
            },
            open_group_index + 1,
        ));
    }

    match indexed_characters
        .get(open_group_index + 2)
        .map(|(_, c)| *c)
    {
        Some(':') => Some((
            ParsedRegexNodeKind::Group {
                capture_index: None,
                capture_name: None,
                lookaround: None,
                is_negative_lookaround: false,
            },
            open_group_index + 3,
        )),
        Some('=') => Some((
            ParsedRegexNodeKind::Group {
                capture_index: None,
                capture_name: None,
                lookaround: Some(ParsedLookaroundKind::Lookahead),
                is_negative_lookaround: false,
            },
            open_group_index + 3,
        )),
        Some('!') => Some((
            ParsedRegexNodeKind::Group {
                capture_index: None,
                capture_name: None,
                lookaround: Some(ParsedLookaroundKind::Lookahead),
                is_negative_lookaround: true,
            },
            open_group_index + 3,
        )),
        Some('<') => match indexed_characters
            .get(open_group_index + 3)
            .map(|(_, c)| *c)
        {
            Some('=') => Some((
                ParsedRegexNodeKind::Group {
                    capture_index: None,
                    capture_name: None,
                    lookaround: Some(ParsedLookaroundKind::Lookbehind),
                    is_negative_lookaround: false,
                },
                open_group_index + 4,
            )),
            Some('!') => Some((
                ParsedRegexNodeKind::Group {
                    capture_index: None,
                    capture_name: None,
                    lookaround: Some(ParsedLookaroundKind::Lookbehind),
                    is_negative_lookaround: true,
                },
                open_group_index + 4,
            )),
            Some(_) => {
                // parse one named capture group
                let mut cursor = open_group_index + 3;
                let mut name = String::new();
                while let Some((_, character)) = indexed_characters.get(cursor).copied() {
                    if character == '>' {
                        *capture_count += 1;
                        let capture_index = *capture_count;
                        return Some((
                            ParsedRegexNodeKind::Group {
                                capture_index: Some(capture_index),
                                capture_name: Some(name),
                                lookaround: None,
                                is_negative_lookaround: false,
                            },
                            cursor + 1,
                        ));
                    }

                    name.push(character);
                    cursor += 1;
                }

                None
            }
            None => None,
        },
        Some(_) => Some((
            ParsedRegexNodeKind::Group {
                capture_index: None,
                capture_name: None,
                lookaround: None,
                is_negative_lookaround: false,
            },
            open_group_index + 2,
        )),
        None => None,
    }
}

/// Add one alternative node and return its node id.
fn parsed_add_alternative_node(
    nodes: &mut Vec<ParsedRegexNode>,
    parent_id: usize,
    start: usize,
) -> usize {
    let node_id = nodes.len();
    nodes.push(ParsedRegexNode {
        kind: ParsedRegexNodeKind::Alternative,
        parent: Some(parent_id),
        start,
        end: start,
    });

    node_id
}

/// Add one group node and return its node id.
fn parsed_add_group_node(
    nodes: &mut Vec<ParsedRegexNode>,
    parent_id: usize,
    start: usize,
    kind: ParsedRegexNodeKind,
) -> usize {
    let node_id = nodes.len();
    nodes.push(ParsedRegexNode {
        kind,
        parent: Some(parent_id),
        start,
        end: start,
    });

    node_id
}
