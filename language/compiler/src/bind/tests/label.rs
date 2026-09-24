use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_label_scopes() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let done: boolean = false;

outer: for (let index = 0; index < 3; index = index + 1) {
    inner: while (true) {
        break outer;
        continue inner;
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
/// @binding.symbol symbol=done role=local kind=variable scope=<module>@1 mutability=mutable

outer: for (let index = 0; index < 3; index = index + 1) {
/// @binding.symbol symbol=outer role=local kind=label scope=<module>@2 visibility=hidden
/// @binding.scope scope=scope2 kind=block parent=<module>@2
/// @binding.symbol symbol=index role=local kind=variable scope=scope2@0 mutability=mutable
/// @binding.scope scope=scope3 kind=block parent=scope2@1

    inner: while (true) {
    /// @binding.symbol symbol=inner role=local kind=label scope=scope3@0 visibility=hidden
    /// @binding.scope scope=scope4 kind=block parent=scope3@0

        break outer;
        continue inner;
    }
}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=5 declarations=4 node_scopes=25
"#,
    );
}
