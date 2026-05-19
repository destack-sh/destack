use std::fmt::Write;

use super::StressMode;

/// Generate explicit resource management surfaces.
pub(super) fn using_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 520);
    source.push_str("declare function openFile(path: string): Result<File, IOError>;\n");
    source.push_str("declare const pool: ConnectionPool;\n");
    source.push_str("declare const files: File[];\n\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "function copyCase{index}(inputPath: string, outputPath: string): Result<void, IOError> {{"
        );
        source.push_str("    using input = openFile(inputPath)?,\n");
        source.push_str("          output = openFile(outputPath)?;\n");
        source.push_str("    copy(input, output);\n");
        source.push_str("}\n\n");

        let _ = writeln!(
            source,
            "async function queryCase{index}(sql: string): Promise<Result<Row[], DatabaseError>> {{"
        );
        source.push_str("    await using connection = await pool.connect();\n");
        source.push_str("    return await connection.query(sql);\n");
        source.push_str("}\n\n");

        let _ = writeln!(source, "function loopCase{index}(): void {{");
        source.push_str("    for (using file of files) {\n");
        source.push_str("        process(file);\n");
        source.push_str("    }\n");
        source.push_str("}\n\n");
    }

    source
}
