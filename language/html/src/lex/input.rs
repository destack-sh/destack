use super::buffer::{BufferQueue, FromSet, SetResult};
use super::lexer::Lexer;
use super::{LexHandler, SmallCharSet};

use std::borrow::Cow;
use std::mem;

impl<Parser: LexHandler> Lexer<Parser> {
    /// Return the next preprocessed input character.
    pub(super) fn get_preprocessed_char(&self, mut c: char, input: &BufferQueue) -> Option<char> {
        // pending carriage-return normalization
        if self.ignore_lf.get() {
            self.ignore_lf.set(false);
            if c == '\n' {
                c = input.next()?;
            }
        }

        // newline normalization
        if c == '\r' {
            self.ignore_lf.set(true);
            c = '\n';
        }

        if c == '\n' {
            self.current_line.set(self.current_line.get() + 1);
        }

        // exact error mode
        if self.options.exact_errors
            && match c as u32 {
                0x01..=0x08 | 0x0B | 0x0E..=0x1F | 0x7F..=0x9F | 0xFDD0..=0xFDEF => true,
                number if (number & 0xFFFE) == 0xFFFE => true,
                _ => false,
            }
        {
            let message = format!("Bad character {c}");
            self.emit_error(Cow::Owned(message));
        }

        self.current_char.set(c);
        Some(c)
    }

    /// Return the next input character when one is available.
    pub(super) fn get_char(&self, input: &BufferQueue) -> Option<char> {
        // reconsume path
        if self.reconsume.get() {
            self.reconsume.set(false);
            return Some(self.current_char.get());
        }

        // normal input path
        input
            .next()
            .and_then(|character| self.get_preprocessed_char(character, input))
    }

    /// Pop one character or character run until the set matches.
    pub(super) fn pop_except_from(
        &self,
        input: &BufferQueue,
        set: SmallCharSet,
    ) -> Option<SetResult> {
        // slow path
        // `FromSet` can hold characters outside the set here, but the state
        // machine handles that the same way as `NotFromSet`
        if self.options.exact_errors || self.reconsume.get() || self.ignore_lf.get() {
            return self.get_char(input).map(FromSet);
        }

        // buffered pop
        let result = input.pop_except_from(set);
        match result {
            Some(FromSet(character)) => self.get_preprocessed_char(character, input).map(FromSet),

            // note: runs outside the set do not update `current_char`
            _ => result,
        }
    }

    /// Eat one exact or case-insensitive byte pattern from the input.
    pub(super) fn eat(
        &self,
        input: &BufferQueue,
        pattern: &str,
        equals: fn(&u8, &u8) -> bool,
    ) -> Option<bool> {
        // leading line-feed suppression
        if self.ignore_lf.get() {
            self.ignore_lf.set(false);
            if self.peek(input) == Some('\n') {
                self.discard_char(input);
            }
        }

        // match against the buffered lookahead
        input.push_front(mem::take(&mut self.temp_buf.borrow_mut()));
        match input.eat(pattern, equals) {
            None if self.at_eof.get() => Some(false),
            None => {
                while let Some(character) = input.next() {
                    self.temp_buf.borrow_mut().push_char(character);
                }

                None
            }
            Some(matched) => Some(matched),
        }
    }

    /// Peek one raw input character.
    pub(super) fn peek(&self, input: &BufferQueue) -> Option<char> {
        // reconsume path
        if self.reconsume.get() {
            return Some(self.current_char.get());
        }

        // normal input path
        input.peek()
    }

    /// Discard one raw input character.
    pub(super) fn discard_char(&self, input: &BufferQueue) {
        // peek() deals in un-processed characters, while get_char() does newline normalization
        //
        // since discard_char is paired with peek(), it must discard one raw input character
        if self.reconsume.get() {
            self.reconsume.set(false);
        } else {
            input.next();
        }
    }
}
