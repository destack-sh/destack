use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_pattern_defaults_declare_nested_bindings() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let { name = "Ada" } = {};
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_binding_nodes().with_summaries(),
        r#"
let { name = "Ada" } = {};
/// @binding.node node=expression scope=<module>@1 source="let { name = \"Ada\" } = {}"
/// @binding.node node=declarator scope=<module>@1 source={ name = "Ada" } = {}
/// @binding.node node=pattern scope=<module>@1 source={ name = "Ada" }
/// @binding.symbol symbol=name role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.node node=pattern scope=<module>@1 source=name
/// @binding.node node=pattern scope=<module>@2 source=name
/// @binding.node node=pattern_field scope=<module>@1 source="name = \"Ada\""
/// @binding.node node=expression scope=<module>@2 source="\"Ada\""
/// @binding.node node=expression scope=<module>@1 source={}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=2 scopes=2 declarations=1 node_scopes=8
"#,
    );
}

#[test]
fn test_bind_assignment_patterns_do_not_declare_storage() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding()
            .with_binding_nodes()
            .with_summaries()
            .with_bind_stats(),
        r#"
let x: int32 = 0;
/// @binding.node node=expression scope=<module>@1 source="let x: int32 = 0"
/// @binding.symbol symbol=x#1 role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.node node=declarator scope=<module>@1 source="x: int32 = 0"
/// @binding.node node=pattern scope=<module>@2 source=x
/// @binding.node node=type_expression scope=<module>@1 source=int32
/// @binding.node node=expression scope=<module>@1 source=0

let label: string = "";
/// @binding.node node=expression scope=<module>@2 source="let label: string = \"\""
/// @binding.symbol symbol=label role=local kind=variable scope=<module>@2 mutability=mutable
/// @binding.node node=declarator scope=<module>@2 source="label: string = \"\""
/// @binding.node node=pattern scope=<module>@3 source=label
/// @binding.node node=type_expression scope=<module>@2 source=string
/// @binding.node node=expression scope=<module>@2 source="\"\""

declare const point: { x: int32; y: string };
/// @binding.node node=expression scope=<module>@3 source="declare const point: { x: int32; y: string }"
/// @binding.symbol symbol=point role=local kind=variable scope=<module>@7 mutability=immutable
/// @binding.node node=declarator scope=<module>@3 source="point: { x: int32; y: string }"
/// @binding.node node=pattern scope=<module>@8 source=point
/// @binding.node node=type_expression scope=<module>@3 source={ x: int32; y: string }
/// @binding.symbol symbol=x#2 role=item kind=variable scope=<module>@3 visibility=member
/// @binding.node node=type_member scope=<module>@3 source="x: int32"
/// @binding.receiver node=type_member symbol=symbol4
/// @binding.node node=type_expression scope=<module>@5 source=int32
/// @binding.symbol symbol=y role=item kind=variable scope=<module>@5 visibility=member
/// @binding.node node=type_member scope=<module>@5 source="y: string"
/// @binding.receiver node=type_member symbol=symbol6
/// @binding.node node=type_expression scope=<module>@7 source=string

({ x, y: label } = point);
/// @binding.node node=assign_pattern scope=<module>@8 source={ x, y: label }
/// @binding.node node=expression scope=<module>@8 source="{ x, y: label } = point"
/// @binding.node node=assign_pattern scope=<module>@8 source=x
/// @binding.node node=assign_pattern_field scope=<module>@8 source=x
/// @binding.node node=expression scope=<module>@8 source=x
/// @binding.node node=assign_pattern_field scope=<module>@8 source="y: label"
/// @binding.node node=assign_pattern scope=<module>@8 source=label
/// @binding.node node=expression scope=<module>@8 source=label
/// @binding.node node=expression scope=<module>@8 source=point

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.symbol symbol=symbol4 role=local kind=variable scope=<module>@4
/// @binding.symbol symbol=symbol6 role=local kind=variable scope=<module>@6
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=8 scopes=2 declarations=5 receivers=2 node_scopes=27
/// @bind.stats files=1 roots=4
"#,
    );
}

#[test]
fn test_bind_destructuring_patterns() {
    let compiler = TestSession::builder()
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
        DirRows::binding().with_summaries().with_bind_stats(),
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
/// @binding.owner_scope owner=visit scope=visit
/// @binding.symbol symbol=id#2 role=local kind=variable scope=visit@0
/// @binding.symbol symbol=first#2 role=local kind=variable scope=visit@1
/// @binding.scope scope=scope3 kind=block parent=visit@2

    id;
    first;
}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=8 scopes=4 declarations=7 node_scopes=33 owner_scopes=1
/// @bind.stats files=1 roots=3
"#,
    );
}
