use crate::tests::harness::TestWorkspace;

/// Formats authored source without requiring a semantic query target.
#[test]
fn test_format_file_without_query_target() {
    let test = TestWorkspace::new("format-file-without-query-target");
    let config_source = "{}\n";
    let config = test.write_text("destack.json", config_source);
    test.apply_text(&config, config_source);
    let source = "export const value=1;\n";
    let path = test.write_text("main.ds", source);
    test.apply_text(&path, source);

    // format the exact workspace file without semantic module resolution
    let edit = test
        .workspace
        .format_file(&test.roots[0], path, None)
        .expect("format file")
        .expect("format edit");

    assert_eq!(edit.patch.span.file, edit.file.id);
    assert_eq!(edit.patch.span.start, 0);
    assert_eq!(edit.patch.span.end, edit.file.len);
    assert_eq!(edit.patch.new_text, "export const value = 1;\n");
}
