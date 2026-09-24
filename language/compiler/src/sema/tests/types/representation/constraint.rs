use crate::tests::{DirRows, TestSession};

#[test]
fn test_interface_parameter_dispatches_dynamically() {
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {
    draw(): void;
}

function paint(item: Drawable): void {
    item.draw();
}

=== dir ===
interface Drawable {
/// @generic.template symbol=Drawable parameters=(this: Drawable)
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable template=(this: Drawable)
/// @definition.where symbol=Drawable relation=satisfies left=this right=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=() => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=() => void

}

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=(Drawable) => void
/// @type.symbol symbol=paint.item source="item: Drawable" type=Drawable
/// @resolution.name source=Drawable target=Drawable

    item.draw();
    /// @resolution.name source=item target=paint.item
    /// @resolution.member source=item.draw receiver=Drawable type=() => void kind=symbol target_receiver=Drawable dispatch=dynamic constraint=Drawable target=Drawable.draw
    /// @resolution.call source=item.draw() parameters=() return=void kind=dynamic target=Drawable.draw receiver=Drawable constraint=Drawable
    /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=item root=paint.item

}
"#,
    );
}

#[test]
fn test_closed_alias_parameter_keeps_direct_dispatch() {
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Drawable = {
    draw(): void;
};

function paint(item: Drawable): void {
    item.draw();
}

=== dir ===
type Drawable = {
/// @type.symbol symbol=Drawable type={ draw(): void }
/// @definition.type symbol=Drawable value={ draw(): void }

    draw(): void;
};

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=(Drawable) => void
/// @type.symbol symbol=paint.item source="item: Drawable" type=Drawable
/// @resolution.name source=Drawable target=Drawable

    item.draw();
    /// @resolution.name source=item target=paint.item
    /// @resolution.member source=item.draw receiver=Drawable type=() => void kind=field target_receiver=Drawable key=draw target_type=() => void
    /// @resolution.call source=item.draw() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=item root=paint.item
    /// @resolution.place source=item.draw placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=item.draw root=paint.item keys=[draw]

}
"#,
    );
}
