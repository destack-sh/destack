use super::StressMode;

/// Generate many nested statement blocks.
pub(super) fn nested_block(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("export function nestedBlock(input: number): number {\n");
    source.push_str("    let value = input;\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    if (value > {index}) {{\n        for (let step = 0; step < {index}; step++) {{\n            value = value + step;\n        }}\n    }} else {{\n        value = value - {index};\n    }}\n"
        ));
    }

    source.push_str("    return value;\n}\n");

    source
}

/// Generate deeply layered control flow.
pub(super) fn control_flow(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("export function controlFlow(values: number[]): number {\n");
    source.push_str("    let total = 0;\n");
    source.push_str("    outer: for (let index = 0; index < values.length; index++) {\n");

    for depth in 0..scale {
        source.push_str(&format!(
            "        if (values[index] === {depth}) {{\n            continue outer;\n        }}\n"
        ));
    }

    source.push_str("        switch (values[index]) {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "            case {index}: {{\n                total = total + {index};\n                break;\n            }}\n"
        ));
    }

    source.push_str("            default: {\n                total = total + values[index];\n            }\n        }\n    }\n    return total;\n}\n");

    source
}
