use super::StressMode;

/// Generate a function with many parameters and blocks.
pub(super) fn large_signature(mode: StressMode, scale: usize, _width: usize) -> String {
    let parameters = (0..scale)
        .map(|index| format!("value{index}: number"))
        .collect::<Vec<_>>()
        .join(", ");

    if mode.is_declaration() {
        return format!("export declare function largeFunction({parameters}): number;\n");
    }

    let mut source = format!("export function largeFunction({parameters}): number {{\n");
    source.push_str("    let total = 0;\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    if (value{index} > total) {{\n        total = total + value{index};\n    }} else {{\n        total = total - value{index};\n    }}\n"
        ));
    }

    source.push_str("    return total;\n}\n");

    source
}
