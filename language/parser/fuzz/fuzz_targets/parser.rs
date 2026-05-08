#![no_main]

use std::path::Path;
use std::sync::Arc;

use destack_core::StringPool;
use destack_parser::Parser;
use destack_source::{File, FileId, FileType, LanguageType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // set up minimal context for parsing
    let file_id = FileId::new(0);
    let uri = Uri::from_path(Path::new("fuzz.ds"));
    let file = Arc::new(File::from_text(
        file_id,
        "fuzz.ds".to_string(),
        uri,
        None,
        FileType::Destack,
        input.to_string(),
    ));

    // parse the input
    let mut parser = Parser::lex_file(
        file,
        LanguageType::Destack,
        Arc::new(StringPool::new()),
    );
    let _ = parser.parse();
});
