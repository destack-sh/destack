use super::StressMode;

/// Generate a large interface surface.
pub(super) fn large_interface(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("export interface LargeInterface<In, Out> {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    readonly property{index}: In;\n    transform{index}(value: In, index: number): Out;\n"
        ));
    }

    source.push_str("}\n");

    source
}
