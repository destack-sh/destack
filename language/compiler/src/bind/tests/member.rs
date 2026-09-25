use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_member_scopes() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
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

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding().with_summaries(),
        r#"
struct User<T> {
/// @binding.symbol symbol=User role=namespace kind=struct scope=<module>@1
/// @binding.symbol symbol=symbol4 role=local kind=variable scope=User@2
/// @binding.scope scope=User kind=namespace parent=<module>@2 owner=User
/// @binding.owner_scope owner=User scope=User
/// @binding.symbol symbol=T#1 role=local kind=generic_type_parameter scope=User@0

    id: string;
    /// @binding.symbol symbol=id role=item kind=variable scope=User@1 visibility=member
    /// @binding.receiver node=member symbol=symbol4

    type Id = string;
    /// @binding.symbol symbol=Id role=item kind=associated_type scope=User@3 visibility=member
    /// @binding.symbol symbol=symbol6 role=local kind=variable scope=Id@0
    /// @binding.scope scope=Id kind=type parent=User@4 owner=Id
    /// @binding.receiver node=member symbol=symbol6
    /// @binding.owner_scope owner=Id scope=Id

    static defaultName: string = "guest";
    /// @binding.symbol symbol=defaultName role=item kind=variable scope=User@4 visibility=member

    rename(name: string): User<T> {
    /// @binding.symbol symbol=rename role=item kind=function scope=User@5 visibility=member
    /// @binding.symbol symbol=this#1 role=local kind=variable scope=rename@0
    /// @binding.scope scope=rename kind=function parent=User@6 owner=rename
    /// @binding.receiver node=member symbol=this#1
    /// @binding.owner_scope owner=rename scope=rename
    /// @binding.symbol symbol=name role=local kind=parameter scope=rename@1
    /// @binding.scope scope=scope5 kind=block parent=rename@2

        this;
    }
}

interface Reader<T> {
/// @binding.symbol symbol=Reader role=namespace kind=interface scope=<module>@2
/// @binding.scope scope=Reader kind=namespace parent=<module>@3 owner=Reader
/// @binding.owner_scope owner=Reader scope=Reader
/// @binding.symbol symbol=T#2 role=local kind=generic_type_parameter scope=Reader@0

    read(value: T): Result<T>;
    /// @binding.symbol symbol=read role=item kind=function scope=Reader@1 visibility=member
    /// @binding.symbol symbol=this#2 role=local kind=variable scope=read@0
    /// @binding.scope scope=read kind=function parent=Reader@2 owner=read
    /// @binding.receiver node=type_member symbol=this#2
    /// @binding.owner_scope owner=read scope=read
    /// @binding.symbol symbol=value role=local kind=parameter scope=read@1

    type Item = T;
    /// @binding.symbol symbol=Item role=item kind=associated_type scope=Reader@2 visibility=member
    /// @binding.symbol symbol=symbol17 role=local kind=variable scope=Item@0
    /// @binding.scope scope=Item kind=type parent=Reader@3 owner=Item
    /// @binding.receiver node=type_member symbol=symbol17
    /// @binding.owner_scope owner=Item scope=Item

}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global

/// @binding.summary symbols=18 scopes=9 declarations=12 receivers=5 node_scopes=30 owner_scopes=6
"#,
    );
}

#[test]
fn test_bind_enum_fields() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
enum Priority {
    Low = 1,
    High = 2,

    label(): string {
        "priority";
    }
}
"#,
        )
        .build();

    compiler.assert_dir_bound(
        "main.tspp",
        DirRows::binding(),
        r#"
enum Priority {
/// @binding.symbol symbol=Priority role=namespace kind=enum scope=<module>@1
/// @binding.scope scope=Priority kind=namespace parent=<module>@2 owner=Priority
/// @binding.owner_scope owner=Priority scope=Priority

    Low = 1,
    /// @binding.symbol symbol=Low role=item kind=variant scope=Priority@0 visibility=member

    High = 2,
    /// @binding.symbol symbol=High role=item kind=variant scope=Priority@1 visibility=member

    label(): string {
    /// @binding.symbol symbol=label role=item kind=function scope=Priority@2 visibility=member
    /// @binding.symbol symbol=this role=local kind=variable scope=label@0
    /// @binding.scope scope=label kind=function parent=Priority@3 owner=label
    /// @binding.receiver node=member symbol=this
    /// @binding.owner_scope owner=label scope=label
    /// @binding.scope scope=scope4 kind=block parent=label@1

        "priority";
    }
}

/// @binding.symbol symbol=<module> role=namespace kind=variable scope=<module>@end
/// @binding.scope scope=<module> kind=module owner=<module>
/// @binding.scope scope=scope1 kind=global
"#,
    );
}
