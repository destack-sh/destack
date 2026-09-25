#![no_main]

use std::sync::Arc;

use tspp_parser::Lexer;
use tspp_source::{File, FileId, FileType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // tokenize the input
    let file_id = FileId::from_logical_str("fuzz.tspp");
    let file = Arc::new(
        File::from_text(
            file_id,
            "fuzz.tspp".to_string(),
            Uri::from_string("fuzz.tspp"),
            None,
            FileType::Tspp,
            input.to_string(),
        )
        .expect("fuzz source should load"),
    );
    let _ = Lexer::lex(file);
});
