#![no_main]

use std::sync::Arc;

use destack_parser::Lexer;
use destack_source::{File, FileId, FileType, LanguageType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // tokenize the input
    let file_id = FileId::from_logical_str("fuzz.ds");
    let file = Arc::new(File::from_text(
        file_id,
        "fuzz.ds".to_string(),
        Uri::from_string("fuzz.ds"),
        None,
        FileType::Destack,
        input.to_string(),
    ));
    let _ = Lexer::lex(file, LanguageType::Destack);
});
