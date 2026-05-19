use super::StressMode;

/// Generate generic parsing ambiguity cases.
pub(super) fn ambiguous_generics(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    if mode.is_tsx() {
        source.push_str("const generic = <T,>(value: T) => value;\n");
    } else {
        source.push_str("const generic = <T>(value: T) => value;\n");
    }

    for index in 0..scale {
        source.push_str(&format!(
            "const call{index} = generic<{{ value: string; index: {index} }}>({{ value: \"x\", index: {index} }});\n"
        ));
    }

    source
}
