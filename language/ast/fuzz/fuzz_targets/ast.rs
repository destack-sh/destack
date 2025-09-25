#![no_main]

use dyst_ast::{BlockFormat, Parser};
use dyst_source::{Source, SourceId};
use dyst_token::TokenType;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let source = Source::new(SourceId::new(0), "<fuzz>".to_string(), input.to_string());
        let mut parser = Parser::from_source(&source);
        let _ = parser.with_recovery(
            parser.mark(),
            |parser| parser.eat_block_body(BlockFormat::Implicit),
            Vec::new(),
            TokenType::End,
        );
    }
});

