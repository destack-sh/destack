use super::StressMode;

/// Generate a large union or nominal type.
pub(super) fn large_type(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut union = Vec::new();

    for index in 0..scale {
        union.push(format!(
            "{{ kind: \"case{index}\"; value: T; next?: LargeType<T> }}"
        ));
    }

    if mode.is_destack() {
        format!("export type LargeType<T> = ({});\n", union.join(" | "))
    } else {
        format!("export type LargeType<T> = {};\n", union.join(" | "))
    }
}

/// Generate many conditional and inferred type members.
pub(super) fn convoluted_types(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("type Unwrap<T> = T extends Promise<infer Value> ? Value : T;\n");
    source.push_str("type Convoluted<T> = {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    readonly key{index}: Unwrap<T> extends infer Value ? readonly [Value, number] : never;\n"
        ));
    }

    source.push_str("};\n");

    if mode.is_destack() {
        source.push_str("type NominalConvoluted<T> = (Convoluted<T>);\n");
    }

    source
}

/// Generate a deeply nested type expression.
pub(super) fn deep_type(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = "type DeepType0 = Seed;\n".to_string();

    for index in 1..=scale {
        let previous = index - 1;
        source.push_str(&format!(
            "type DeepType{index} = {{ readonly item{index}: DeepType{previous}[]; next{index}: Promise<DeepType{previous}> | null }};\n"
        ));
    }

    if mode.is_destack() {
        source.push_str(&format!("type DeepType = (DeepType{scale});\n"));
    } else {
        source.push_str(&format!("type DeepType = DeepType{scale};\n"));
    }

    source
}
