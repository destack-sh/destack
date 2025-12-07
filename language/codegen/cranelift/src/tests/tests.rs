use std::num::NonZeroU32;

use destack_mir as mir;
use destack_source::{ModuleId, StringId};
use destack_workspace::ModuleMir;

use crate::CodegenCraneliftBackend;

/// Helper to compile MIR text to CLIF text.
pub(crate) fn compile_mir_to_clif(source: &str) -> String {
    let (tree, strings) = mir::parse::Parser::parse(source).expect("failed to parse MIR");

    // create a ModuleMir and populate it
    let module = ModuleMir::new(ModuleId::EPHEMERAL);
    *module.tree.write() = tree;

    // copy strings into module's string pool
    for i in 0..strings.len() {
        let id = StringId(NonZeroU32::new((i + 1) as u32).unwrap());
        let s = strings.get(id);
        module.strings.intern(s);
    }

    let backend = CodegenCraneliftBackend::native().expect("failed to create backend");
    backend
        .compile_to_clif(&module, "test")
        .expect("failed to compile")
}

/// Normalize CLIF output for cross-platform comparison:
/// - Removes header comments (lines starting with ";")
/// - Normalizes calling convention to "native"
/// - Removes trailing comments ("; ..." at end of lines)
pub(crate) fn normalize_clif(clif: &str) -> String {
    // known calling conventions to normalize
    let calling_conventions = [
        "apple_aarch64",
        "system_v",
        "windows_fastcall",
        "fast",
        "cold",
        "probestack",
    ];

    let lines: Vec<String> = clif
        .lines()
        .filter(|line| !line.starts_with("; function:"))
        .map(|line| {
            // strip trailing comments like "; v0 = 42"
            if let Some(idx) = line.find("  ;") {
                line[..idx].to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    let mut result = lines.join("\n");

    // normalize calling convention to "native"
    for calling_convention in calling_conventions {
        result = result.replace(calling_convention, "native");
    }

    result.trim().to_string()
}

/// Compile MIR text to normalized CLIF text.
pub(crate) fn compile_mir_to_normalized_clif(source: &str) -> String {
    let clif = compile_mir_to_clif(source);
    normalize_clif(&clif)
}
