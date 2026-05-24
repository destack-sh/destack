use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_loop_and_if_let_scopes() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
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
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_summaries(),
        r#"
for (let index: number = 0; index < 10; index = index + 1) {
/// @binding.scope scope=scope2 kind=block parent=<module>@1
/// @binding.symbol symbol=index#1 role=local form=variable scope=scope2@0 mutability=mutable
/// @binding.scope scope=scope3 kind=block parent=scope2@1

    let index: number = index;
    /// @binding.symbol symbol=index#2 role=local form=variable scope=scope3@0 mutability=mutable

}

let output: number = if (let Some(value) = maybe) {
/// @binding.symbol symbol=output role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=<module>@1
/// @binding.symbol symbol=value role=local form=variable scope=scope4@0
/// @binding.scope scope=scope5 kind=block parent=scope4@1

    value
} else {
/// @binding.scope scope=scope6 kind=block parent=<module>@1

    0
};
/// @binding.symbol symbol=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=7 declarations=4 node_scopes=37
"#,
    );
}

#[test]
fn test_bind_redeclaration_cursors() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let x: number = 1;
let x: number = x;
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_binding_nodes().with_summaries(),
        r#"
let x: number = 1;
/// @binding.node node=expression scope=<module>@1 source="let x: number = 1"
/// @binding.symbol symbol=x#1 role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.node node=declarator scope=<module>@1 source="x: number = 1"
/// @binding.node node=pattern scope=<module>@2 source=x
/// @binding.node node=type_expression scope=<module>@1 source=number
/// @binding.node node=expression scope=<module>@1 source=1

let x: number = x;
/// @binding.node node=expression scope=<module>@2 source="let x: number = x"
/// @binding.symbol symbol=x#2 role=local form=variable scope=<module>@2 mutability=mutable
/// @binding.node node=declarator scope=<module>@2 source="x: number = x"
/// @binding.node node=pattern scope=<module>@3 source=x
/// @binding.node node=type_expression scope=<module>@2 source=number
/// @binding.node node=expression scope=<module>@2 source=x
/// @binding.symbol symbol=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=3 scopes=2 declarations=2 node_scopes=10
"#,
    );
}

#[test]
fn test_bind_declaration_context_stays_on_declared_pattern() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export let result = try {
    fallback
} catch (error) {
    fallback
};
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding(),
        r#"
export let result = try {
/// @binding.symbol symbol=result role=local form=variable scope=<module>@1 mutability=mutable export=named
/// @binding.scope scope=scope2 kind=block parent=<module>@1

    fallback
} catch (error) {
/// @binding.scope scope=scope3 kind=block parent=<module>@1
/// @binding.symbol symbol=error role=local form=variable scope=scope3@0
/// @binding.scope scope=scope4 kind=block parent=scope3@1

    fallback
};
/// @binding.symbol symbol=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}
