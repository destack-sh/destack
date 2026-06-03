use super::TestSession;

/// Build one generated source tree fixture.
fn generated_source_tree(count: usize) -> Vec<(String, String)> {
    let mut files = vec![(
        "destack.json".to_string(),
        r#"{
  "name": "@test/app"
}
"#
        .to_string(),
    )];

    for index in 0..count {
        files.push((
            format!("src/generated/file-{index}.ds"),
            format!(
                r#"export const value{index} = {index};
"#
            ),
        ));
    }

    files
}

#[test]
fn test_open_tracks_large_source_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.assert_file_count(10_001);
}

#[test]
fn test_reload_reports_one_update_in_large_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.write(
        "src/generated/file-500.ds",
        r#"export const value500 = 999;
"#,
    );
    let updates = test.reload();

    test.assert_update_paths(&updates, &["src/generated/file-500.ds"]);
    test.assert_file_count(10_001);
}
