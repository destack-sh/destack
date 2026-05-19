use std::fmt::Write;

use super::StressMode;

/// Generate ownership, borrow, move, and dereference surfaces.
pub(super) fn memory_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 520);
    source.push_str("struct Cell<T> { value: T; }\n");
    source.push_str("declare function useReadonly<T>(value: &readonly T): void;\n");
    source.push_str("declare function useExclusive<T>(value: &exclusive T): void;\n");
    source.push_str("declare function take<T>(value: ^T): void;\n\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "function memoryCase{index}(input: ^Cell<int32>): int32 {{"
        );
        source.push_str("    const readonlyValue = &readonly *input;\n");
        source.push_str("    const exclusiveValue = &exclusive *input;\n");
        source.push_str("    useReadonly(readonlyValue);\n");
        source.push_str("    useExclusive(exclusiveValue);\n");
        source.push_str("    const moved = ^input;\n");
        source.push_str("    take(moved);\n");
        let _ = writeln!(source, "    {index}");
        source.push_str("}\n\n");
    }

    source
}
