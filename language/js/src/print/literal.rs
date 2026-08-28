use crate::{Literal, Module, Precedence, StringLiteral, TemplateElement, TemplateLiteral};

use super::printer::{PrintError, Printer};
use super::token::TokenClass;

impl Printer {
    /// Print one scalar JavaScript literal.
    pub(super) fn literal(&mut self, literal: &Literal, module: &Module) -> Result<(), PrintError> {
        match literal {
            Literal::Null => self.word("null"),
            Literal::Undefined => {
                self.word("void")?;
                self.number("0")
            }
            Literal::Boolean(value) => self.word(if *value { "true" } else { "false" }),
            Literal::Bigint(value) => {
                let text = format!("{value}n");

                self.bigint(&text)
            }
            Literal::Number(value) => self.number_literal(*value),
            Literal::String(value) => self.string(*value, module),
            Literal::RegexString { content, flags } => {
                let content = module.strings.get(*content);
                self.write_token("/", TokenClass::Regex)?;
                self.raw(content)?;
                self.raw("/")?;

                if let Some(flags) = flags {
                    self.raw(module.strings.get(*flags))?;
                }

                Ok(())
            }
        }
    }

    /// Print one template literal.
    pub(super) fn template(
        &mut self,
        template: &TemplateLiteral,
        module: &Module,
    ) -> Result<(), PrintError> {
        if let Some(tag) = template.tag {
            let needs_parentheses = module.tree.get(tag).is_optional_chain(&module.tree);
            if needs_parentheses {
                self.token("(")?;
            }
            self.expression(tag, Precedence::Call, module)?;
            if needs_parentheses {
                self.token(")")?;
            }
        }

        self.token("`")?;
        self.template_element(template.head, module)?;

        for substitution in &template.substitutions {
            self.token("${")?;
            self.expression(substitution.expression, Precedence::Lowest, module)?;
            self.token("}")?;
            self.template_element(substitution.tail, module)?;
        }

        self.token("`")
    }

    /// Print one attributed JavaScript string literal.
    pub(super) fn string(
        &mut self,
        string: StringLiteral,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.write_attributed(string.provenance, None, |printer| {
            printer.token("\"")?;

            for character in module.strings.get(string.value).chars() {
                printer.string_character(character)?;
            }

            printer.raw("\"")
        })
    }

    /// Print one number literal.
    fn number_literal(&mut self, value: f64) -> Result<(), PrintError> {
        if value.is_nan() {
            self.number("0")?;
            self.token("/")?;
            self.number("0")
        } else if value == f64::INFINITY {
            self.number("1")?;
            self.token("/")?;
            self.number("0")
        } else if value == f64::NEG_INFINITY {
            self.token("-")?;
            self.number("1")?;
            self.token("/")?;
            self.number("0")
        } else if value == 0.0 && value.is_sign_negative() {
            self.number("-0")
        } else {
            let text = value.to_string();

            self.number(&text)
        }
    }

    /// Print one attributed raw template element.
    fn template_element(
        &mut self,
        element: TemplateElement,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.write_attributed(element.provenance, None, |printer| {
            printer.raw(module.strings.get(element.raw))
        })
    }

    /// Print one escaped JavaScript string character.
    fn string_character(&mut self, character: char) -> Result<(), PrintError> {
        match character {
            '"' => self.raw("\\\""),
            '\\' => self.raw("\\\\"),
            '\u{0008}' => self.raw("\\b"),
            '\u{000c}' => self.raw("\\f"),
            '\n' => self.raw("\\n"),
            '\r' => self.raw("\\r"),
            '\t' => self.raw("\\t"),
            '\u{2028}' => self.raw("\\u2028"),
            '\u{2029}' => self.raw("\\u2029"),
            '\u{0000}'..='\u{001f}' => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let value = character as usize;
                let escape = [
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX[(value >> 4) & 0x0f],
                    HEX[value & 0x0f],
                ];

                // SAFETY: every byte comes from the ASCII escape prefix or hexadecimal table
                let escape = unsafe { std::str::from_utf8_unchecked(&escape) };

                self.raw(escape)
            }
            _ => {
                let mut encoded = [0; 4];
                let encoded = character.encode_utf8(&mut encoded);

                self.raw(encoded)
            }
        }
    }
}
