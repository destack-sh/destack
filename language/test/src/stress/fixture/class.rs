/// Generate a large class or class declaration.
pub(super) fn large_class(scale: usize, _width: usize) -> String {
    let mut source = "export final class LargeClass<T> {\n".to_string();
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

/// Generate a large ambient class declaration.
pub(super) fn large_ambient_class(scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("export declare class LargeClass<T> {\n");
    source.push_str("    readonly seed: T;\n");

    for index in 0..scale {
        source.push_str(&format!("    method{index}(value: T): T;\n"));
    }

    source.push_str("}\n");

    source
}
