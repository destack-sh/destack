use crate::tests::module::TestModule;
use crate::tests::snapshot::DirSnapshotSet;

#[test]
fn test_bind_label_scopes() {
    let module = TestModule::parse(
        r#"
let done: boolean = false;

outer: for (let index = 0; index < 3; index = index + 1) {
    inner: {
        break outer;
        break inner;
    }
}
"#,
    );
    let dir_bound = module.bind();

    module.assert_dir_bound_snapshot(
        &dir_bound,
        DirSnapshotSet::binding(),
        r#"
let done: boolean = false;
/// @binding.symbol name=done role=local form=variable scope=<module>@1 mutability=mutable

outer: for (let index = 0; index < 3; index = index + 1) {
/// @binding.symbol name=outer role=local form=label scope=<module>@2
/// @binding.scope scope=outer kind=label parent=<module>@3 owner=outer
/// @binding.scope scope=scope3 kind=block parent=outer@0
/// @binding.symbol name=index role=local form=variable scope=scope3@0 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=scope3@1

    inner: {
    /// @binding.symbol name=inner role=local form=label scope=scope4@0
    /// @binding.scope scope=inner kind=label parent=scope4@1 owner=inner
    /// @binding.scope scope=scope6 kind=block parent=inner@0

        break outer;
        break inner;
    }
}
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=7 declarations=4 node_scopes=26 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
