use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_bind_destructuring_patterns() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
let { id, name: displayName }: User = user;
let [first, , ...rest]: Items = items;

function visit({ id }: User, [first]: Items) {
    id;
    first;
}
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::binding()),
        r#"
let { id, name: displayName }: User = user;
/// @binding.symbol key=id#1 role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.symbol key=displayName role=local form=variable scope=<module>@2 mutability=mutable

let [first, , ...rest]: Items = items;
/// @binding.symbol key=first#1 role=local form=variable scope=<module>@3 mutability=mutable
/// @binding.symbol key=rest role=local form=variable scope=<module>@4 mutability=mutable

function visit({ id }: User, [first]: Items) {
/// @binding.symbol key=visit role=item form=function scope=<module>@5
/// @binding.scope scope=visit kind=function parent=<module>@6 owner=visit
/// @binding.symbol key=id#2 role=local form=variable scope=visit@0
/// @binding.symbol key=first#2 role=local form=variable scope=visit@1
/// @binding.scope scope=scope3 kind=block parent=visit@2

    id;
    first;
}
/// @binding.symbol key=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=8 scopes=4 declarations=7 node_scopes=31 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
