use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match type and value arguments of an indexed constructor.
#[test]
fn test_match_indexed_constructor() {
    TestMatcher::context(
        "new constructors[0]<$TYPE>($VALUE)",
        dir::NodeType::Expression,
        r#"
new constructors[0]<Item>(value)
new constructors[1]<Item>(value)
"#,
    )
    .assert(
        r#"
new constructors[0]<Item>(value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match TYPE.node="Item" VALUE.node="value"
new constructors[1]<Item>(value)
"#,
    );
}

/// Match a direct generic call argument.
#[test]
fn test_match_generic_argument() {
    TestMatcher::new(
        "foo<$ARGUMENT>()",
        r#"
foo<User>()
foo<Result>()
bar<User>()
"#,
    )
    .assert(
        r#"
foo<User>()
^^^^^^^^^^^ match ARGUMENT.node="User"
foo<Result>()
^^^^^^^^^^^^^ match ARGUMENT.node="Result"
bar<User>()
"#,
    );
}

/// Match a generic parameter selected from a declaration context.
#[test]
fn test_match_generic_parameter() {
    TestMatcher::context(
        "function f<$NAME: $CONSTRAINT = $DEFAULT>() {}",
        dir::NodeType::GenericParameter,
        r#"
function first<Value: Serializable = DefaultValue>() {}
function second<Item: Entity = DefaultItem>() {}
"#,
    )
    .assert(
        r#"
function first<Value: Serializable = DefaultValue>() {}
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match NAME.name="Value" CONSTRAINT.node="Serializable" DEFAULT.node="DefaultValue"
function second<Item: Entity = DefaultItem>() {}
                ^^^^^^^^^^^^^^^^^^^^^^^^^^ match NAME.name="Item" CONSTRAINT.node="Entity" DEFAULT.node="DefaultItem"
"#,
    );
}
