#![no_main]

use destack_parser::Lexer;
use destack_source::{FileId, LanguageType};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // tokenize the input
    let file_id = FileId::new(0);
    let _ = Lexer::lex(file_id, input, LanguageType::Destack);
});
