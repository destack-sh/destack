#![no_main]

use tspp_formatter::format_file_source;
use tspp_repository::FormatterOptions;
use tspp_source::{File, FileId, FileType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // require one byte to select the source kind
    if data.is_empty() {
        return;
    }

    let (file_type, file_name) = file_from_byte(data[0]);
    let input = String::from_utf8_lossy(&data[1..]).into_owned();

    let options = FormatterOptions::default();
    let file = fuzz_file(file_name, file_type, &input);
    let Ok(first) = format_file_source(&file, &input, options) else {
        return;
    };
    let file = fuzz_file(file_name, file_type, &first);
    let second = format_file_source(&file, &first, options)
        .unwrap_or_else(|error| panic!("formatted source failed to reformat: {error}"));

    // require canonical output after one pass
    if first != second {
        panic!("formatter fuzz target was not idempotent");
    }
});

/// Create one source file for a formatter pass.
fn fuzz_file(name: &str, file_type: FileType, source: &str) -> File {
    File::from_text(
        FileId::from_logical_str(name),
        name.to_string(),
        Uri::from_string(name),
        None,
        file_type,
        source.to_string(),
    )
    .expect("fuzz source should load")
}

/// Select one formatter file type and its canonical name.
fn file_from_byte(byte: u8) -> (FileType, &'static str) {
    match byte % 2 {
        0 => (FileType::Tspp, "fuzz.tspp"),
        _ => (FileType::TsppDeclaration, "fuzz.d.tspp"),
    }
}
