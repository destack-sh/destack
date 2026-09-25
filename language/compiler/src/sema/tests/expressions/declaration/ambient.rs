use crate::tests::{DirRows, TestSession};

/// An ambient signature has no implementation to run a parameter initializer.
#[test]
fn test_reject_a_parameter_initializer_on_an_ambient_signature() {
    let session = TestSession::single(
        r#"
declare function read(count: int32 = 1): void;

class Reader {
    declare read(count: int32 = 1): void;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function read(count: int32 = 1): void;

class Reader {
    declare read(count: int32 = 1): void;
}

=== dir ===
declare function read(count: int32 = 1): void;
/// @type.symbol symbol=read source="declare function read(count: int32 = 1): void" type=(int32 | undefined?) => void

class Reader {
/// @type.symbol symbol=Reader type=typeof Reader
/// @definition.class symbol=Reader
/// @definition.method symbol=Reader.read source="declare read(count: int32 = 1): void" slot=read type=(this: Reader, int32 | undefined?) => void

    declare read(count: int32 = 1): void;
    /// @type.symbol symbol=Reader.read source="declare read(count: int32 = 1): void" type=(this: Reader, int32 | undefined?) => void
    /// @type.symbol symbol=Reader.read.count source="count: int32 = 1" type=int32

}
"#,
        r#"
/// @diagnostic.error id=parameter-initializer-outside-implementation message="a parameter initializer is only allowed in a function or constructor implementation"
/// @diagnostic.label line=2 column=38 span="1" line_source="declare function read(count: int32 = 1): void;"
/// @diagnostic.error id=parameter-initializer-outside-implementation message="a parameter initializer is only allowed in a function or constructor implementation"
/// @diagnostic.label line=5 column=33 span="1" line_source="declare read(count: int32 = 1): void;"
"#,
    );
}
