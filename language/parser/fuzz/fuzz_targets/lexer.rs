#![no_main]

use destack_parser::Lexer;
use destack_source::{FileId, LanguageOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // tokenize the input
    let file_id = FileId::new(0);
    let language = LanguageOptions::default();
    let _ = Lexer::lex(file_id, input, language);
});
