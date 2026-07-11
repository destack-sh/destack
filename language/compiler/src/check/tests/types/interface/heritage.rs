use crate::tests::{DirRows, TestSession};

#[test]
fn test_interface_extends_inherits_interface_members() {
    let session = TestSession::single(
        r#"
interface Named {
    name(): string;
}

interface Drawable extends Named {
    draw(): void;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {
    name(): string;
}

interface Drawable extends Named {
    draw(): void;
}

=== checked ===
interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: Named) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: Named) => string

}

interface Drawable extends Named {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.extends symbol=Drawable source=Named target=Named
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: Drawable) => void
/// @resolution.name source=Named target=Named

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: Drawable) => void

}
"#,
    );
}

#[test]
fn test_interface_extends_rejects_non_interface_base() {
    let session = TestSession::single(
        r#"
struct Shape {}

interface Drawable extends Shape {
    draw(): void;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Shape {}

interface Drawable extends Shape {
    draw(): void;
}

=== checked ===
struct Shape {}
/// @type.symbol symbol=Shape source="struct Shape {}" type=Shape
/// @definition.struct symbol=Shape source="struct Shape {}"

interface Drawable extends Shape {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: Drawable) => void
/// @resolution.name source=Shape target=Shape

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: Drawable) => void

}
"#,
        r#"
/// @diagnostic.error code=EC615 message="interface 'Drawable' can only extend interfaces, not 'Shape'"
/// @diagnostic.label line=4 column=28 span="Shape" line_source="interface Drawable extends Shape {"
"#,
    );
}

#[test]
fn test_interface_extends_rejects_type_alias_base() {
    let session = TestSession::single(
        r#"
interface Named {}
type Alias = Named;

interface Drawable extends Alias {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {}
type Alias = Named;

interface Drawable extends Alias {}

=== checked ===
interface Named {}
/// @type.symbol symbol=Named source="interface Named {}" type=Named
/// @definition.interface symbol=Named source="interface Named {}"

type Alias = Named;
/// @type.symbol symbol=Alias source="type Alias = Named" type=Named
/// @definition.type symbol=Alias source="type Alias = Named" value=Named
/// @resolution.name source=Named target=Named

interface Drawable extends Alias {}
/// @type.symbol symbol=Drawable source="interface Drawable extends Alias {}" type=Drawable
/// @definition.interface symbol=Drawable source="interface Drawable extends Alias {}"
/// @resolution.name source=Alias target=Alias
"#,
        r#"
/// @diagnostic.error code=EC615 message="interface 'Drawable' can only extend interfaces, not 'Alias'"
/// @diagnostic.label line=5 column=28 span="Alias" line_source="interface Drawable extends Alias {}"
"#,
    );
}

#[test]
fn test_interface_extends_rejects_union_base() {
    let session = TestSession::single(
        r#"
interface Named {}
interface DrawableBase {}

interface Drawable extends Named | DrawableBase {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {}
interface DrawableBase {}

interface Drawable extends Named | DrawableBase {}

=== checked ===
interface Named {}
/// @type.symbol symbol=Named source="interface Named {}" type=Named
/// @definition.interface symbol=Named source="interface Named {}"

interface DrawableBase {}
/// @type.symbol symbol=DrawableBase source="interface DrawableBase {}" type=DrawableBase
/// @definition.interface symbol=DrawableBase source="interface DrawableBase {}"

interface Drawable extends Named | DrawableBase {}
/// @type.symbol symbol=Drawable source="interface Drawable extends Named | DrawableBase {}" type=Drawable
/// @definition.interface symbol=Drawable source="interface Drawable extends Named | DrawableBase {}"
/// @resolution.name source=Named target=Named
/// @resolution.name source=DrawableBase target=DrawableBase
"#,
        r#"
/// @diagnostic.error code=EC615 message="interface 'Drawable' can only extend interfaces, not 'Named | DrawableBase'"
/// @diagnostic.label line=5 column=34 span="|" line_source="interface Drawable extends Named | DrawableBase {}"
"#,
    );
}

#[test]
fn test_interface_extends_rejects_conflicting_generic_base() {
    let session = TestSession::single(
        r#"
interface Base<T> {
    value(): T;
}

interface Left extends Base<string> {}
interface Right extends Base<int32> {}

interface Both extends Left, Right {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Base<out T> {
    value(): T;
}

interface Left extends Base<string> {}
interface Right extends Base<int32> {}

interface Both extends Left, Right {}

=== checked ===
interface Base<T> {
/// @generic.template symbol=Base parameters=(out T)
/// @type.symbol symbol=Base type=Base
/// @definition.interface symbol=Base template=(out T)
/// @definition.where symbol=Base relation=satisfies left=this right=Base<T>
/// @definition.method symbol=Base.value source="value(): T" slot=value type=(this: Base<T>) => T
/// @type.symbol symbol=Base.T source=T type=T

    value(): T;
    /// @type.symbol symbol=Base.value source="value(): T" type=(this: Base<T>) => T
    /// @resolution.name source=T target=Base.T

}

interface Left extends Base<string> {}
/// @type.symbol symbol=Left source="interface Left extends Base<string> {}" type=Left
/// @definition.interface symbol=Left source="interface Left extends Base<string> {}"
/// @definition.extends symbol=Left source=Base<string> target=Base arguments=(string)
/// @resolution.name source=Base target=Base

interface Right extends Base<int32> {}
/// @type.symbol symbol=Right source="interface Right extends Base<int32> {}" type=Right
/// @definition.interface symbol=Right source="interface Right extends Base<int32> {}"
/// @definition.extends symbol=Right source=Base<int32> target=Base arguments=(int32)
/// @resolution.name source=Base target=Base

interface Both extends Left, Right {}
/// @type.symbol symbol=Both source="interface Both extends Left, Right {}" type=Both
/// @definition.interface symbol=Both source="interface Both extends Left, Right {}"
/// @definition.extends symbol=Both source=Left target=Left
/// @definition.extends symbol=Both source=Right target=Right
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

/// @generic.instance id=Base<T> template=Base arguments=(T)
"#,
        r#"
/// @diagnostic.error code=EC617 message="type 'Both' has conflicting heritage for 'Base'"
/// @diagnostic.label line=9 column=30 span="Right" line_source="interface Both extends Left, Right {}"
"#,
    );
}

#[test]
fn test_interface_extends_rejects_circular_heritage() {
    let session = TestSession::single(
        r#"
interface Left extends Right {}
interface Right extends Left {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Left extends Right {}
interface Right extends Left {}

=== checked ===
interface Left extends Right {}
/// @type.symbol symbol=Left source="interface Left extends Right {}" type=Left
/// @definition.interface symbol=Left source="interface Left extends Right {}"
/// @definition.extends symbol=Left source=Right target=Right
/// @resolution.name source=Right target=Right

interface Right extends Left {}
/// @type.symbol symbol=Right source="interface Right extends Left {}" type=Right
/// @definition.interface symbol=Right source="interface Right extends Left {}"
/// @definition.extends symbol=Right source=Left target=Left
/// @resolution.name source=Left target=Left
"#,
        r#"
/// @diagnostic.error code=EC618 message="type 'Left' has circular heritage"
/// @diagnostic.label line=2 column=24 span="Right" line_source="interface Left extends Right {}"
/// @diagnostic.error code=EC618 message="type 'Right' has circular heritage"
/// @diagnostic.label line=3 column=25 span="Left" line_source="interface Right extends Left {}"
"#,
    );
}
