use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Key, Keyword, Name, is_identifier};
use destack_base::StringId;
use destack_fir::format::text;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_workspace::{QuoteProperty, QuoteStyle};

impl<'ast> Format<DestackFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        format_name_with_quote_policy(f, *self, false)
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        format_key_with_quote_policy(f, *self, false)
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}

/// Format a key with quote policy controls.
pub(crate) fn format_key_with_quote_policy<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    key: Key,
    force_quote_keys: bool,
) -> FormatResult<()> {
    match key {
        Key::Name(name) => {
            format_name_with_quote_policy(f, name, force_quote_keys)?;
        }
        Key::Private(name) => {
            write!(f, [token("#")])?;
            write!(f, [name])?;
        }
        Key::Expression(expression) => {
            write!(f, [token("[")])?;
            write!(f, [expression])?;
            write!(f, [token("]")])?;
        }
        Key::NamedExpression { name, key } => {
            write!(f, [token("[")])?;
            write!(f, [name])?;
            write!(f, [token(":"), space()])?;
            write!(f, [key])?;
            write!(f, [token("]")])?;
        }
    }

    Ok(())
}

/// Check whether a string is an identifier safe to leave unquoted in JavaScript.
pub(crate) fn is_identifier_for_quotes(content: &str) -> bool {
    if contains_katakana_middle_dot(content) {
        return false;
    }
    is_identifier(content)
}

/// Check whether a string contains katakana middle dot characters.
fn contains_katakana_middle_dot(content: &str) -> bool {
    content
        .chars()
        .any(|c| matches!(c, '\u{30FB}' | '\u{FF65}'))
}

/// Format a name key while applying quote policy.
fn format_name_with_quote_policy<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Name,
    force_quote_keys: bool,
) -> FormatResult<()> {
    let context = f.context();
    let is_destack = context.options.language_type.is_destack();
    let mut quote_props = context.options.quote_props;
    if is_destack {
        quote_props = QuoteProperty::Preserve;
    }

    match name {
        Name::Identifier(string_id) => {
            let content = context.strings.get(string_id);
            if content.starts_with('#')
                && (context.options.language_type.is_javascript()
                    || context.options.language_type.is_typescript())
            {
                let name = content.strip_prefix('#').unwrap_or(content);
                if name.is_empty() {
                    write!(f, [token("#")])?;
                } else {
                    write!(f, [token("#"), text(name)])?;
                }
                return Ok(());
            }

            let should_quote = quote_props == QuoteProperty::Consistent && force_quote_keys;
            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::String(string_id) => {
            let content = context.strings.get(string_id);
            let is_ident = is_identifier_for_quotes(content);
            let should_quote = match quote_props {
                QuoteProperty::Preserve => true,
                QuoteProperty::AsNeeded => force_quote_keys || !is_ident,
                QuoteProperty::Consistent => force_quote_keys || !is_ident,
            };

            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::Number(string_id) => {
            string_id.format(f)?;
        }
    }

    Ok(())
}

/// Format a quoted key using the preferred quote character.
fn format_quoted_name<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    string_id: StringId,
) -> FormatResult<()> {
    let mut quote_style = f.context().options.quote_style;
    if quote_style == QuoteStyle::Semantic && !f.context().options.language_type.is_destack() {
        quote_style = QuoteStyle::Double;
    }
    let content = f.context().strings.get(string_id);
    let quote_char = quote_style.char_for(content);
    let quote_str = if quote_char == '"' { "\"" } else { "'" };

    write!(f, [token(quote_str), string_id, token(quote_str)])
}
