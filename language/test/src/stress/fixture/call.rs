use super::StressMode;

/// Generate deeply nested calls and argument lists.
pub(super) fn deep_call(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut expression = "input".to_string();

    for index in 0..scale {
        expression = format!(
            "wrap{index}({expression}, value{index}, () => fallback{index}, {{ index: {index} }})"
        );
    }

    format!("const deepCall = {expression};\n")
}
