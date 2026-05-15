use crate::tests::module::TestModule;
use crate::tests::snapshot::DirSnapshotSet;

#[test]
fn test_bind_generic_scopes() {
    let module = TestModule::parse(
        r#"
function wrap<T>(value: T): T {
    let value: T = value;
    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
type Hole = Box<infer _>;
type Anonymous<T> = T extends infer _ ? true : false;
"#,
    );
    let dir_bound = module.bind();

    module.assert_dir_bound_snapshot(
        &dir_bound,
        DirSnapshotSet::binding(),
        r#"
function wrap<T>(value: T): T {
/// @binding.symbol name=wrap role=item form=function scope=<module>@1
/// @binding.scope scope=wrap kind=function parent=<module>@2 owner=wrap
/// @binding.symbol name=T#1 role=local form=type_alias scope=wrap@0
/// @binding.symbol name=value#1 role=local form=variable scope=wrap@1
/// @binding.scope scope=scope3 kind=block parent=wrap@2

    let value: T = value;
    /// @binding.symbol name=value#2 role=local form=variable scope=scope3@0 mutability=mutable

    return value;
}

type Element<T> = T extends Array<infer U> ? U : T;
/// @binding.symbol name=Element role=item form=type_alias scope=<module>@2
/// @binding.scope scope=Element kind=type parent=<module>@3 owner=Element
/// @binding.symbol name=T#2 role=local form=type_alias scope=Element@0
/// @binding.scope scope=scope5 kind=type_conditional parent=Element@1
/// @binding.symbol name=U role=local form=type_alias scope=scope5@0

type Hole = Box<infer _>;
/// @binding.symbol name=Hole role=item form=type_alias scope=<module>@3
/// @binding.scope scope=Hole kind=type parent=<module>@4 owner=Hole

type Anonymous<T> = T extends infer _ ? true : false;
/// @binding.symbol name=Anonymous role=item form=type_alias scope=<module>@4
/// @binding.scope scope=Anonymous kind=type parent=<module>@5 owner=Anonymous
/// @binding.symbol name=T#3 role=local form=type_alias scope=Anonymous@0
/// @binding.scope scope=scope8 kind=type_conditional parent=Anonymous@1
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=11 scopes=9 declarations=10 node_scopes=38 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
