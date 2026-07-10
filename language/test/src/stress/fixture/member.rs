/// Generate deep member and element access.
pub(super) fn deep_member(scale: usize, _width: usize) -> String {
    let mut expression = "root".to_string();

    for index in 0..scale {
        expression = format!("{expression}.child{index}[index{index}]?.next{index}");
    }

    format!("const deepMember = {expression};\n")
}
