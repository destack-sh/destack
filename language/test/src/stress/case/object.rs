use super::StressMode;

/// Generate object literal ambiguity cases.
pub(super) fn ambiguous_objects(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    if mode.is_destack() {
        source.push_str("struct StressObject {\n    value: number;\n}\n\n");
    }

    source.push_str("const ambiguousObject = {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    method{index}<T extends {{ value: number }}>(value: T): T {{ return value; }},\n"
        ));
    }

    source.push_str("};\n");

    source
}

/// Generate a very large object literal.
pub(super) fn large_object(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const largeObject = {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    key{index}: {{ value: input[{index}], next: input[{index}] ?? fallback }},\n"
        ));
    }

    source.push_str("};\n");

    source
}
