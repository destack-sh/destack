use super::buffer::BufferQueue;
use super::lexer::Lexer;
use super::token::LexHandler;
use super::{HtmlString, entity};

use std::borrow::Cow::{self, Borrowed};
use std::char::from_u32;
use std::mem;

/// One parsed character-reference payload.
pub(super) struct CharacterReference {
    /// The resulting characters.
    pub(super) chars: [char; 2],
    /// The number of valid character slots.
    pub(super) num_chars: u8,
}

/// One character-reference tokenizer step result.
pub(super) enum CharacterReferenceStatus {
    /// The tokenizer needs more input.
    Stuck,
    /// The tokenizer advanced but is not done yet.
    Progress,
    /// The tokenizer finished one character reference.
    Done(CharacterReference),
}

/// One character-reference tokenizer mode.
#[derive(Debug)]
enum CharacterReferenceMode {
    /// The initial character-reference state.
    Begin,
    /// The state after `&#`.
    Octothorpe,
    /// The numeric reference state with one active base.
    Numeric(u32),
    /// The numeric semicolon validation state.
    NumericSemicolon,
    /// The named reference state.
    Named,
    /// The bogus named reference state.
    BogusName,
}

/// One stateful character-reference tokenizer.
#[derive(Debug)]
pub(super) struct CharacterReferenceTokenizer {
    /// The current tokenizer mode.
    state: CharacterReferenceMode,
    /// Whether the reference is being consumed inside an attribute value.
    is_consumed_in_attribute: bool,

    /// The current numeric value.
    numeric_value: u32,
    /// Whether the numeric value exceeded the scalar range.
    is_numeric_value_too_big: bool,
    /// Whether at least one numeric digit was seen.
    seen_digit: bool,
    /// The optional hexadecimal marker.
    hexadecimal_marker: Option<char>,

    /// The buffered named reference text.
    named_reference: HtmlString,
    /// The best named-entity match so far.
    best_match: Option<(u32, u32)>,
    /// The byte length of the best named-entity match.
    best_match_length: usize,
}

impl CharacterReference {
    /// The empty character-reference result.
    const EMPTY: CharacterReference = CharacterReference {
        chars: ['\0', '\0'],
        num_chars: 0,
    };
}

impl CharacterReferenceTokenizer {
    /// Create one new character-reference tokenizer.
    pub(super) fn new(is_consumed_in_attribute: bool) -> CharacterReferenceTokenizer {
        CharacterReferenceTokenizer {
            is_consumed_in_attribute,
            state: CharacterReferenceMode::Begin,
            numeric_value: 0,
            is_numeric_value_too_big: false,
            seen_digit: false,
            hexadecimal_marker: None,
            named_reference: HtmlString::new(),
            best_match: None,
            best_match_length: 0,
        }
    }

    /// Return the buffered named reference text.
    fn named_reference(&self) -> &HtmlString {
        &self.named_reference
    }

    /// Return the mutable buffered named reference text.
    fn named_reference_mut(&mut self) -> &mut HtmlString {
        &mut self.named_reference
    }

    /// Finish with one single-character result.
    fn finish_one(&mut self, c: char) -> CharacterReferenceStatus {
        CharacterReferenceStatus::Done(CharacterReference {
            chars: [c, '\0'],
            num_chars: 1,
        })
    }

    /// Convert one scalar value into one character or replacement marker.
    fn scalar_or_replacement(value: u32) -> char {
        from_u32(value).unwrap_or('\u{fffd}')
    }
}

impl CharacterReferenceTokenizer {
    /// Step the character-reference tokenizer once.
    pub(super) fn step<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        match self.state {
            CharacterReferenceMode::Begin => self.do_begin(lexer, input),
            CharacterReferenceMode::Octothorpe => self.do_octothorpe(lexer, input),
            CharacterReferenceMode::Numeric(base) => self.do_numeric(lexer, input, base),
            CharacterReferenceMode::NumericSemicolon => self.do_numeric_semicolon(lexer, input),
            CharacterReferenceMode::Named => self.do_named(lexer, input),
            CharacterReferenceMode::BogusName => self.do_bogus_name(lexer, input),
        }
    }

    /// Process the initial character-reference state.
    fn do_begin<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        match lexer.peek(input) {
            Some('a'..='z' | 'A'..='Z' | '0'..='9') => {
                self.state = CharacterReferenceMode::Named;
                self.named_reference.clear();
                CharacterReferenceStatus::Progress
            }
            Some('#') => {
                lexer.discard_char(input);
                self.state = CharacterReferenceMode::Octothorpe;
                CharacterReferenceStatus::Progress
            }
            Some(_) => CharacterReferenceStatus::Done(CharacterReference::EMPTY),
            None => CharacterReferenceStatus::Stuck,
        }
    }

    /// Process the `&#` state.
    fn do_octothorpe<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        match lexer.peek(input) {
            Some(c @ ('x' | 'X')) => {
                lexer.discard_char(input);
                self.hexadecimal_marker = Some(c);
                self.state = CharacterReferenceMode::Numeric(16);
            }
            Some(_) => {
                self.hexadecimal_marker = None;
                self.state = CharacterReferenceMode::Numeric(10);
            }
            None => return CharacterReferenceStatus::Stuck,
        }
        CharacterReferenceStatus::Progress
    }

    /// Process one numeric character-reference step.
    fn do_numeric<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
        base: u32,
    ) -> CharacterReferenceStatus {
        let Some(c) = lexer.peek(input) else {
            return CharacterReferenceStatus::Stuck;
        };
        match c.to_digit(base) {
            Some(n) => {
                lexer.discard_char(input);
                self.numeric_value = self.numeric_value.wrapping_mul(base);
                if self.numeric_value > 0x10FFFF {
                    // invalid numeric value
                    // keep scanning digits and the semicolon, but ignore the final number
                    self.is_numeric_value_too_big = true;
                }
                self.numeric_value = self.numeric_value.wrapping_add(n);
                self.seen_digit = true;
                CharacterReferenceStatus::Progress
            }

            None if !self.seen_digit => self.unconsume_numeric(lexer, input),

            None => {
                self.state = CharacterReferenceMode::NumericSemicolon;
                CharacterReferenceStatus::Progress
            }
        }
    }

    /// Process the numeric semicolon validation state.
    fn do_numeric_semicolon<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        match lexer.peek(input) {
            Some(';') => lexer.discard_char(input),
            Some(_) => lexer.emit_error(Borrowed(
                "Semicolon missing after numeric character reference",
            )),
            None => return CharacterReferenceStatus::Stuck,
        };
        self.finish_numeric(lexer)
    }

    /// Unconsume one malformed numeric prefix.
    fn unconsume_numeric<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        let mut unconsume = HtmlString::from_char('#');
        if let Some(c) = self.hexadecimal_marker {
            unconsume.push_char(c)
        }

        input.push_front(unconsume);
        lexer.emit_error(Borrowed("Numeric character reference without digits"));
        CharacterReferenceStatus::Done(CharacterReference::EMPTY)
    }

    /// Finish one numeric character reference.
    fn finish_numeric<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
    ) -> CharacterReferenceStatus {
        // numeric replacement
        let (c, error) = match self.numeric_value {
            n if (n > 0x10FFFF) || self.is_numeric_value_too_big => ('\u{fffd}', true),
            0x00 | 0xD800..=0xDFFF => ('\u{fffd}', true),

            0x80..=0x9F => match entity::C1_REPLACEMENTS[(self.numeric_value - 0x80) as usize] {
                Some(c) => (c, true),
                None => (Self::scalar_or_replacement(self.numeric_value), true),
            },

            0x01..=0x08 | 0x0B | 0x0D..=0x1F | 0x7F | 0xFDD0..=0xFDEF => {
                (Self::scalar_or_replacement(self.numeric_value), true)
            }

            n if (n & 0xFFFE) == 0xFFFE => (Self::scalar_or_replacement(n), true),

            n => (Self::scalar_or_replacement(n), false),
        };

        // parse error
        if error {
            let msg = if lexer.options.exact_errors {
                Cow::from(format!(
                    "Invalid numeric character reference value 0x{:06X}",
                    self.numeric_value
                ))
            } else {
                Cow::from("Invalid numeric character reference")
            };
            lexer.emit_error(msg);
        }

        self.finish_one(c)
    }

    /// Process one named character-reference step.
    fn do_named<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        // raw peek path
        // peek + discard skips newline normalization, which makes unconsume easier
        let Some(c) = lexer.peek(input) else {
            return CharacterReferenceStatus::Stuck;
        };
        lexer.discard_char(input);
        self.named_reference_mut().push_char(c);

        // entity lookup
        match entity::lookup_named_entity(&self.named_reference()[..]) {
            // full match or prefix
            Some(m) => {
                if m.0 != 0 {
                    // latest full match
                    self.best_match = Some(m);
                    self.best_match_length = self.named_reference().len();
                }

                // prefix match
                CharacterReferenceStatus::Progress
            }

            // no further match
            None => self.finish_named(lexer, input, Some(c)),
        }
    }

    /// Emit one named-reference parse error.
    fn emit_name_error<Sink: LexHandler>(&mut self, lexer: &Lexer<Sink>) {
        let msg = if lexer.options.exact_errors {
            Cow::from(format!(
                "Invalid character reference &{}",
                self.named_reference()
            ))
        } else {
            Cow::from("Invalid character reference")
        };
        lexer.emit_error(msg);
    }

    /// Push the buffered name back into the input queue.
    fn unconsume_name(&mut self, input: &BufferQueue) {
        input.push_front(mem::take(&mut self.named_reference));
    }

    /// Finish one named character reference.
    fn finish_named<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
        end_char: Option<char>,
    ) -> CharacterReferenceStatus {
        match self.best_match {
            None => {
                // incomplete named match
                match end_char {
                    Some(c) if c.is_ascii_alphanumeric() => {
                        // keep scanning for a semicolon
                        self.state = CharacterReferenceMode::BogusName;
                        return CharacterReferenceStatus::Progress;
                    }

                    // `&;` is not a parse error
                    Some(';') if self.named_reference().len() > 1 => self.emit_name_error(lexer),

                    _ => (),
                }
                self.unconsume_name(input);
                CharacterReferenceStatus::Done(CharacterReference::EMPTY)
            }

            Some((c1, c2)) => {
                // completed match metadata
                // the buffer can extend past the best match:
                // `&not` matches, `&noti` is still viable, `&notit` stops

                let best_match_length = self.best_match_length;
                if best_match_length == 0 {
                    self.emit_name_error(lexer);
                    self.unconsume_name(input);
                    return CharacterReferenceStatus::Done(CharacterReference::EMPTY);
                }

                // following character
                let Some(last_matched) = self.named_reference()[best_match_length - 1..]
                    .chars()
                    .next()
                else {
                    self.emit_name_error(lexer);
                    self.unconsume_name(input);
                    return CharacterReferenceStatus::Done(CharacterReference::EMPTY);
                };

                // there may be no following character after a full match at eof
                let next_after = if best_match_length == self.named_reference().len() {
                    None
                } else {
                    self.named_reference()[best_match_length..].chars().next()
                };

                // attribute ambiguity
                // attribute values keep the source text for historical compat
                // when the match has no semicolon and the next byte is `=` or alphanumeric

                let unconsume_all = match (self.is_consumed_in_attribute, last_matched, next_after)
                {
                    (_, ';', _) => false,
                    (true, _, Some('=')) => true,
                    (true, _, Some(c)) if c.is_ascii_alphanumeric() => true,
                    _ => {
                        // missing semicolon
                        lexer.emit_error(Borrowed(
                            "Character reference does not end with semicolon",
                        ));
                        false
                    }
                };

                // either unconsume or emit the matched characters
                if unconsume_all {
                    self.unconsume_name(input);
                    CharacterReferenceStatus::Done(CharacterReference::EMPTY)
                } else {
                    input.push_front(HtmlString::from_slice(
                        &self.named_reference()[best_match_length..],
                    ));
                    lexer.ignore_lf.set(false);
                    CharacterReferenceStatus::Done(CharacterReference {
                        chars: [
                            from_u32(c1).unwrap_or('\u{fffd}'),
                            from_u32(c2).unwrap_or('\0'),
                        ],
                        num_chars: if c2 == 0 { 1 } else { 2 },
                    })
                }
            }
        }
    }

    /// Process one bogus named reference step.
    fn do_bogus_name<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReferenceStatus {
        // peek + discard skips over newline normalization, therefore making it easier to
        // un-consume
        let Some(c) = lexer.peek(input) else {
            return CharacterReferenceStatus::Stuck;
        };
        lexer.discard_char(input);
        self.named_reference_mut().push_char(c);
        match c {
            _ if c.is_ascii_alphanumeric() => return CharacterReferenceStatus::Progress,
            ';' => self.emit_name_error(lexer),
            _ => (),
        }
        self.unconsume_name(input);
        CharacterReferenceStatus::Done(CharacterReference::EMPTY)
    }

    /// Finish the character reference at end of file.
    pub(super) fn end_of_file<Sink: LexHandler>(
        &mut self,
        lexer: &Lexer<Sink>,
        input: &BufferQueue,
    ) -> CharacterReference {
        loop {
            let status = match self.state {
                CharacterReferenceMode::Begin => {
                    CharacterReferenceStatus::Done(CharacterReference::EMPTY)
                }
                CharacterReferenceMode::Numeric(_) if !self.seen_digit => {
                    self.unconsume_numeric(lexer, input)
                }
                CharacterReferenceMode::Numeric(_) | CharacterReferenceMode::NumericSemicolon => {
                    lexer.emit_error(Borrowed("EOF in numeric character reference"));
                    self.finish_numeric(lexer)
                }
                CharacterReferenceMode::Named => self.finish_named(lexer, input, None),
                CharacterReferenceMode::BogusName => {
                    self.unconsume_name(input);
                    CharacterReferenceStatus::Done(CharacterReference::EMPTY)
                }
                CharacterReferenceMode::Octothorpe => {
                    input.push_front(HtmlString::from_slice("#"));
                    lexer.emit_error(Borrowed("EOF after '#' in character reference"));
                    CharacterReferenceStatus::Done(CharacterReference::EMPTY)
                }
            };

            match status {
                CharacterReferenceStatus::Done(char_ref) => {
                    return char_ref;
                }
                CharacterReferenceStatus::Stuck => {
                    return CharacterReference::EMPTY;
                }
                CharacterReferenceStatus::Progress => {}
            }
        }
    }
}
