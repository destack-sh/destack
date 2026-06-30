use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_generic_scopes() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
function wrap<T>(value: T): T {
    let value: T = value;
    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
type Return<T> = T extends () => infer R ? R : never;
type Hole = Box<infer _>;
type Anonymous<T> = T extends infer _ ? true : false;
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_summaries().with_bind_stats(),
        r#"
function wrap<T>(value: T): T {
/// @binding.symbol symbol=wrap role=item kind=function scope=<module>@1
/// @binding.scope scope=wrap kind=function parent=<module>@2 owner=wrap
/// @binding.owner_scope owner=wrap scope=wrap
/// @binding.symbol symbol=T#1 role=local kind=generic_type_parameter scope=wrap@0
/// @binding.symbol symbol=value#1 role=local kind=variable scope=wrap@1
/// @binding.scope scope=scope3 kind=block parent=wrap@2

    let value: T = value;
    /// @binding.symbol symbol=value#2 role=local kind=variable scope=scope3@0 mutability=mutable

    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
/// @binding.symbol symbol=Element role=item kind=type_alias scope=<module>@2
/// @binding.scope scope=Element kind=type parent=<module>@3 owner=Element
/// @binding.owner_scope owner=Element scope=Element
/// @binding.symbol symbol=T#2 role=local kind=generic_type_parameter scope=Element@0
/// @binding.scope scope=scope5 kind=type_conditional parent=Element@1
/// @binding.symbol symbol=U role=local kind=type_alias scope=scope5@0

type Return<T> = T extends () => infer R ? R : never;
/// @binding.symbol symbol=Return role=item kind=type_alias scope=<module>@3
/// @binding.scope scope=Return kind=type parent=<module>@4 owner=Return
/// @binding.owner_scope owner=Return scope=Return
/// @binding.symbol symbol=T#3 role=local kind=generic_type_parameter scope=Return@0
/// @binding.scope scope=scope7 kind=type_conditional parent=Return@1
/// @binding.scope scope=scope8 kind=type parent=scope7@0
/// @binding.symbol symbol=R role=local kind=type_alias scope=scope7@0

type Hole = Box<infer _>;
/// @binding.symbol symbol=Hole role=item kind=type_alias scope=<module>@4
/// @binding.scope scope=Hole kind=type parent=<module>@5 owner=Hole
/// @binding.owner_scope owner=Hole scope=Hole

type Anonymous<T> = T extends infer _ ? true : false;
/// @binding.symbol symbol=Anonymous role=item kind=type_alias scope=<module>@5
/// @binding.scope scope=Anonymous kind=type parent=<module>@6 owner=Anonymous
/// @binding.owner_scope owner=Anonymous scope=Anonymous
/// @binding.symbol symbol=T#4 role=local kind=generic_type_parameter scope=Anonymous@0
/// @binding.scope scope=scope11 kind=type_conditional parent=Anonymous@1

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=14 scopes=12 declarations=13 node_scopes=47 owner_scopes=5
/// @bind.stats files=1 roots=5
"#,
    );
}
