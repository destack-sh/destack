use super::StressMode;

/// Generate heavily nested conditional expressions.
pub(super) fn nested_ternary(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut expression = "fallback".to_string();

    for index in (0..scale).rev() {
        expression = format!("flag{index} ? value{index} : ({expression})");
    }

    format!("const nestedTernary = {expression};\n")
}
