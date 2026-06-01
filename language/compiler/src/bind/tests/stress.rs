use std::collections::BTreeMap;

use crate::tests::TestSession;

#[test]
fn test_bind_stats_scale_with_visited_nodes() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|index| format!("let value{index}: number = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let compiler = TestSession::single(&source);
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_bound_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "bind".to_string())]),
    );
    let expected = format!(
        "bind.stats.files=1\n\
bind.stats.roots={ITEMS}\n\
bind.stats.visited.expressions={}\n\
bind.stats.visited.declarations=0\n\
bind.stats.visited.patterns={ITEMS}\n\
bind.stats.visited.types={ITEMS}",
        ITEMS * 2
    );

    assert_eq!(metadata, expected);
}
