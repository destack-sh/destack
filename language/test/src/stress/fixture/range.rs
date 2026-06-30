use std::fmt::Write;

use super::StressMode;

/// Generate range type, expression, subscript, and pattern surfaces.
pub(super) fn range_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 560);
    source.push_str("type Digit = 0..=9;\n");
    source.push_str("type LowerAscii = 'a'..='z';\n");
    source.push_str("declare const values: [int32];\n\n");

    for index in 0..scale {
        let end = index + 8;
        let _ = writeln!(source, "type Window{index} = {index}..={end};");
        let _ = writeln!(source, "const open{index} = {index}..{end};");
        let _ = writeln!(source, "const closed{index} = {index}..={end};");
        let _ = writeln!(source, "const from{index} = {index}..;");
        let _ = writeln!(source, "const to{index} = ..{end};");
        let _ = writeln!(source, "const through{index} = ..={end};");
        let _ = writeln!(source, "const full{index} = ..;");
        let _ = writeln!(source, "const slice{index} = values[{index}..{end}];");
        let _ = writeln!(
            source,
            "const readonlySlice{index} = (&readonly values)[{index}..={end}];"
        );
        let _ = writeln!(source, "function classify{index}(value: int32): int32 {{");
        source.push_str("    return match (value) {\n");
        source.push_str("        0..=9 => value\n");
        source.push_str("        ..=255 => 255\n");
        source.push_str("        _ => -1\n");
        source.push_str("    };\n");
        source.push_str("}\n\n");
    }

    source
}
