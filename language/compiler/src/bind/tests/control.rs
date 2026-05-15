use crate::tests::module::TestModule;
use crate::tests::snapshot::DirSnapshotSet;

#[test]
fn test_bind_loop_and_if_let_scopes() {
    let module = TestModule::parse(
        r#"
for (let index: number = 0; index < 10; index = index + 1) {
    let index: number = index;
}

let output: number = if (let Some(value) = maybe) {
    value
} else {
    0
};
"#,
    );
    let dir_bound = module.bind();

    module.assert_dir_bound_snapshot(
        &dir_bound,
        DirSnapshotSet::binding(),
        r#"
for (let index: number = 0; index < 10; index = index + 1) {
/// @binding.scope scope=scope2 kind=block parent=<module>@1
/// @binding.symbol name=index#1 role=local form=variable scope=scope2@0 mutability=mutable
/// @binding.scope scope=scope3 kind=block parent=scope2@1

    let index: number = index;
    /// @binding.symbol name=index#2 role=local form=variable scope=scope3@0 mutability=mutable

}

let output: number = if (let Some(value) = maybe) {
/// @binding.symbol name=output role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=<module>@1
/// @binding.symbol name=value role=local form=variable scope=scope4@0
/// @binding.scope scope=scope5 kind=block parent=scope4@1

    value
} else {
/// @binding.scope scope=scope6 kind=block parent=<module>@1

    0
};
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=7 declarations=4 node_scopes=37 replaced_symbols=0 replaced_scopes=0
"#,
    );
}

#[test]
fn test_bind_redeclaration_cursors() {
    let module = TestModule::parse(
        r#"
let x: number = 1;
let x: number = x;
"#,
    );
    let dir_bound = module.bind();

    module.assert_dir_bound_snapshot(
        &dir_bound,
        DirSnapshotSet::binding().with_binding_nodes(),
        r#"
let x: number = 1;
/// @binding.node node=expression scope=<module>@1 source="let x: number = 1"
/// @binding.symbol name=x#1 role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.node node=declarator scope=<module>@1 source="x: number = 1"
/// @binding.node node=pattern scope=<module>@2 source=x
/// @binding.node node=type_expression scope=<module>@1 source=number
/// @binding.node node=expression scope=<module>@1 source=1

let x: number = x;
/// @binding.node node=expression scope=<module>@2 source="let x: number = x"
/// @binding.symbol name=x#2 role=local form=variable scope=<module>@2 mutability=mutable
/// @binding.node node=declarator scope=<module>@2 source="x: number = x"
/// @binding.node node=pattern scope=<module>@3 source=x
/// @binding.node node=type_expression scope=<module>@2 source=number
/// @binding.node node=expression scope=<module>@2 source=x
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=3 scopes=2 declarations=2 node_scopes=10 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
