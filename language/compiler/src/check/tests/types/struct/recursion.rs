use crate::tests::{DirRows, TestSession};

#[test]
fn test_recursive_struct_fields_report_circular_representation() {
    let session = TestSession::single(
        r#"
struct Left {
    right: Right;
}

struct Right {
    left: Left;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Left {
    right: Right;
}

struct Right {
    left: Left;
}

=== checked ===
struct Left {
/// @type.symbol symbol=Left type=Left

    right: Right;
    /// @resolution.name source=Right target=Right
    /// @type.symbol symbol=Left.right type=Right
}

struct Right {
/// @type.symbol symbol=Right type=Right

    left: Left;
    /// @resolution.name source=Left target=Left
    /// @type.symbol symbol=Right.left type=Left
}
"#,
        r#"
/// @diagnostic.error code=EC103 message="type is circular"
/// @diagnostic.label line=3 column=5 source="right: Right;"
"#,
    );
}

#[test]
fn test_circular_class_fields_preserve_nominal_targets() {
    let compiler = TestSession::builder()
        .module(
            "player.ds",
            r#"
import { World } from "./world.ds";

export class Player {
    world: World;
}
"#,
        )
        .module(
            "world.ds",
            r#"
import { Player } from "./player.ds";

export class World {
    player: Player;
}
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["player.ds", "world.ds"],
        DirRows::checked(),
        r#"
=== player.ds ===

=== annotated ===
import { World } from "./world.ds";

export class Player {
    world: World;
}

=== checked ===
import { World } from "./world.ds";
/// @resolution.name source=World target=world.World

export class Player {
/// @type.symbol symbol=Player type=Player

    world: World;
    /// @resolution.name source=World target=world.World
    /// @type.symbol symbol=Player.world type=world.World
}


=== world.ds ===

=== annotated ===
import { Player } from "./player.ds";

export class World {
    player: Player;
}

=== checked ===
import { Player } from "./player.ds";
/// @resolution.name source=Player target=player.Player

export class World {
/// @type.symbol symbol=World type=World

    player: Player;
    /// @resolution.name source=Player target=player.Player
    /// @type.symbol symbol=World.player type=player.Player
}
"#,
    );
}
