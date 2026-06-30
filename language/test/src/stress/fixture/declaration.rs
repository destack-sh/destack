use super::StressMode;

/// Generate a large declaration surface.
pub(super) fn large_declaration(mode: StressMode, scale: usize, _width: usize) -> String {
    if mode.is_declaration() {
        let mut source = String::new();
        source.push_str("export declare interface LargeDeclaration<T> {\n");

        for index in 0..scale {
            source.push_str(&format!(
                "    readonly item{index}: T | Promise<T>;\n    read{index}(value: T): Result<T, Error>;\n"
            ));
        }

        source.push_str("}\n\n");
        source.push_str(
            "export declare function buildLargeDeclaration<T>(value: T): LargeDeclaration<T>;\n",
        );

        return source;
    }

    let mut source = String::new();
    source.push_str("export interface LargeDeclaration<T> {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    readonly item{index}: T | Promise<T>;\n    read{index}(value: T): Result<T, Error>;\n"
        ));
    }

    source.push_str("}\n\n");
    source.push_str("export const largeDeclaration = {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    item{index}: input.item{index},\n    read{index}(value) {{ return value; }},\n"
        ));
    }

    source.push_str("};\n");

    source
}

/// Generate damaged declarations with a later recovered declaration.
pub(super) fn damaged_declaration(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    if mode.is_declaration() {
        for index in 0..scale {
            source.push_str(&format!(
                "export declare interface BrokenDeclaration{index} {{\n    value: ;\n}}\n\n"
            ));
        }

        source.push_str("export declare const stressRecovered: string;\n");

        return source;
    }

    for index in 0..scale {
        source.push_str(&format!(
            "export class BrokenDeclaration{index} {{\n    value = ;\n}}\n\n"
        ));
    }

    source.push_str("export const stressRecovered = 1;\n");

    source
}
