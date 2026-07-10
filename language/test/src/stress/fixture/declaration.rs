/// Generate a large declaration surface.
pub(super) fn large_declaration(scale: usize, _width: usize) -> String {
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

/// Generate a large ambient declaration surface.
pub(super) fn large_ambient_declaration(scale: usize, _width: usize) -> String {
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

    source
}

/// Generate damaged declarations with a later recovered declaration.
pub(super) fn damaged_declaration(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "export class BrokenDeclaration{index} {{\n    value = ;\n}}\n\n"
        ));
    }

    source.push_str("export const stressRecovered = 1;\n");

    source
}

/// Generate damaged ambient declarations with a later recovered declaration.
pub(super) fn damaged_ambient_declaration(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "export declare interface BrokenDeclaration{index} {{\n    value: ;\n}}\n\n"
        ));
    }

    source.push_str("export declare const stressRecovered: string;\n");

    source
}
