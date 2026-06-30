use std::fmt::Write;

use super::StressMode;

/// Generate comptime expressions and static generic forms.
pub(super) fn comptime_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 620);
    source.push_str("declare function compute(value: int32): int32;\n\n");

    for index in 0..scale {
        let _ = writeln!(source, "type Buffer{index}<comptime N: uint> = [uint8; N];");
        let _ = writeln!(source, "struct StaticBuffer{index}<comptime Size: uint> {{");
        source.push_str("    comptime {\n");
        source.push_str("        assert(Size > 0 && Size <= 65536);\n");
        source.push_str("    }\n");
        source.push_str("    @if(Size == 4)\n");
        source.push_str("    tag: \"small\";\n");
        source.push_str("    @if(Size != 4)\n");
        source.push_str("    tag: \"large\";\n");
        source.push_str("    data: [uint8; Size];\n");
        source.push_str("}\n\n");

        let _ = writeln!(source, "const staticValue{index} = comptime {{");
        source.push_str("    let value = 1;\n");
        let _ = writeln!(source, "    value += compute({index});");
        source.push_str("    value\n");
        source.push_str("};\n\n");
    }

    source
}
