use super::buffer::{BufferQueue, SetResult};
use super::lexer::AttrValueKind::*;
use super::lexer::LexerMode::{
    AfterAttributeName, AfterAttributeValueQuoted, AfterDoctypeIdentifier, AfterDoctypeKeyword,
    AfterDoctypeName, AttributeName, AttributeValue, BeforeAttributeName, BeforeAttributeValue,
    BeforeDoctypeIdentifier, BeforeDoctypeName, BetweenDoctypePublicAndSystemIdentifiers,
    BogusComment, BogusDoctype, CdataSection, CdataSectionBracket, CdataSectionEnd, Comment,
    CommentEnd, CommentEndBang, CommentEndDash, CommentLessThanSign, CommentLessThanSignBang,
    CommentLessThanSignBangDash, CommentLessThanSignBangDashDash, CommentStart, CommentStartDash,
    Data, Doctype, DoctypeIdentifierDoubleQuoted, DoctypeIdentifierSingleQuoted, DoctypeName,
    EndTagOpen, MarkupDeclarationOpen, Plaintext, RawData, RawEndTagName, RawEndTagOpen,
    RawLessThanSign, ScriptDataDoubleEscapeEnd, ScriptDataEscapeStart, ScriptDataEscapeStartDash,
    ScriptDataEscapedDash, ScriptDataEscapedDashDash, SelfClosingStartTag, TagName, TagOpen,
};
use super::lexer::RawKind::{Rawtext, Rcdata, ScriptData, ScriptDataEscaped};
use super::lexer::ScriptEscapeKind::DoubleEscaped;
use super::lexer::{Lexer, LexerMode, ProcessResult};
use super::macros::go;
use super::reference::{CharacterReference, CharacterReferenceStatus};
use super::{HtmlString, LexHandler, LexerResult};

use std::borrow::Cow::Borrowed;

impl<Parser: LexHandler> Lexer<Parser> {
    /// Step the lexer once for the current state.
    pub(super) fn step(&self, input: &BufferQueue) -> ProcessResult<Parser::Handle> {
        if self.character_reference_tokenizer.borrow().is_some() {
            return self.step_character_reference_tokenizer(input);
        }

        match self.state.get() {
            // content-family states
            Data
            | LexerMode::Plaintext
            | RawData(_)
            | RawLessThanSign(_)
            | RawEndTagOpen(_)
            | RawEndTagName(_)
            | ScriptDataEscapeStart(_)
            | ScriptDataEscapeStartDash
            | ScriptDataEscapedDash(_)
            | ScriptDataEscapedDashDash(_)
            | ScriptDataDoubleEscapeEnd => self.step_content_state(input),

            // tag-family states
            TagOpen | EndTagOpen | TagName => self.step_tag_state(input),

            // attribute-family states
            BeforeAttributeName
            | AttributeName
            | AfterAttributeName
            | BeforeAttributeValue
            | AttributeValue(_)
            | AfterAttributeValueQuoted
            | SelfClosingStartTag => self.step_attribute_state(input),

            // markup-family states
            CommentStart
            | CommentStartDash
            | Comment
            | CommentLessThanSign
            | CommentLessThanSignBang
            | CommentLessThanSignBangDash
            | CommentLessThanSignBangDashDash
            | CommentEndDash
            | CommentEnd
            | CommentEndBang
            | Doctype
            | BeforeDoctypeName
            | DoctypeName
            | AfterDoctypeName
            | AfterDoctypeKeyword(_)
            | BeforeDoctypeIdentifier(_)
            | DoctypeIdentifierDoubleQuoted(_)
            | DoctypeIdentifierSingleQuoted(_)
            | AfterDoctypeIdentifier(_)
            | BetweenDoctypePublicAndSystemIdentifiers
            | BogusDoctype
            | BogusComment
            | MarkupDeclarationOpen
            | CdataSection
            | CdataSectionBracket
            | CdataSectionEnd => self.step_markup_state(input),
        }
    }
    fn step_character_reference_tokenizer(
        &self,
        input: &BufferQueue,
    ) -> ProcessResult<Parser::Handle> {
        let mut tokenizer_slot = self.character_reference_tokenizer.borrow_mut();
        let Some(character_reference_tokenizer) = tokenizer_slot.as_mut() else {
            return ProcessResult::Suspend;
        };

        match character_reference_tokenizer.step(self, input) {
            CharacterReferenceStatus::Done(character_reference) => {
                self.process_character_reference(character_reference);
                *tokenizer_slot = None;
                ProcessResult::Continue
            }

            CharacterReferenceStatus::Stuck => ProcessResult::Suspend,
            CharacterReferenceStatus::Progress => ProcessResult::Continue,
        }
    }

    /// Emit the finished character-reference payload.
    fn process_character_reference(&self, character_reference: CharacterReference) {
        let CharacterReference {
            mut chars,
            mut num_chars,
        } = character_reference;

        if num_chars == 0 {
            chars[0] = '&';
            num_chars = 1;
        }

        for i in 0..num_chars {
            let c = chars[i as usize];
            match self.state.get() {
                Data | RawData(Rcdata) => self.emit_char(c),

                AttributeValue(_) => go!(self: push_value c),

                _ => self.emit_char(c),
            }
        }
    }

    /// Indicate that we have reached the end of the input.
    pub(crate) fn end(&self) {
        // pending character reference
        let input = BufferQueue::default();
        match self.character_reference_tokenizer.take() {
            None => (),
            Some(mut tokenizer) => {
                self.process_character_reference(tokenizer.end_of_file(self, &input));
            }
        }

        // final lexer drain
        self.at_eof.set(true);
        if !matches!(self.run(&input), LexerResult::Done) {
            self.emit_error(Borrowed("Lexer did not finish after EOF"));
        }

        if !input.is_empty() {
            self.emit_error(Borrowed("Lexer finished with remaining input"));
        }

        loop {
            match self.eof_step() {
                ProcessResult::Continue => (),
                ProcessResult::Suspend => break,
                ProcessResult::Script(_) | ProcessResult::EncodingIndicator(_) => break,
            }
        }

        self.parser.end();

        if self.options.profile {
            self.dump_profile();
        }
    }

    /// Print the accumulated lexer profile.
    fn dump_profile(&self) {
        let mut results: Vec<(LexerMode, u64)> = self
            .state_profile
            .borrow()
            .iter()
            .map(|(s, t)| (*s, *t))
            .collect();
        results.sort_by(|&(_, x), &(_, y)| y.cmp(&x));

        let total: u64 = results
            .iter()
            .map(|&(_, t)| t)
            .fold(0, ::std::ops::Add::add);
        println!("\nTokenizer profile, in nanoseconds");
        println!(
            "\n{:12}         total in parser callbacks",
            self.time_in_parser.get()
        );
        println!("\n{total:12}         total in tokenizer");

        for (mode, elapsed_ns) in results.into_iter() {
            let pct = 100.0 * (elapsed_ns as f64) / (total as f64);
            println!("{elapsed_ns:12}  {pct:4.1}%  {mode:?}");
        }
    }

    /// Step the lexer after the input stream ends.
    fn eof_step(&self) -> ProcessResult<Parser::Handle> {
        match self.state.get() {
            Data | RawData(Rcdata) | RawData(Rawtext) | RawData(ScriptData) | Plaintext => {
                go!(self: eof)
            }

            TagName
            | RawData(ScriptDataEscaped(_))
            | BeforeAttributeName
            | AttributeName
            | AfterAttributeName
            | AttributeValue(_)
            | AfterAttributeValueQuoted
            | SelfClosingStartTag
            | ScriptDataEscapedDash(_)
            | ScriptDataEscapedDashDash(_) => {
                self.bad_eof_error();
                go!(self: to Data)
            }

            BeforeAttributeValue => go!(self: reconsume AttributeValue Unquoted),

            TagOpen => {
                self.bad_eof_error();
                self.emit_char('<');
                go!(self: to Data)
            }

            EndTagOpen => {
                self.bad_eof_error();
                self.emit_char('<');
                self.emit_char('/');
                go!(self: to Data)
            }

            RawLessThanSign(ScriptDataEscaped(DoubleEscaped)) => {
                go!(self: to RawData ScriptDataEscaped DoubleEscaped)
            }

            RawLessThanSign(kind) => {
                self.emit_char('<');
                go!(self: to RawData kind)
            }

            RawEndTagOpen(kind) => {
                self.emit_char('<');
                self.emit_char('/');
                go!(self: to RawData kind)
            }

            RawEndTagName(kind) => {
                self.emit_char('<');
                self.emit_char('/');
                self.emit_temp_buf();
                go!(self: to RawData kind)
            }

            ScriptDataEscapeStart(kind) => go!(self: to RawData ScriptDataEscaped kind),

            ScriptDataEscapeStartDash => go!(self: to RawData ScriptData),

            ScriptDataDoubleEscapeEnd => {
                go!(self: to RawData ScriptDataEscaped DoubleEscaped)
            }

            CommentStart | CommentStartDash | Comment | CommentEndDash | CommentEnd
            | CommentEndBang => {
                self.bad_eof_error();
                go!(self: emit_comment; to Data)
            }

            CommentLessThanSign | CommentLessThanSignBang => {
                go!(self: reconsume Comment)
            }

            CommentLessThanSignBangDash => go!(self: reconsume CommentEndDash),

            CommentLessThanSignBangDashDash => go!(self: reconsume CommentEnd),

            Doctype | BeforeDoctypeName => {
                self.bad_eof_error();
                go!(self: create_doctype; force_quirks; emit_doctype; to Data)
            }

            DoctypeName
            | AfterDoctypeName
            | AfterDoctypeKeyword(_)
            | BeforeDoctypeIdentifier(_)
            | DoctypeIdentifierDoubleQuoted(_)
            | DoctypeIdentifierSingleQuoted(_)
            | AfterDoctypeIdentifier(_)
            | BetweenDoctypePublicAndSystemIdentifiers => {
                self.bad_eof_error();
                go!(self: force_quirks; emit_doctype; to Data)
            }

            BogusDoctype => go!(self: emit_doctype; to Data),

            BogusComment => go!(self: emit_comment; to Data),

            MarkupDeclarationOpen => {
                self.bad_char_error();
                go!(self: to BogusComment)
            }

            CdataSection => {
                self.emit_temp_buf();
                self.bad_eof_error();
                go!(self: to Data)
            }

            CdataSectionBracket => go!(self: push_temp ']'; to CdataSection),

            CdataSectionEnd => go!(self: push_temp ']'; push_temp ']'; to CdataSection),
        }
    }

    /// Return whether the current CPU supports the lexer SIMD fast path.
    pub(super) fn is_supported_simd_feature_detected() -> bool {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            is_x86_feature_detected!("sse2")
        }

        #[cfg(target_arch = "aarch64")]
        {
            std::arch::is_aarch64_feature_detected!("neon")
        }

        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
        false
    }

    /// Scan the data state with SIMD instructions.
    ///
    /// This dispatches to the SSE2 or NEON helper for full chunks and then
    /// finishes any remaining bytes with scalar logic.
    ///
    /// The implementation follows the naive SIMD approach described [here].
    /// The caller must only invoke this on CPUs that support SSE2 or NEON.
    ///
    /// [data state]: https://html.spec.whatwg.org/#data-state
    /// [here]: https://lemire.me/blog/2024/06/08/scan-html-faster-with-simd-instructions-chrome-edition/
    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    pub(super) unsafe fn data_state_simd_fast_path(
        &self,
        input: &mut HtmlString,
    ) -> Option<SetResult> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let (mut i, mut n_newlines) = unsafe { self.data_state_sse2_fast_path(input) };

        #[cfg(target_arch = "aarch64")]
        let (mut i, mut n_newlines) = unsafe { self.data_state_neon_fast_path(input) };

        // remaining bytes
        while let Some(c) = input.as_bytes().get(i) {
            if matches!(*c, b'<' | b'&' | b'\r' | b'\0') {
                break;
            }
            if *c == b'\n' {
                n_newlines += 1;
            }

            i += 1;
        }

        let set_result = if i == 0 {
            let first_char = input.pop_front_char()?;
            debug_assert!(matches!(first_char, '<' | '&' | '\r' | '\0'));

            // this path cannot hit newline normalization, so the empty queue is harmless
            let preprocessed_char =
                self.get_preprocessed_char(first_char, &BufferQueue::default())?;
            SetResult::FromSet(preprocessed_char)
        } else {
            debug_assert!(
                input.len() >= i,
                "Trying to remove {:?} bytes from a string that is only {:?} bytes long",
                i,
                input.len()
            );
            let consumed_chunk = unsafe { input.unsafe_substring(0, i as u32) };
            unsafe { input.unsafe_pop_front(i as u32) };
            SetResult::NotFromSet(consumed_chunk)
        };

        self.current_line.set(self.current_line.get() + n_newlines);

        Some(set_result)
    }

    /// Scan the data state with SSE2 on x86 and x86_64.
    ///
    /// Return the number of processed bytes and the number of newline bytes.
    /// The caller must only invoke this on CPUs that support SSE2.
    ///
    /// [data state]: https://html.spec.whatwg.org/#data-state
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse2")]
    unsafe fn data_state_sse2_fast_path(&self, input: &mut HtmlString) -> (usize, u64) {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{
            __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
            _mm_set1_epi8,
        };
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{
            __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
            _mm_set1_epi8,
        };

        debug_assert!(!input.is_empty());

        let quote_mask = _mm_set1_epi8('<' as i8);
        let escape_mask = _mm_set1_epi8('&' as i8);
        let carriage_return_mask = _mm_set1_epi8('\r' as i8);
        let zero_mask = _mm_set1_epi8('\0' as i8);
        let newline_mask = _mm_set1_epi8('\n' as i8);

        let raw_bytes: &[u8] = input.as_bytes();
        let start = raw_bytes.as_ptr();

        const STRIDE: usize = 16;
        let mut i = 0;
        let mut n_newlines = 0;
        while i + STRIDE <= raw_bytes.len() {
            // chunk load
            let data = unsafe { _mm_loadu_si128(start.add(i) as *const __m128i) };

            // mask comparisons
            let quotes = _mm_cmpeq_epi8(data, quote_mask);
            let escapes = _mm_cmpeq_epi8(data, escape_mask);
            let carriage_returns = _mm_cmpeq_epi8(data, carriage_return_mask);
            let zeros = _mm_cmpeq_epi8(data, zero_mask);
            let newlines = _mm_cmpeq_epi8(data, newline_mask);

            // combined match mask
            let test_result = _mm_or_si128(
                _mm_or_si128(quotes, zeros),
                _mm_or_si128(escapes, carriage_returns),
            );
            let bitmask = _mm_movemask_epi8(test_result);
            let newline_mask = _mm_movemask_epi8(newlines);

            if bitmask != 0 {
                // first transition byte
                let position = if cfg!(target_endian = "little") {
                    bitmask.trailing_zeros() as usize
                } else {
                    bitmask.leading_zeros() as usize
                };

                n_newlines += (newline_mask & ((1 << position) - 1)).count_ones() as u64;
                i += position;
                break;
            } else {
                n_newlines += newline_mask.count_ones() as u64;
            }

            i += STRIDE;
        }

        (i, n_newlines)
    }

    /// Scan the data state with NEON on AArch64.
    ///
    /// Return the number of processed bytes and the number of newline bytes.
    /// The caller must only invoke this on CPUs that support NEON.
    ///
    /// [data state]: https://html.spec.whatwg.org/#data-state
    #[cfg(target_arch = "aarch64")]
    #[target_feature(enable = "neon")]
    unsafe fn data_state_neon_fast_path(&self, input: &mut HtmlString) -> (usize, u64) {
        use std::arch::aarch64::{vceqq_u8, vdupq_n_u8, vld1q_u8, vmaxvq_u8, vorrq_u8};

        debug_assert!(!input.is_empty());

        let quote_mask = vdupq_n_u8(b'<');
        let escape_mask = vdupq_n_u8(b'&');
        let carriage_return_mask = vdupq_n_u8(b'\r');
        let zero_mask = vdupq_n_u8(b'\0');
        let newline_mask = vdupq_n_u8(b'\n');

        let raw_bytes: &[u8] = input.as_bytes();
        let start = raw_bytes.as_ptr();

        const STRIDE: usize = 16;
        let mut i = 0;
        let mut n_newlines = 0;
        while i + STRIDE <= raw_bytes.len() {
            // chunk load
            let data = unsafe { vld1q_u8(start.add(i)) };

            // mask comparisons
            let quotes = vceqq_u8(data, quote_mask);
            let escapes = vceqq_u8(data, escape_mask);
            let carriage_returns = vceqq_u8(data, carriage_return_mask);
            let zeros = vceqq_u8(data, zero_mask);
            let newlines = vceqq_u8(data, newline_mask);

            // combined match mask
            let test_result =
                vorrq_u8(vorrq_u8(quotes, zeros), vorrq_u8(escapes, carriage_returns));
            let bitmask = vmaxvq_u8(test_result);
            let newline_mask = vmaxvq_u8(newlines);
            if bitmask != 0 {
                // first transition byte
                let chunk_bytes = unsafe { std::slice::from_raw_parts(start.add(i), STRIDE) };
                let Some(position) = chunk_bytes
                    .iter()
                    .position(|&b| matches!(b, b'<' | b'&' | b'\r' | b'\0'))
                else {
                    i += STRIDE;
                    continue;
                };

                n_newlines += chunk_bytes[..position]
                    .iter()
                    .filter(|&&b| b == b'\n')
                    .count() as u64;

                i += position;
                break;
            } else if newline_mask != 0 {
                let chunk_bytes = unsafe { std::slice::from_raw_parts(start.add(i), STRIDE) };
                n_newlines += chunk_bytes.iter().filter(|&&b| b == b'\n').count() as u64;
            }

            i += STRIDE;
        }

        (i, n_newlines)
    }
}
