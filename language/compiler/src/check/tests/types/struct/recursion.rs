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
/// @definition.struct symbol=Left
/// @definition.field symbol=Left.right source="right: Right" key=right type=Right

    right: Right;
    /// @type.symbol symbol=Left.right source="right: Right" type=Right
    /// @resolution.name source=Right target=Right

}

struct Right {
/// @type.symbol symbol=Right type=Right
/// @definition.struct symbol=Right
/// @definition.field symbol=Right.left source="left: Left" key=left type=Left

    left: Left;
    /// @type.symbol symbol=Right.left source="left: Left" type=Left
    /// @resolution.name source=Left target=Left

}
"#,
        r#"
/// @diagnostic.error code=EC103 message="type is circular"
/// @diagnostic.label line=7 column=5 span="left" line_source="left: Left;"
/// @diagnostic.error code=EC103 message="type is circular"
/// @diagnostic.label line=3 column=5 span="right" line_source="right: Right;"
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

    constructor(world: World) {
        this.world = world;
    }
}
"#,
        )
        .module(
            "world.ds",
            r#"
import { Player } from "./player.ds";

export class World {
    player: Player;

    constructor(player: Player) {
        this.player = player;
    }
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

    constructor(world: World): Player {
        this.world = world;
    }
}

=== checked ===
import { World } from "./world.ds";

export class Player {
/// @type.symbol symbol=Player type=Player
/// @definition.class symbol=Player
/// @definition.field symbol=Player.world source="world: World" key=world type=world.World
/// @definition.method symbol=Player.constructor slot=constructor role=constructor type=(world.World) => Player

    world: World;
    /// @type.symbol symbol=Player.world source="world: World" type=world.World
    /// @resolution.name source=World target=world.World

    constructor(world: World) {
    /// @type.symbol symbol=Player.constructor type=(world.World) => Player
    /// @type.symbol symbol=Player.constructor.world source="world: World" type=world.World
    /// @resolution.name source=World target=world.World

        this.world = world;
        /// @resolution.receiver source=this kind=this declaration=Player type=Player
        /// @resolution.pattern.assign source=this.world kind=place place=field(Player.world) type=world.World
        /// @resolution.name source=world target=Player.constructor.world

    }
}

=== world.ds ===

=== annotated ===
import { Player } from "./player.ds";

export class World {
    player: Player;

    constructor(player: Player): World {
        this.player = player;
    }
}

=== checked ===
import { Player } from "./player.ds";

export class World {
/// @type.symbol symbol=World type=World
/// @definition.class symbol=World
/// @definition.field symbol=World.player source="player: Player" key=player type=player.Player
/// @definition.method symbol=World.constructor slot=constructor role=constructor type=(player.Player) => World

    player: Player;
    /// @type.symbol symbol=World.player source="player: Player" type=player.Player
    /// @resolution.name source=Player target=player.Player

    constructor(player: Player) {
    /// @type.symbol symbol=World.constructor type=(player.Player) => World
    /// @type.symbol symbol=World.constructor.player source="player: Player" type=player.Player
    /// @resolution.name source=Player target=player.Player

        this.player = player;
        /// @resolution.receiver source=this kind=this declaration=World type=World
        /// @resolution.pattern.assign source=this.player kind=place place=field(World.player) type=player.Player
        /// @resolution.name source=player target=World.constructor.player

    }
}
"#,
    );
}
