use crate::tests::TestSession;

#[test]
fn test_bind_counters_scale_with_visited_nodes() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|index| format!("let value{index}: number = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_bound_key("main.tspp"), "bind.");
    let expected = format!(
        "bind.files=1\n\
bind.roots={ITEMS}\n\
bind.expressions={}\n\
bind.declarations=0\n\
bind.patterns={ITEMS}\n\
bind.type_expressions={ITEMS}",
        ITEMS * 2
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_bind_counters_scale_with_large_function_body() {
    const ITEMS: usize = 50_000;

    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: number = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("function heavy() {{\n{body}\n}}");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_bound_key("main.tspp"), "bind.");
    let expected = format!(
        "bind.files=1\n\
bind.roots=1\n\
bind.expressions={}\n\
bind.declarations=1\n\
bind.patterns={ITEMS}\n\
bind.type_expressions={ITEMS}",
        ITEMS * 2 + 2
    );

    assert_eq!(counters, expected);
}
