use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_bind_generic_scopes() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
function wrap<T>(value: T): T {
    let value: T = value;
    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
type Hole = Box<infer _>;
type Anonymous<T> = T extends infer _ ? true : false;
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::binding()),
        r#"
function wrap<T>(value: T): T {
/// @binding.symbol key=wrap role=item form=function scope=<module>@1
/// @binding.scope scope=wrap kind=function parent=<module>@2 owner=wrap
/// @binding.symbol key=T#1 role=local form=type_alias scope=wrap@0
/// @binding.symbol key=value#1 role=local form=variable scope=wrap@1
/// @binding.scope scope=scope3 kind=block parent=wrap@2

    let value: T = value;
    /// @binding.symbol key=value#2 role=local form=variable scope=scope3@0 mutability=mutable

    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
/// @binding.symbol key=Element role=item form=type_alias scope=<module>@2
/// @binding.scope scope=Element kind=type parent=<module>@3 owner=Element
/// @binding.symbol key=T#2 role=local form=type_alias scope=Element@0
/// @binding.scope scope=scope5 kind=type_conditional parent=Element@1
/// @binding.symbol key=U role=local form=type_alias scope=scope5@0

type Hole = Box<infer _>;
/// @binding.symbol key=Hole role=item form=type_alias scope=<module>@3
/// @binding.scope scope=Hole kind=type parent=<module>@4 owner=Hole

type Anonymous<T> = T extends infer _ ? true : false;
/// @binding.symbol key=Anonymous role=item form=type_alias scope=<module>@4
/// @binding.scope scope=Anonymous kind=type parent=<module>@5 owner=Anonymous
/// @binding.symbol key=T#3 role=local form=type_alias scope=Anonymous@0
/// @binding.scope scope=scope8 kind=type_conditional parent=Anonymous@1
/// @binding.symbol key=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=11 scopes=9 declarations=10 node_scopes=38 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
