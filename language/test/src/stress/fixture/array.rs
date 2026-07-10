/// Generate a large array literal and fixed array syntax.
pub(super) fn large_array(scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const largeArray = [\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    {{ index: {index}, value: values[{index}] ?? fallback }},\n"
        ));
    }

    source.push_str("];\n");

    source.push_str("const fixedArray = [largeArray[0]; largeArray.length];\n");

    source
}
