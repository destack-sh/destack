use crate::tests::{DirRows, TestSession};

/// Two recursive object aliases of the same shape accept each other.
#[test]
fn test_recursive_object_aliases_accept_each_other() {
    let session = TestSession::single(
        r#"
type TreeA = { value: float64; child: TreeA | null };
type TreeB = { value: float64; child: TreeB | null };

declare const source: TreeA;
const tree: TreeB = source;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type TreeA = { value: float64; child: TreeA | null };
type TreeB = { value: float64; child: TreeB | null };

declare const source: { value: float64; child: TreeA | null };
const tree: { value: float64; child: TreeB | null } = source;

=== dir ===
type TreeA = { value: float64; child: TreeA | null };
/// @type.symbol symbol=TreeA source="type TreeA = { value: float64; child: TreeA | null }" type={ value: float64; child: TreeA | null }
/// @definition.type symbol=TreeA source="type TreeA = { value: float64; child: TreeA | null }" value={ value: float64; child: TreeA | null }
/// @resolution.name source=TreeA target=TreeA

type TreeB = { value: float64; child: TreeB | null };
/// @type.symbol symbol=TreeB source="type TreeB = { value: float64; child: TreeB | null }" type={ value: float64; child: TreeB | null }
/// @definition.type symbol=TreeB source="type TreeB = { value: float64; child: TreeB | null }" value={ value: float64; child: TreeB | null }
/// @resolution.name source=TreeB target=TreeB

declare const source: TreeA;
/// @type.symbol symbol=source source=source type={ value: float64; child: TreeA | null }
/// @resolution.pattern source=source kind=binding target=source
/// @resolution.name source=TreeA target=TreeA

const tree: TreeB = source;
/// @type.symbol symbol=tree source=tree type={ value: float64; child: TreeB | null }
/// @resolution.pattern source=tree kind=binding target=tree
/// @resolution.name source=TreeB target=TreeB
/// @type.node source=source type={ value: float64; child: TreeA | null }
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=source root=source
"#,
    );
}

/// Two structs holding each other by value report a circular representation.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
/// @diagnostic.error id=circular-type message="type is circular"
/// @diagnostic.label line=3 column=5 span="right" line_source="right: Right;"
/// @diagnostic.error id=circular-type message="type is circular"
/// @diagnostic.label line=7 column=5 span="left" line_source="left: Left;"
"#,
    );
}

/// Classes holding each other across modules keep their nominal targets.
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

    compiler.assert_dir_many(
        &["player.ds", "world.ds"],
        DirRows::checked(),
        r#"
=== player.ds ===

=== annotated ===
import { World } from "./world.ds";

export class Player {
    world: World;

    constructor(world: World) {
        this.world = world;
    }
}

=== dir ===
import { World } from "./world.ds";

export class Player {
/// @type.symbol symbol=Player type=Player
/// @definition.class symbol=Player
/// @definition.field symbol=Player.world source="world: World" key=world type=world.World
/// @definition.method symbol=Player.constructor slot=constructor role=constructor type=<Player.constructor.P0: Place>(world.World) => Managed<Player, Player.constructor.P0>

    world: World;
    /// @type.symbol symbol=Player.world source="world: World" type=world.World
    /// @resolution.name source=World target=world.World

    constructor(world: World) {
    /// @generic.template symbol=Player.constructor parameters=(P0: Place)
    /// @type.symbol symbol=Player.constructor type=<Player.constructor.P0: Place>(world.World) => Managed<Player, Player.constructor.P0>
    /// @type.symbol symbol=Player.constructor.world source="world: World" type=world.World
    /// @resolution.name source=World target=world.World

        this.world = world;
        /// @resolution.receiver source=this kind=this declaration=Player type=Player
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.world kind=place
        /// @resolution.access source=this.world root=this keys=[world]
        /// @resolution.assignment source=this.world write="receiver=Player, target=field(receiver=Player, target=Player.world, type=world.World), type=world.World" type=world.World
        /// @resolution.name source=world target=Player.constructor.world
        /// @resolution.place source=world placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=world root=Player.constructor.world

    }
}

=== world.ds ===

=== annotated ===
import { Player } from "./player.ds";

export class World {
    player: Player;

    constructor(player: Player) {
        this.player = player;
    }
}

=== dir ===
import { Player } from "./player.ds";

export class World {
/// @type.symbol symbol=World type=World
/// @definition.class symbol=World
/// @definition.field symbol=World.player source="player: Player" key=player type=player.Player
/// @definition.method symbol=World.constructor slot=constructor role=constructor type=<World.constructor.P0: Place>(player.Player) => Managed<World, World.constructor.P0>

    player: Player;
    /// @type.symbol symbol=World.player source="player: Player" type=player.Player
    /// @resolution.name source=Player target=player.Player

    constructor(player: Player) {
    /// @generic.template symbol=World.constructor parameters=(P0: Place)
    /// @type.symbol symbol=World.constructor type=<World.constructor.P0: Place>(player.Player) => Managed<World, World.constructor.P0>
    /// @type.symbol symbol=World.constructor.player source="player: Player" type=player.Player
    /// @resolution.name source=Player target=player.Player

        this.player = player;
        /// @resolution.receiver source=this kind=this declaration=World type=World
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.player kind=place
        /// @resolution.access source=this.player root=this keys=[player]
        /// @resolution.assignment source=this.player write="receiver=World, target=field(receiver=World, target=World.player, type=player.Player), type=player.Player" type=player.Player
        /// @resolution.name source=player target=World.constructor.player
        /// @resolution.place source=player placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=player root=World.constructor.player

    }
}
"#,
    );
}
