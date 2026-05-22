#![no_main]

use std::sync::Arc;

use destack_core::StringPool;
use destack_parser::Parser;
use destack_source::{DiagnosticSeverity, File, FileId, FileType, LanguageType, Uri};
use destack_test::stress::{StressExpectation, generate_parser_fuzz_case};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let (input, file_type, expectation) = match data[0] % 3 {
        0 => {
            let file_type = file_type_from_byte(data.get(1).copied().unwrap_or(0));
            let Ok(source) = std::str::from_utf8(data.get(2..).unwrap_or_default()) else {
                return;
            };

            (source.to_string(), file_type, StressExpectation::Recovery)
        }
        _ => generate_parser_fuzz_case(&data[1..]),
    };
    let file_name = file_name_for_type(file_type);

    // set up minimal context for parsing
    let file_id = FileId::from_logical_str(file_name);
    let uri = Uri::from_string(file_name);
    let file = Arc::new(File::from_text(
        file_id,
        file_name.to_string(),
        uri,
        None,
        file_type,
        input,
    ));

    // parse the input
    let language = LanguageType::try_from(file_type).expect("file type has no parser language");
    let mut parser = Parser::lex_file(file, language, Arc::new(StringPool::new()));
    let _ = parser.parse();

    if expectation == StressExpectation::Valid
        && parser
            .diagnostics()
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        panic!("generated parser fuzz case produced diagnostics");
    }
});

fn file_type_from_byte(byte: u8) -> FileType {
    match byte % 5 {
        0 => FileType::Destack,
        1 => FileType::DestackDeclaration,
        2 => FileType::TypeScript,
        3 => FileType::TypeScriptXml,
        _ => FileType::TypeScriptDeclaration,
    }
}

fn file_name_for_type(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Destack => "fuzz.ds",
        FileType::DestackDeclaration => "fuzz.d.ds",
        FileType::TypeScript => "fuzz.ts",
        FileType::TypeScriptXml => "fuzz.tsx",
        FileType::TypeScriptDeclaration => "fuzz.d.ts",
        _ => "fuzz.ds",
    }
}
