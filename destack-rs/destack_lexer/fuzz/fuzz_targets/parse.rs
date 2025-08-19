#![no_main]

use destack_lexer::{render_tokens, tokenize};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let tokens: Vec<_> = tokenize(input).collect();
        let rendered_input = render_tokens(&tokens, input);
        assert_eq!(rendered_input, input);
        let reparsed_tokens: Vec<_> = tokenize(input).collect();
        assert_eq!(reparsed_tokens, tokens);
    }
});
