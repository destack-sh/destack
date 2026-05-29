use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_module_scope_surface() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { dep as local, type TypeDep } from "dep";

global {
    let process: Process;
}

let x: number = 1;
let x: number = x + 1;

function wrap<T extends Box<_>, U = T>(value: T): U
where U: Clone {
    let value: T = value;
    try {
        value
    } catch (error) {
        value
    }
}

type Pick<T> = {
    [K in keyof T as `get${K}`]: T[K];
};
"#,
        )
        .build();

    compiler.assert_dir_bound("main.ds", DirRows::binding().with_summaries(), r#"
import { dep as local, type TypeDep } from "dep";
/// @binding.symbol symbol=local role=local kind=import scope=<module>@1
/// @binding.symbol symbol=TypeDep role=local kind=import scope=<module>@2

global {
/// @binding.scope scope=scope1 kind=global parent=<module>@3

    let process: Process;
    /// @binding.symbol symbol=process role=local kind=variable scope=scope1@0 mutability=mutable origin=global

}

let x: number = 1;
/// @binding.symbol symbol=x#1 role=local kind=variable scope=<module>@3 mutability=mutable

let x: number = x + 1;
/// @binding.symbol symbol=x#2 role=local kind=variable scope=<module>@4 mutability=mutable

function wrap<T extends Box<_>, U = T>(value: T): U
/// @binding.symbol symbol=wrap role=item kind=function scope=<module>@5
/// @binding.scope scope=wrap kind=function parent=<module>@6 owner=wrap
/// @binding.owner_scope owner=wrap scope=wrap
/// @binding.symbol symbol=T#1 role=local kind=generic_type_parameter scope=wrap@0
/// @binding.symbol symbol=U role=local kind=generic_type_parameter scope=wrap@1
/// @binding.symbol symbol=value#1 role=local kind=variable scope=wrap@2

where U: Clone {
/// @binding.scope scope=scope3 kind=block parent=wrap@3

    let value: T = value;
    /// @binding.symbol symbol=value#2 role=local kind=variable scope=scope3@0 mutability=mutable

    try {
    /// @binding.scope scope=scope4 kind=block parent=scope3@1

        value
    } catch (error) {
    /// @binding.scope scope=scope5 kind=block parent=scope3@1
    /// @binding.symbol symbol=error role=local kind=variable scope=scope5@0 mutability=mutable
    /// @binding.scope scope=scope6 kind=block parent=scope5@1

        value
    }
}

type Pick<T> = {
/// @binding.symbol symbol=Pick role=item kind=type_alias scope=<module>@6
/// @binding.scope scope=Pick kind=type parent=<module>@7 owner=Pick
/// @binding.owner_scope owner=Pick scope=Pick
/// @binding.symbol symbol=T#2 role=local kind=generic_type_parameter scope=Pick@0
/// @binding.scope scope=scope8 kind=type parent=Pick@1

    [K in keyof T as `get${K}`]: T[K];
    /// @binding.symbol symbol=K role=local kind=type_alias scope=scope8@0

};
/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>

/// @binding.summary symbols=15 scopes=9 declarations=14 node_scopes=63 owner_scopes=2
"#,
    );
}

#[test]
fn test_bind_module_directive_namespace() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
module {
    let renderer: Renderer = createRenderer();
}

let renderer: string = "local";
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.ds",
        DirRows::binding().with_summaries(),
        r#"
module {
/// @binding.scope scope=scope2 kind=namespace parent=<module>@1

    let renderer: Renderer = createRenderer();
    /// @binding.symbol symbol=renderer#1 role=local kind=variable scope=scope2@0 mutability=mutable

}

let renderer: string = "local";
/// @binding.symbol symbol=renderer#2 role=local kind=variable scope=<module>@1 mutability=mutable
/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=3 scopes=3 declarations=2 node_scopes=13
"#,
    );
}
