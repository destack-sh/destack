use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_bind_member_scopes() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
struct User<T> {
    id: string;
    type Id = string;
    static defaultName: string = "guest";

    rename(name: string): User<T> {
        this;
    }
}

interface Reader<T> {
    read(value: T): Result<T>;
    type Item = T;
}
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::binding()),
        r#"
struct User<T> {
/// @binding.symbol name=User role=namespace form=struct scope=<module>@1
/// @binding.scope scope=User kind=namespace parent=<module>@2 owner=User
/// @binding.symbol name=T#1 role=local form=type_alias scope=User@0

    id: string;
    /// @binding.symbol name=id role=item form=variable scope=User@1

    type Id = string;
    /// @binding.symbol name=Id role=item form=type_alias scope=User@2
    /// @binding.scope scope=Id kind=type parent=User@3 owner=Id

    static defaultName: string = "guest";
    /// @binding.symbol name=defaultName role=item form=variable scope=User@3

    rename(name: string): User<T> {
    /// @binding.symbol name=rename role=item form=function scope=User@4
    /// @binding.scope scope=rename kind=function parent=User@5 owner=rename
    /// @binding.symbol name=name role=local form=variable scope=rename@0
    /// @binding.scope scope=scope5 kind=block parent=rename@1

        this;
    }
}

interface Reader<T> {
/// @binding.symbol name=Reader role=namespace form=interface scope=<module>@2
/// @binding.scope scope=Reader kind=namespace parent=<module>@3 owner=Reader
/// @binding.symbol name=T#2 role=local form=type_alias scope=Reader@0

    read(value: T): Result<T>;
    /// @binding.symbol name=read role=item form=function scope=Reader@1
    /// @binding.scope scope=read kind=function parent=Reader@2 owner=read
    /// @binding.symbol name=value role=local form=variable scope=read@0

    type Item = T;
    /// @binding.symbol name=Item role=item form=type_alias scope=Reader@2
    /// @binding.scope scope=Item kind=type parent=Reader@3 owner=Item

}
/// @binding.symbol name=<module> role=namespace form=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=13 scopes=9 declarations=12 node_scopes=30 replaced_symbols=0 replaced_scopes=0
"#,
    );
}
