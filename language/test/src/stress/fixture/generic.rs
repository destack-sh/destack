/// Generate generic parsing ambiguity cases.
pub(super) fn ambiguous_generics(scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const generic = <T,>(value: T) => value;\n");

    for index in 0..scale {
        source.push_str(&format!(
            "const call{index} = generic<{{ value: string; index: {index} }}>({{ value: \"x\", index: {index} }});\n"
        ));
    }

    source
}
