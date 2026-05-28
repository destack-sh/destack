use crate::tests::{DirRows, TestSession};

#[test]
fn test_circular_struct_fields_preserve_nominal_targets() {
    let compiler = TestSession::new()
        .module(
            "left.ds",
            r#"
import { Right } from "./right.ds";

export struct Left {
    right: Right;
}
"#,
        )
        .module(
            "right.ds",
            r#"
import { Left } from "./left.ds";

export struct Right {
    left: Left;
}
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["left.ds", "right.ds"],
        DirRows::checked(),
        r#"
=== left.ds ===
import { Right } from "./right.ds";
/// @resolution.name source=Right target=right.Right

export struct Left {
/// @type.symbol symbol=Left type=Left

    right: Right;
    /// @resolution.name source=Right target=right.Right
    /// @type.symbol symbol=Left.right type=right.Right
}


=== right.ds ===
import { Left } from "./left.ds";
/// @resolution.name source=Left target=left.Left

export struct Right {
/// @type.symbol symbol=Right type=Right

    left: Left;
    /// @resolution.name source=Left target=left.Left
    /// @type.symbol symbol=Right.left type=left.Left
}
"#,
    );
}

#[test]
fn test_circular_class_fields_preserve_nominal_targets() {
    let compiler = TestSession::new()
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
import { World } from "./world.ds";
/// @resolution.name source=World target=world.World

export class Player {
/// @type.symbol symbol=Player type=Player

    world: World;
    /// @resolution.name source=World target=world.World
    /// @type.symbol symbol=Player.world type=world.World
}


=== world.ds ===
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
