use crate::tests::{DirRows, TestSession};

/// Persist a C representation on a struct definition.
#[test]
fn test_apply_c_struct_representation() {
    let session = TestSession::single(
        r#"
@repr("C")
struct Header {
    value: uint8;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("C")
struct Header {
    value: uint8;
}

=== dir ===
@repr("C")
/// @decorator.node source="@repr(\"C\")" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

struct Header {
/// @definition.struct symbol=Header
/// @definition.field symbol=Header.value source="value: uint8" key=value type=uint8

    value: uint8;
}
"#,
    );
}

/// Persist a transparent representation on a newtype definition.
#[test]
fn test_apply_transparent_newtype_representation() {
    let session = TestSession::single(
        r#"
@repr("transparent")
newtype Handle = int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("transparent")
newtype Handle = int32;

=== dir ===
@repr("transparent")
/// @decorator.node source="@repr(\"transparent\")" owner="newtype Handle = int32" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("transparent") as Representation) newtype=repr backing=(Representation,) value="repr(\"transparent\")"

newtype Handle = int32;
/// @definition.newtype symbol=Handle source="newtype Handle = int32" backing=int32 constructors=[(int32) => Handle]
"#,
    );
}

/// Persist a transparent representation on a single-field struct definition.
#[test]
fn test_apply_transparent_struct_representation() {
    let session = TestSession::single(
        r#"
@repr("transparent")
struct Handle {
    value: uint8;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("transparent")
struct Handle {
    value: uint8;
}

=== dir ===
@repr("transparent")
/// @decorator.node source="@repr(\"transparent\")" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("transparent") as Representation) newtype=repr backing=(Representation,) value="repr(\"transparent\")"

struct Handle {
/// @definition.struct symbol=Handle
/// @definition.field symbol=Handle.value source="value: uint8" key=value type=uint8

    value: uint8;
}
"#,
    );
}

/// Persist a C representation on a non-polymorphic class definition.
#[test]
fn test_apply_c_class_representation() {
    let session = TestSession::single(
        r#"
@repr("C")
class Handle {}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("C")
class Handle {}

=== dir ===
@repr("C")
/// @decorator.node source="@repr(\"C\")" owner="class Handle {}" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

class Handle {}
/// @definition.class symbol=Handle source="class Handle {}"
"#,
    );
}

/// Reject C representation on a class declaring virtual dispatch.
#[test]
fn test_reject_c_representation_on_virtual_class() {
    let session = TestSession::single(
        r#"
@repr("C")
class Handle {
    virtual read(): uint8 { return 0; }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("C")
class Handle {
    virtual read(): uint8 {
        return 0;
    }
}

=== dir ===
@repr("C")
/// @decorator.node source="@repr(\"C\")" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

class Handle {
/// @definition.class symbol=Handle
/// @definition.method symbol=Handle.read source="virtual read(): uint8 { return 0; }" slot=read abstraction=virtual type=(this: Handle) => uint8

    virtual read(): uint8 { return 0; }
}
"#,
        r#"
/// @diagnostic.error id=unsupported-representation message="representation 'C' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"C\")" line_source="@repr(\"C\")"
"#,
    );
}

/// Reject C representation inherited from a polymorphic base class.
#[test]
fn test_reject_c_representation_on_virtual_subclass() {
    let session = TestSession::single(
        r#"
abstract class Base {
    abstract read(): uint8;
}

@repr("C")
abstract class Handle extends Base {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
abstract class Base {
    abstract read(): uint8;
}

@repr("C")
abstract class Handle extends Base {}

=== dir ===
abstract class Base {
/// @definition.class symbol=Base abstract=true
/// @definition.method symbol=Base.read source="abstract read(): uint8" slot=read abstraction=abstract type=(this: Base) => uint8

    abstract read(): uint8;
}

@repr("C")
/// @decorator.node source="@repr(\"C\")" owner="abstract class Handle extends Base {}" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

abstract class Handle extends Base {}
/// @definition.class symbol=Handle source="abstract class Handle extends Base {}" abstract=true
/// @definition.extends symbol=Handle source=Base target=Base
"#,
        r#"
/// @diagnostic.error id=unsupported-representation message="representation 'C' is not supported by this declaration"
/// @diagnostic.label line=6 column=2 span="repr(\"C\")" line_source="@repr(\"C\")"
"#,
    );
}

/// Reject a transparent representation on a struct with no fields.
#[test]
fn test_reject_transparent_struct_representation() {
    let session = TestSession::single(
        r#"
@repr("transparent")
struct Header {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("transparent")
struct Header {}

=== dir ===
@repr("transparent")
/// @decorator.node source="@repr(\"transparent\")" owner="struct Header {}" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("transparent") as Representation) newtype=repr backing=(Representation,) value="repr(\"transparent\")"

struct Header {}
/// @definition.struct symbol=Header source="struct Header {}"
"#,
        r#"
/// @diagnostic.error id=unsupported-representation message="representation 'transparent' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"transparent\")" line_source="@repr(\"transparent\")"
"#,
    );
}

/// Reject more than one representation on the same declaration.
#[test]
fn test_reject_duplicate_representation() {
    let session = TestSession::single(
        r#"
@repr("C")
@repr("C")
struct Header {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("C")
@repr("C")
struct Header {}

=== dir ===
@repr("C")
/// @decorator.node source="@repr(\"C\")" owner="struct Header {}" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

@repr("C")
/// @decorator.node source="@repr(\"C\")" owner="struct Header {}" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

struct Header {}
/// @definition.struct symbol=Header source="struct Header {}"
"#,
        r#"
/// @diagnostic.error id=duplicate-representation-decorator message="duplicate representation decorator"
/// @diagnostic.label line=3 column=2 span="repr(\"C\")" line_source="@repr(\"C\")"
/// @diagnostic.related line=2 column=2 span="repr(\"C\")" line_source="@repr(\"C\")" message="first representation selected here"
"#,
    );
}

/// Reject a C representation on a string-backed enum.
#[test]
fn test_reject_c_string_enum_representation() {
    let session = TestSession::single(
        r#"
@repr("C")
enum Mode {
    Read = "read",
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("C")
enum Mode {
    Read = "read",
}

=== dir ===
@repr("C")
/// @decorator.node source="@repr(\"C\")" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("C") as Representation) newtype=repr backing=(Representation,) value="repr(\"C\")"

enum Mode {
/// @definition.enum symbol=Mode backing=string
/// @definition.variant symbol=Mode.Read source="Read = \"read\"" key=Read value="\"read\""

    Read = "read",
}
"#,
        r#"
/// @diagnostic.error id=unsupported-representation message="representation 'C' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"C\")" line_source="@repr(\"C\")"
"#,
    );
}

/// Reject an enum value outside its explicit integer representation.
#[test]
fn test_reject_enum_value_outside_representation() {
    let session = TestSession::single(
        r#"
@repr("uint8")
enum Mode {
    Read = 256,
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_definitions().with_decorators(),
        r#"
=== annotated ===
@repr("uint8")
enum Mode {
    Read = 256,
}

=== dir ===
@repr("uint8")
/// @decorator.node source="@repr(\"uint8\")" expression=repr target=decorator.repr type=repr kind=newtype parameters=(Representation) arguments=(provided("uint8") as Representation) newtype=repr backing=(Representation,) value="repr(\"uint8\")"

enum Mode {
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 256" key=Read value=256

    Read = 256,
}
"#,
        r#"
/// @diagnostic.error id=unsupported-representation message="representation 'uint8' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"uint8\")" line_source="@repr(\"uint8\")"
"#,
    );
}
