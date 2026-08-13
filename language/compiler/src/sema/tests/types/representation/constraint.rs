use crate::tests::{DirRows, TestSession};

#[test]
fn test_interface_parameter_erases_to_dynamic_storage() {
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

function paint(item: Dynamic<Drawable>): void {
    item.draw();
}

=== checked ===
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: this) => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: this) => void

}

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=(Drawable) => void
/// @type.symbol symbol=paint type=(Dynamic<Drawable>) => void
/// @type.symbol symbol=paint.item source="item: Drawable" type=Dynamic<Drawable>
/// @resolution.name source=Drawable target=Drawable

    item.draw();
    /// @resolution.name source=item target=paint.item
    /// @resolution.member source=item.draw receiver=Dynamic<Drawable> type=(this: Drawable) => void kind=symbol target_receiver=Dynamic<Drawable> dispatch=dynamic constraint=Drawable target=Drawable.draw
    /// @resolution.call source=item.draw() parameters=() return=void kind=dynamic target=Drawable.draw receiver=Dynamic<Drawable> constraint=Drawable
    /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=item root=paint.item

}
"#,
    );
}

#[test]
fn test_closed_alias_parameter_stays_direct_storage() {
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

function paint(item: { draw: () => void }): void {
    item.draw();
}

=== checked ===
type Drawable = {
/// @type.symbol symbol=Drawable type={ draw(): void }
/// @definition.type symbol=Drawable value={ draw(): void }

    draw(): void;
};

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=(Drawable) => void
/// @type.symbol symbol=paint.item source="item: Drawable" type={ draw(): void }
/// @resolution.name source=Drawable target=Drawable

    item.draw();
    /// @resolution.name source=item target=paint.item
    /// @resolution.member source=item.draw receiver={ draw(): void } type=() => void kind=field target_receiver={ draw(): void } key=draw target_type=() => void
    /// @resolution.call source=item.draw() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=item root=paint.item
    /// @resolution.access source=item.draw root=paint.item keys=[draw]

}
"#,
    );
}
