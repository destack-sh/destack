use super::counters::assert_check_counters;

const ITEMS: usize = 1_000;

/// Repeated extension method calls derive one member lookup per site.
#[test]
fn test_repeated_extension_method_calls_derive_a_lookup_per_site() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = 0.double();"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        r#"extension of int32 {{
    double(this): int32 {{
        this + this
    }}
}}

function heavy(): void {{
{body}
}}"#,
    );

    assert_check_counters(
        &source,
        r#"
check.solve.variables=0
check.solve.constraints=0
check.solve.obligations=1000
check.solve.solutions=0
check.solve.decisions=3003
check.relations.decided=1
check.relations.reused=2000
check.bindings.built=1
check.bindings.reused=999
check.members.derived=1000
check.members.refused=0
check.instantiations=1000
check.interns=12028
check.reduces=25034
"#,
    );
}
