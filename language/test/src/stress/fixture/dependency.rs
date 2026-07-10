/// Generate many import and export declarations.
pub(super) fn large_import_export(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "import {{ item{index} as imported{index} }} from \"./module{index}\";\n"
        ));
    }

    source.push('\n');

    source.push_str("export const importedValues = [\n");

    for index in 0..scale {
        source.push_str(&format!("    imported{index},\n"));
    }

    source.push_str("];\n");

    source
}

/// Generate many imports followed by one ambient export.
pub(super) fn large_ambient_import_export(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "import {{ item{index} as imported{index} }} from \"./module{index}\";\n"
        ));
    }

    source.push_str("\nexport declare const importedValues: readonly unknown[];\n");

    source
}
