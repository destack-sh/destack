use crate::{ComponentValueList, print_component_value_list};

use super::component::parse_component_value_list;
use destack_source::{File, Span};
use lightningcss::rules::CssRule as LightningCssRule;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

/// Return the block prelude portion of one serialized rule.
pub(crate) fn rule_block_prelude(source: &str) -> ComponentValueList {
    source
        .split_once('{')
        .map(|(prelude, _)| parse_component_value_list(prelude.trim_end()))
        .unwrap_or_else(|| parse_component_value_list(source.trim_end_matches(';').trim_end()))
}

/// Serialize one rule with default printer options.
pub(crate) fn serialize_rule(rule: &LightningCssRule<'static>) -> String {
    serialize_value(rule)
}

/// Serialize one CSS value with default printer options.
pub(crate) fn serialize_value<T: ToCss>(value: &T) -> String {
    value
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default()
}

/// Return the source span for one CSS rule.
pub(crate) fn span_for_rule(source: &str, file: &File, rule: &LightningCssRule<'static>) -> Span {
    let location = match rule {
        LightningCssRule::Media(rule) => Some(rule.loc),
        LightningCssRule::Import(rule) => Some(rule.loc),
        LightningCssRule::Style(rule) => Some(rule.loc),
        LightningCssRule::Keyframes(rule) => Some(rule.loc),
        LightningCssRule::FontFace(rule) => Some(rule.loc),
        LightningCssRule::FontPaletteValues(rule) => Some(rule.loc),
        LightningCssRule::FontFeatureValues(rule) => Some(rule.loc),
        LightningCssRule::Page(rule) => Some(rule.loc),
        LightningCssRule::Supports(rule) => Some(rule.loc),
        LightningCssRule::CounterStyle(rule) => Some(rule.loc),
        LightningCssRule::Namespace(rule) => Some(rule.loc),
        LightningCssRule::MozDocument(rule) => Some(rule.loc),
        LightningCssRule::Nesting(rule) => Some(rule.loc),
        LightningCssRule::NestedDeclarations(rule) => Some(rule.loc),
        LightningCssRule::Viewport(rule) => Some(rule.loc),
        LightningCssRule::CustomMedia(rule) => Some(rule.loc),
        LightningCssRule::LayerStatement(rule) => Some(rule.loc),
        LightningCssRule::LayerBlock(rule) => Some(rule.loc),
        LightningCssRule::Property(rule) => Some(rule.loc),
        LightningCssRule::Container(rule) => Some(rule.loc),
        LightningCssRule::Scope(rule) => Some(rule.loc),
        LightningCssRule::StartingStyle(rule) => Some(rule.loc),
        LightningCssRule::ViewTransition(rule) => Some(rule.loc),
        LightningCssRule::Unknown(rule) => Some(rule.loc),
        LightningCssRule::Custom(_) | LightningCssRule::Ignored => None,
    };

    let serialized_rule = serialize_rule(rule);
    let serialized_prelude = print_component_value_list(&rule_block_prelude(&serialized_rule));
    let start = location
        .map(|location| {
            location_to_offset(source, location.line as usize, location.column as usize)
        })
        .and_then(|start| {
            find_rule_start(
                source,
                start as usize,
                &serialized_rule,
                &serialized_prelude,
            )
            .map(|start| start as u32)
        })
        .or_else(|| {
            find_rule_start(source, 0, &serialized_rule, &serialized_prelude)
                .map(|start| start as u32)
        });
    let Some(start) = start else {
        return Span::empty(file.id);
    };
    let end = rule_end_offset(source, start as usize) as u32;

    Span::new(file.id, start, end)
}

/// Return the prelude span for one CSS rule when it has one.
pub(crate) fn prelude_span_for_rule(source: &str, rule_span: Span) -> Option<Span> {
    if rule_span.is_empty() {
        return None;
    }

    let bytes = source.as_bytes();
    let start = rule_span.start as usize;
    let end = rule_span.end as usize;
    let mut index = start;
    let mut state = ScanState::default();

    while index < end {
        let next = skip_css_scan(bytes, index, &mut state);

        if next != index {
            index = next;
            continue;
        }

        if state.brace_depth == 0 && state.parenthesis_depth == 0 && state.bracket_depth == 0 {
            match bytes[index] {
                b'{' | b';' => {
                    let prelude_end = trim_ascii_whitespace_end(bytes, start, index);

                    return Some(Span::new(
                        rule_span.file,
                        rule_span.start,
                        prelude_end as u32,
                    ));
                }
                _ => {}
            }
        }

        state.advance(bytes[index]);
        index += 1;
    }

    let prelude_end = trim_ascii_whitespace_end(bytes, start, end);

    Some(Span::new(
        rule_span.file,
        rule_span.start,
        prelude_end as u32,
    ))
}

/// Return the authored span for one import url within one rule.
pub(crate) fn import_url_span_for_rule(source: &str, rule_span: Span, url: &str) -> Option<Span> {
    if rule_span.is_empty() || url.is_empty() {
        return None;
    }

    let start = rule_span.start as usize;
    let end = rule_span.end as usize;
    let rule_source = source.get(start..end)?;
    let offset = rule_source.find(url)?;

    Some(Span::new(
        rule_span.file,
        (start + offset) as u32,
        (start + offset + url.len()) as u32,
    ))
}

/// Convert one line and column pair into one byte offset.
pub(crate) fn location_to_offset(source: &str, line: usize, column: usize) -> u32 {
    File::byte_offset_from_position(source, line, column)
}

/// Return the exclusive byte end offset for one rule that starts at `start`.
fn rule_end_offset(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start;
    let mut state = ScanState::default();

    while index < bytes.len() {
        let next = skip_css_scan(bytes, index, &mut state);

        if next != index {
            index = next;
            continue;
        }

        if state.brace_depth == 0 && state.parenthesis_depth == 0 && state.bracket_depth == 0 {
            if bytes[index] == b';' {
                return index + 1;
            }

            if bytes[index] == b'{' {
                state.brace_depth += 1;
                index += 1;

                break;
            }
        }

        state.advance(bytes[index]);
        index += 1;
    }

    while index < bytes.len() {
        let next = skip_css_scan(bytes, index, &mut state);

        if next != index {
            index = next;
            continue;
        }

        if bytes[index] == b'}' && state.brace_depth == 1 {
            return index + 1;
        }

        state.advance(bytes[index]);
        index += 1;
    }

    bytes.len()
}

/// Return the best authored byte offset for one rule start.
fn find_rule_start(
    source: &str,
    hint: usize,
    serialized_rule: &str,
    serialized_prelude: &str,
) -> Option<usize> {
    let hint = hint.min(source.len());

    if !serialized_rule.is_empty() && source.get(hint..)?.starts_with(serialized_rule) {
        return Some(hint);
    }

    if !serialized_prelude.is_empty()
        && let Some(offset) = source.get(hint..)?.find(serialized_prelude)
    {
        return Some(hint + offset);
    }

    if !serialized_rule.is_empty()
        && let Some(offset) = source.find(serialized_rule)
    {
        return Some(offset);
    }

    if !serialized_prelude.is_empty()
        && let Some(offset) = source.find(serialized_prelude)
    {
        return Some(offset);
    }

    if hint < source.len() {
        return Some(hint);
    }

    None
}

/// Trim trailing ASCII whitespace from one byte range end.
fn trim_ascii_whitespace_end(bytes: &[u8], start: usize, mut end: usize) -> usize {
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    end
}

/// One scanning state for CSS source ranges.
#[derive(Default)]
struct ScanState {
    parenthesis_depth: usize,
    bracket_depth: usize,
    brace_depth: usize,
    string_delimiter: Option<u8>,
}

impl ScanState {
    /// Advance this state by one ordinary byte.
    fn advance(&mut self, byte: u8) {
        match byte {
            b'(' => self.parenthesis_depth += 1,
            b')' => self.parenthesis_depth = self.parenthesis_depth.saturating_sub(1),
            b'[' => self.bracket_depth += 1,
            b']' => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            b'{' => self.brace_depth += 1,
            b'}' => self.brace_depth = self.brace_depth.saturating_sub(1),
            b'\'' | b'"' => self.string_delimiter = Some(byte),
            _ => {}
        }
    }
}

/// Skip comments and string bodies while scanning CSS source.
fn skip_css_scan(bytes: &[u8], index: usize, state: &mut ScanState) -> usize {
    if let Some(delimiter) = state.string_delimiter {
        let mut index = index;

        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index += 2;
                continue;
            }

            index += 1;

            if bytes[index - 1] == delimiter {
                state.string_delimiter = None;
                break;
            }
        }

        return index;
    }

    if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
        let mut index = index + 2;

        while index + 1 < bytes.len() {
            if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                return index + 2;
            }

            index += 1;
        }

        return bytes.len();
    }

    index
}
