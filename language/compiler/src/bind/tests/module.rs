use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_bind_module_scope_surface() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::binding()),
        r#"
import { dep as local, type TypeDep } from "dep";
/// @binding.symbol name=local role=local form=import scope=<module>@1
/// @binding.symbol name=TypeDep role=local form=import scope=<module>@2

global {
/// @binding.scope scope=scope1 kind=global parent=<module>@3

    let process: Process;
    /// @binding.symbol name=process role=local form=variable scope=scope1@0 mutability=mutable origin=global

}

let x: number = 1;
/// @binding.symbol name=x#1 role=local form=variable scope=<module>@3 mutability=mutable

let x: number = x + 1;
/// @binding.symbol name=x#2 role=local form=variable scope=<module>@4 mutability=mutable

function wrap<T extends Box<_>, U = T>(value: T): U
/// @binding.symbol name=wrap role=item form=function scope=<module>@5
/// @binding.scope scope=wrap kind=function parent=<module>@6 owner=wrap
/// @binding.symbol name=T#1 role=local form=type_alias scope=wrap@0
/// @binding.symbol name=U role=local form=type_alias scope=wrap@1
/// @binding.symbol name=value#1 role=local form=variable scope=wrap@2

where U: Clone {
/// @binding.scope scope=scope3 kind=block parent=wrap@3

    let value: T = value;
    /// @binding.symbol name=value#2 role=local form=variable scope=scope3@0 mutability=mutable

    try {
    /// @binding.scope scope=scope4 kind=block parent=scope3@1

        value
    } catch (error) {
    /// @binding.scope scope=scope5 kind=block parent=scope3@1
    /// @binding.symbol name=error role=local form=variable scope=scope5@0
    /// @binding.scope scope=scope6 kind=block parent=scope5@1

        value
    }
}

type Pick<T> = {
/// @binding.symbol name=Pick role=item form=type_alias scope=<module>@6
/// @binding.scope scope=Pick kind=type parent=<module>@7 owner=Pick
/// @binding.symbol name=T#2 role=local form=type_alias scope=Pick@0
/// @binding.scope scope=scope8 kind=type parent=Pick@1

    [K in keyof T as `get${K}`]: T[K];
    /// @binding.symbol name=K role=local form=type_alias scope=scope8@0

};
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>

/// @binding.summary symbols=15 scopes=9 declarations=14 node_scopes=62 replaced_symbols=0 replaced_scopes=0
"#,
    );
}

#[test]
fn test_bind_module_directive_namespace() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::binding()),
        r#"
module {
/// @binding.scope scope=scope2 kind=namespace parent=<module>@1

    let renderer: Renderer = createRenderer();
    /// @binding.symbol name=renderer#1 role=local form=variable scope=scope2@0 mutability=mutable

}

let renderer: string = "local";
/// @binding.symbol name=renderer#2 role=local form=variable scope=<module>@1 mutability=mutable
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=3 scopes=3 declarations=2 node_scopes=13 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
