use std::fmt::Write;

/// Generate const expressions and static generic forms.
pub(super) fn const_forms(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 620);
    source.push_str("declare function compute(value: int32): int32;\n\n");

    for index in 0..scale {
        let _ = writeln!(source, "type Buffer{index}<const N: uint> = [uint8; N];");
        let _ = writeln!(source, "struct StaticBuffer{index}<const Size: uint> {{");
        source.push_str("    const {\n");
        source.push_str("        assert(Size > 0 && Size <= 65536);\n");
        source.push_str("    }\n");
        source.push_str("    tag: Size extends 4 ? \"small\" : \"large\";\n");
        source.push_str("    data: [uint8; Size];\n");
        source.push_str("}\n\n");

        let _ = writeln!(source, "const staticValue{index} = const {{");
        source.push_str("    let value = 1;\n");
        let _ = writeln!(source, "    value += compute({index});");
        source.push_str("    value\n");
        source.push_str("};\n\n");
    }

    source
}
