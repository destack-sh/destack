use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_foreach_declaration_mutability() {
    let compiler = TestSession::single(
        r#"
declare const values: number[];

for (let mutable of values) {}
for (const immutable of values) {}
for (using resource of values) {}
"#,
    );

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding(),
        r#"
declare const values: number[];
/// @binding.symbol symbol=values role=local kind=variable scope=<module>@1 mutability=immutable

for (let mutable of values) {}
/// @binding.scope scope=scope2 kind=block parent=<module>@2
/// @binding.symbol symbol=mutable role=local kind=variable scope=scope2@0 mutability=mutable
/// @binding.scope scope=scope3 kind=block parent=scope2@1

for (const immutable of values) {}
/// @binding.scope scope=scope4 kind=block parent=<module>@2
/// @binding.symbol symbol=immutable role=local kind=variable scope=scope4@0 mutability=immutable
/// @binding.scope scope=scope5 kind=block parent=scope4@1

for (using resource of values) {}
/// @binding.scope scope=scope6 kind=block parent=<module>@2
/// @binding.symbol symbol=resource role=local kind=variable scope=scope6@0 mutability=immutable
/// @binding.scope scope=scope7 kind=block parent=scope6@1

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}

#[test]
fn test_bind_match_arm_pattern_in_guard_and_body() {
    let compiler = TestSession::single(
        r#"
declare const packet: { value: int32 };

const result = match (packet) {
    { value } if (value > 0) => value
    _ => 0
};
"#,
    );

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding().with_binding_nodes(),
        r#"
declare const packet: { value: int32 };
/// @binding.node node=expression scope=<module>@1 source="declare const packet: { value: int32 }"
/// @binding.symbol symbol=packet role=local kind=variable scope=<module>@3 mutability=immutable
/// @binding.node node=declarator scope=<module>@1 source="packet: { value: int32 }"
/// @binding.node node=pattern scope=<module>@4 source=packet
/// @binding.node node=type_expression scope=<module>@1 source={ value: int32 }
/// @binding.symbol symbol=value#1 role=item kind=variable scope=<module>@1 visibility=member
/// @binding.node node=type_member scope=<module>@1 source="value: int32"
/// @binding.receiver node=type_member symbol=symbol2
/// @binding.node node=type_expression scope=<module>@3 source=int32

const result = match (packet) {
/// @binding.node node=expression scope=<module>@4
/// @binding.symbol symbol=result role=local kind=variable scope=<module>@4 mutability=immutable
/// @binding.node node=declarator scope=<module>@4
/// @binding.node node=pattern scope=<module>@5 source=result
/// @binding.node node=expression scope=<module>@4
/// @binding.node node=expression scope=<module>@4 source=packet

    { value } if (value > 0) => value
    /// @binding.scope scope=scope2 kind=block parent=<module>@4
    /// @binding.node node=match_arm scope=scope2@0 source="{ value } if (value > 0) => value"
    /// @binding.node node=pattern scope=scope2@0 source={ value }
    /// @binding.symbol symbol=value#2 role=local kind=variable scope=scope2@0 mutability=immutable
    /// @binding.node node=pattern_field scope=scope2@1 source=value
    /// @binding.node node=expression scope=scope2@1 source="value > 0"
    /// @binding.node node=expression scope=scope2@1 source=value
    /// @binding.node node=expression scope=scope2@1 source=0
    /// @binding.node node=expression scope=scope2@1 source=value

    _ => 0
    /// @binding.scope scope=scope3 kind=block parent=<module>@4
    /// @binding.node node=match_arm scope=scope3@0 source="_ => 0"
    /// @binding.node node=pattern scope=scope3@0 source=_
    /// @binding.node node=expression scope=scope3@0 source=0

};

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.symbol symbol=symbol2 role=local kind=variable scope=<module>@2
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}

#[test]
fn test_bind_switch_selector_outside_case_body() {
    let compiler = TestSession::single(
        r#"
declare const selected: int32;

switch (selected) {
    case selected:
        let value = selected;
        break;
    default:
        break;
}
"#,
    );

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding().with_binding_nodes(),
        r#"
declare const selected: int32;
/// @binding.node node=expression scope=<module>@1 source="declare const selected: int32"
/// @binding.symbol symbol=selected role=local kind=variable scope=<module>@1 mutability=immutable
/// @binding.node node=declarator scope=<module>@1 source="selected: int32"
/// @binding.node node=pattern scope=<module>@2 source=selected
/// @binding.node node=type_expression scope=<module>@1 source=int32

switch (selected) {
/// @binding.node node=expression scope=<module>@2
/// @binding.node node=expression scope=<module>@2 source=selected

    case selected:
    /// @binding.node node=switch_case scope=<module>@2
    /// @binding.node node=expression scope=<module>@2 source=selected

        let value = selected;
        /// @binding.scope scope=scope2 kind=block parent=<module>@2
        /// @binding.node node=block scope=scope2@0
        /// @binding.node node=expression scope=scope2@0 source="let value = selected"
        /// @binding.symbol symbol=value role=local kind=variable scope=scope2@0 mutability=mutable
        /// @binding.node node=declarator scope=scope2@0 source="value = selected"
        /// @binding.node node=pattern scope=scope2@1 source=value
        /// @binding.node node=expression scope=scope2@0 source=selected

        break;
        /// @binding.node node=expression scope=scope2@1 source=break

    default:
    /// @binding.node node=switch_case scope=<module>@2

        break;
        /// @binding.scope scope=scope3 kind=block parent=<module>@2
        /// @binding.node node=block scope=scope3@0 source=break;
        /// @binding.node node=expression scope=scope3@0 source=break

}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}

#[test]
fn test_bind_loop_and_if_let_scopes() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
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
        "main.tspp",
        DirRows::binding().with_summaries(),
        r#"
for (let index: number = 0; index < 10; index = index + 1) {
/// @binding.scope scope=scope2 kind=block parent=<module>@1
/// @binding.symbol symbol=index#1 role=local kind=variable scope=scope2@0 mutability=mutable
/// @binding.scope scope=scope3 kind=block parent=scope2@1

    let index: number = index;
    /// @binding.symbol symbol=index#2 role=local kind=variable scope=scope3@0 mutability=mutable

}

let output: number = if (let Some(value) = maybe) {
/// @binding.symbol symbol=output role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=<module>@1
/// @binding.symbol symbol=value role=local kind=variable scope=scope4@0 mutability=mutable
/// @binding.scope scope=scope5 kind=block parent=scope4@1

    value
} else {
/// @binding.scope scope=scope6 kind=block parent=<module>@1

    0
};

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=5 scopes=7 declarations=4 node_scopes=38
"#,
    );
}

#[test]
fn test_bind_redeclaration_cursors() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
let x: number = 1;
let x: number = x;
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding().with_binding_nodes().with_summaries(),
        r#"
let x: number = 1;
/// @binding.node node=expression scope=<module>@1 source="let x: number = 1"
/// @binding.symbol symbol=x#1 role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.node node=declarator scope=<module>@1 source="x: number = 1"
/// @binding.node node=pattern scope=<module>@2 source=x
/// @binding.node node=type_expression scope=<module>@1 source=number
/// @binding.node node=expression scope=<module>@1 source=1

let x: number = x;
/// @binding.node node=expression scope=<module>@2 source="let x: number = x"
/// @binding.symbol symbol=x#2 role=local kind=variable scope=<module>@2 mutability=mutable
/// @binding.node node=declarator scope=<module>@2 source="x: number = x"
/// @binding.node node=pattern scope=<module>@3 source=x
/// @binding.node node=type_expression scope=<module>@2 source=number
/// @binding.node node=expression scope=<module>@2 source=x

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=3 scopes=2 declarations=2 node_scopes=10
"#,
    );
}

#[test]
fn test_bind_declaration_context_stays_on_declared_pattern() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
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
        "main.tspp",
        DirRows::binding(),
        r#"
export let result = try {
/// @binding.symbol symbol=result role=local kind=variable scope=<module>@1 mutability=mutable export=named
/// @binding.scope scope=scope2 kind=block parent=<module>@1

    fallback
} catch (error) {
/// @binding.scope scope=scope3 kind=block parent=<module>@1
/// @binding.symbol symbol=error role=local kind=variable scope=scope3@0 mutability=mutable
/// @binding.scope scope=scope4 kind=block parent=scope3@1

    fallback
};

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}
