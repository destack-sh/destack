use destack_source::{FileId, ProvenanceTable};

use crate::{BytecodeFormatOptions, Parser, format_bytecode};

/// Format one bytecode fixture.
pub(crate) fn format_fixture(source: &str) -> String {
    let mut parser = Parser::new(FileId::new(0), source);
    let mut provenance = ProvenanceTable::build();
    let object = parser.parse(&mut provenance).expect("parse bytecode");
    let formatted = format_bytecode(
        &object,
        parser.function_names(),
        BytecodeFormatOptions::default(),
    )
    .expect("format bytecode");

    formatted.trim().to_string()
}

/// Assert exact canonical bytecode output.
#[track_caller]
pub(crate) fn assert_format_eq(input: &str, expected: &str) {
    let first = format_fixture(input);
    assert_eq!(first, expected.trim());

    let second = format_fixture(&first);
    assert_eq!(second, first);
}
