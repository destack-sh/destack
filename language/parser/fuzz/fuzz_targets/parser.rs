#![no_main]

use std::sync::Arc;

use tspp_dir::Tree;
use tspp_parser::{ParseOptions, Parser};
use tspp_source::{File, FileId, FileType, LanguageType, ModuleId, PackageId, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // require one byte to select the source kind
    if data.is_empty() {
        return;
    }

    let (file_type, file_name) = file_from_byte(data[0]);
    let input = String::from_utf8_lossy(&data[1..]).into_owned();

    // set up minimal context for parsing
    let file_id = FileId::from_logical_str(file_name);
    let uri = Uri::from_string(file_name);
    let file = Arc::new(
        File::from_text(file_id, file_name.to_string(), uri, None, file_type, input)
            .expect("fuzz source should load"),
    );

    // parse the input
    let language = LanguageType::try_from(file_type).expect("file type has no parser language");
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let parser = Parser::new(
        file,
        language,
        Tree::new(module_id),
        ParseOptions::default(),
    );
    let _ = parser.parse();
});

/// Select one parser file type and its canonical name.
fn file_from_byte(byte: u8) -> (FileType, &'static str) {
    match byte % 2 {
        0 => (FileType::Tspp, "fuzz.tspp"),
        _ => (FileType::TsppDeclaration, "fuzz.d.tspp"),
    }
}
