#![no_main]

use destack_formatter::format_file_source;
use destack_source::{File, FileId, FileType, Uri};
use destack_test::stress::{StressExpectation, generate_formatter_fuzz_case};
use destack_workspace::FormatterOptions;
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
        _ => generate_formatter_fuzz_case(&data[1..]),
    };
    let file_name = file_name_for_type(file_type);

    let options = FormatterOptions::default();
    let file = fuzz_file(file_name, file_type, &input);
    let Ok(first) = format_file_source(&file, &input, options) else {
        return;
    };
    let file = fuzz_file(file_name, file_type, &first);
    let Ok(second) = format_file_source(&file, &first, options) else {
        return;
    };

    if first != second {
        panic!("formatter fuzz target was not idempotent");
    }

    if expectation == StressExpectation::Valid && first.is_empty() && !input.is_empty() {
        panic!("generated formatter fuzz case formatted to empty output");
    }
});

fn fuzz_file(name: &str, file_type: FileType, source: &str) -> File {
    File::from_text(
        FileId::from_logical_str(name),
        name.to_string(),
        Uri::from_string(name),
        None,
        file_type,
        source.to_string(),
    )
}

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
