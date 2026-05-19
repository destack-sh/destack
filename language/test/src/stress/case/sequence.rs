use std::fmt::Write;

use super::StressMode;

/// Generate Destack sequence forms.
pub(super) fn sequence_type_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 512);
    source.push_str("type ByteSlice = [uint8];\n");
    source.push_str("type ByteBlock<comptime N: uint> = [uint8; N];\n");
    source.push_str("type PointTuple = (int32, int32);\n\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const tuple{index}: (int32, int32) = ({index}, {index});"
        );
        let _ = writeln!(source, "const singleton{index}: (int32,) = ({index},);");
        let _ = writeln!(source, "const fixed{index}: [uint8; 4] = [{index}; 4];");
        let _ = writeln!(
            source,
            "const slice{index}: [uint8] = [fixed{index}[0], fixed{index}[1], fixed{index}[2], fixed{index}[3]];"
        );
    }

    source
}

/// Generate Destack sequence pattern forms.
pub(super) fn sequence_pattern_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 360);
    source.push_str("declare function parsePng(bytes: [uint8]): int32;\n");
    source.push_str("declare function parseJpeg(bytes: [uint8]): int32;\n\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "function sequencePattern{index}(bytes: [uint8]): int32 {{"
        );
        source.push_str("    return match (bytes) {\n");
        source.push_str("        [0x89, 0x50, 0x4e, 0x47, ...rest] => parsePng(rest)\n");
        source.push_str("        [0xff, 0xd8, ...rest] => parseJpeg(rest)\n");
        source.push_str("        [first, second, ...rest] if (first == second) => rest.length\n");
        source.push_str("        _ => 0\n");
        source.push_str("    };\n");
        source.push_str("}\n\n");
    }

    source
}
