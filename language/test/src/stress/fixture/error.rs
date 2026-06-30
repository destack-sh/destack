use std::fmt::Write;

use super::StressMode;

/// Generate Result and Try integration surfaces.
pub(super) fn error_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 720);
    source.push_str("declare function read(): Result<int32, MissingError | FormatError>;\n");
    source.push_str("declare function readAsync(): Promise<Result<int32, MissingError>>;\n");
    source.push_str("declare function fallback(): int32;\n");
    source.push_str("struct MissingError { path: string; }\n");
    source.push_str("struct FormatError { line: int32; }\n\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "async function errorCase{index}(): Promise<int32> {{"
        );
        source.push_str("    const first = read()?;\n");
        source.push_str("    const second = await? readAsync();\n");
        source.push_str("    const third = await! readAsync();\n");
        source.push_str("    const local = read() ?? fallback();\n");
        source.push_str("    const forced = read()!;\n");
        source.push_str("    const recovered = try {\n");
        source.push_str("        first + second + third + local + forced\n");
        source.push_str("    } catch match (failure) {\n");
        source.push_str("        MissingError { path } => path.length;\n");
        source.push_str("        FormatError { line } => line;\n");
        source.push_str("    };\n");
        source.push_str("    return recovered;\n");
        source.push_str("}\n\n");
    }

    source
}
