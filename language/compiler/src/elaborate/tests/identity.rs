use tspp_mir::Function;

use crate::tests::TestProgram;

#[test]
fn test_generate_distinct_destructors_for_generic_instances() {
    let mut program = TestProgram::mir(
        r#"
type Box<T> {
    value: ref<T, unique, mutable>;
}

function test(
    v0: ref<int32, unique, mutable>,
    v1: ref<float64, unique, mutable>,
): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<float64, unique, mutable>):
    v2: Box<int32> = aggregate (v0)
    v3: Box<float64> = aggregate (v1)
    return
}
"#,
    );

    // elaborate both concrete type instances
    program.optimize();
    let destructors = program
        .lowered
        .tree
        .iter_nodes::<Function>()
        .filter_map(|(_, function)| {
            (program.strings.get(function.name) == "drop.frame")
                .then_some((&function.arguments, function.symbol))
        })
        .collect::<Vec<_>>();

    // preserve distinct textual and persistent destructor identities
    assert_eq!(destructors.len(), 2);
    assert_ne!(destructors[0].0, destructors[1].0);
    assert_ne!(destructors[0].1, destructors[1].1);
}
