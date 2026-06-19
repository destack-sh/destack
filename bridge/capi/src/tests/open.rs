use super::session::TestSession;

#[test]
fn test_open_memory_source() {
    let session = TestSession::open(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ]);

    // list seeded source files
    assert_eq!(session.paths(), ["destack.json", "src/index.ds"]);
}
