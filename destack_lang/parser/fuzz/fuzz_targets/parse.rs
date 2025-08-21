#![no_main]

use destack_parser::lexer::{render_tokens, tokenize};
use libfuzzer_sys::fuzz_target;

/// Tokenize an input string in a roundtrip.
macro_rules! assert_tokenize_roundtrip {
    ($input:expr) => {
        // tokenize & render back to input string
        let tokens: Vec<_> = tokenize($input).collect();
        let rendered_input = render_tokens(&tokens, $input);
        assert_eq!(rendered_input, $input);
        // tokenize *again* on the rendered input
        let reparsed_tokens: Vec<_> = tokenize(&rendered_input).collect();
        assert_eq!(reparsed_tokens, tokens);
    };
}

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        assert_tokenize_roundtrip!(input);
    }
});
