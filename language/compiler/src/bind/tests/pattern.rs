use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_destructuring_patterns() {
    let compiler = TestSession::new()
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

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_summaries(),
        r#"
let { id, name: displayName }: User = user;
/// @binding.symbol symbol=id#1 role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.symbol symbol=displayName role=local kind=variable scope=<module>@2 mutability=mutable

let [first, , ...rest]: Items = items;
/// @binding.symbol symbol=first#1 role=local kind=variable scope=<module>@3 mutability=mutable
/// @binding.symbol symbol=rest role=local kind=variable scope=<module>@4 mutability=mutable

function visit({ id }: User, [first]: Items) {
/// @binding.symbol symbol=visit role=item kind=function scope=<module>@5
/// @binding.scope scope=visit kind=function parent=<module>@6 owner=visit
/// @binding.symbol symbol=id#2 role=local kind=variable scope=visit@0
/// @binding.symbol symbol=first#2 role=local kind=variable scope=visit@1
/// @binding.scope scope=scope3 kind=block parent=visit@2

    id;
    first;
}
/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=8 scopes=4 declarations=7 node_scopes=31
"#,
    );
}
