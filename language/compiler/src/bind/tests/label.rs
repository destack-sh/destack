use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_label_scopes() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let done: boolean = false;

outer: for (let index = 0; index < 3; index = index + 1) {
    inner: {
        break outer;
        break inner;
    }
}
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_summaries(),
        r#"
let done: boolean = false;
/// @binding.symbol symbol=done role=local form=variable scope=<module>@1 mutability=mutable

outer: for (let index = 0; index < 3; index = index + 1) {
/// @binding.symbol symbol=outer role=local form=label scope=<module>@2
/// @binding.scope scope=outer kind=label parent=<module>@3 owner=outer
/// @binding.scope scope=scope3 kind=block parent=outer@0
/// @binding.symbol symbol=index role=local form=variable scope=scope3@0 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=scope3@1

    inner: {
    /// @binding.symbol symbol=inner role=local form=label scope=scope4@0
    /// @binding.scope scope=inner kind=label parent=scope4@1 owner=inner
    /// @binding.scope scope=scope6 kind=block parent=inner@0

        break outer;
        break inner;
    }
}
/// @binding.symbol symbol=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=7 declarations=4 node_scopes=26
"#,
    );
}
