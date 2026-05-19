use super::StressMode;

/// Generate many import and export declarations.
pub(super) fn large_import_export(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "import {{ item{index} as imported{index} }} from \"./module{index}\";\n"
        ));
    }

    source.push('\n');

    if mode.is_declaration() {
        source.push_str("export declare const importedValues: readonly unknown[];\n");

        return source;
    }

    source.push_str("export const importedValues = [\n");

    for index in 0..scale {
        source.push_str(&format!("    imported{index},\n"));
    }

    source.push_str("];\n");

    source
}
