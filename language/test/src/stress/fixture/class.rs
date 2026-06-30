use super::StressMode;

/// Generate a large class or class declaration.
pub(super) fn large_class(mode: StressMode, scale: usize, _width: usize) -> String {
    if mode.is_declaration() {
        let mut source = String::new();
        source.push_str("export declare class LargeClass<T> {\n");
        source.push_str("    readonly seed: T;\n");

        for index in 0..scale {
            source.push_str(&format!("    method{index}(value: T): T;\n"));
        }

        source.push_str("}\n");

        return source;
    }

    let prefix = if mode.is_destack() { "final " } else { "" };
    let mut source = format!("export {prefix}class LargeClass<T> {{\n");
    source.push_str("    readonly seed: T;\n\n");
    source.push_str("    constructor(seed: T) {\n        this.seed = seed;\n    }\n\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    method{index}(value: T): T {{\n        return value ?? this.seed;\n    }}\n\n"
        ));
    }

    source.push_str("}\n");

    source
}
