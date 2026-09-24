use super::counters::assert_check_counters;

const ITEMS: usize = 1_000;

/// Repeated identical generic calls each type their own site on a cold pass.
#[test]
fn test_repeated_generic_calls_type_each_site() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = pick(1, 2);"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "function pick<T>(a: T, b: T): T {{\n    a\n}}\n\nfunction heavy(): void {{\n{body}\n}}"
    );

    assert_check_counters(
        &source,
        r#"
check.solve.variables=3000
check.solve.constraints=1000
check.solve.obligations=1000
check.solve.solutions=3000
check.solve.decisions=2000
check.relations.decided=4
check.relations.reused=3996
check.bindings.built=0
check.bindings.reused=0
check.members.derived=0
check.members.refused=0
check.instantiations=1000
check.interns=18014
check.reduces=78026
"#,
    );
}

/// Distinct literal arguments instantiate one generic callee per site.
#[test]
fn test_distinct_literal_calls_instantiate_per_site() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = pick({index}, {index});"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "function pick<T>(a: T, b: T): T {{\n    a\n}}\n\nfunction heavy(): void {{\n{body}\n}}"
    );

    assert_check_counters(
        &source,
        r#"
check.solve.variables=3000
check.solve.constraints=1000
check.solve.obligations=1000
check.solve.solutions=3000
check.solve.decisions=2000
check.relations.decided=2000
check.relations.reused=2000
check.bindings.built=0
check.bindings.reused=0
check.members.derived=0
check.members.refused=0
check.instantiations=1000
check.interns=18014
check.reduces=88006
"#,
    );
}
