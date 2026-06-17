use crate::tests::{DirRows, TestSession};

#[test]
fn test_interface_parameter_induces_hidden_generic_constraint() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

function paint(item: Drawable): void {
    item.draw();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {
    draw(): void;
}

function paint<T0: Drawable>(item: T0): void {
    item.draw();
}

=== checked ===
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: Drawable) => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: Drawable) => void

}

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=<paint.T0: Drawable>(paint.T0) => void
/// @generic.template symbol=paint parameters=[T0: Drawable]

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=symbol target=Drawable.draw
    /// @resolution.call source="item.draw()" parameters=() return=void kind=symbol target=Drawable.draw receiver=paint.T0
}
"#);
}

#[test]
fn test_type_alias_parameter_induces_hidden_generic_constraint() {
    let session = TestSession::single(
        r#"
type Drawable = {
    draw(): void;
};

function paint(item: Drawable): void {
    item.draw();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Drawable = {
    draw(): void;
};

function paint<T0: Drawable>(item: T0): void {
    item.draw();
}

=== checked ===
type Drawable = {
/// @type.symbol symbol=Drawable type={ draw(): void }
/// @definition.type symbol=Drawable source="type Drawable = {\n    draw(): void;\n}" value={ draw(): void }

    draw(): void;
    /// @type.symbol symbol=Drawable.draw type=() => void

};

function paint(item: Drawable): void {
/// @generic.template symbol=paint parameters=[T0: Drawable]
/// @type.symbol symbol=paint type=<paint.T0: Drawable>(paint.T0) => void
/// @type.symbol symbol=item type=paint.T0
/// @resolution.name source=Drawable target=Drawable

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=symbol target=Drawable.draw
    /// @resolution.call source=item.draw() parameters=() return=void kind=symbol target=Drawable.draw receiver=paint.T0

}
"#);
}
