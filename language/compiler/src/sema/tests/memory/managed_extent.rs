use crate::tests::{DirRows, TestSession};

/// A place reached through a managed handle lives in the managed extent.
#[test]
fn test_resolve_a_handle_field_place_in_the_managed_extent() {
    let session = TestSession::single(
        r#"
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function read(world: World): int32 {
    const player = &readonly world.player;
    return player.score;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function read(world: World): int32 {
    const player: &'managed readonly Player = &readonly world.player;
    return player.score;
}

=== dir ===
struct Player {
/// @type.symbol symbol=Player type=Player
/// @definition.struct symbol=Player
/// @definition.field symbol=Player.score source="score: int32" key=score type=int32

    score: int32;
    /// @type.symbol symbol=Player.score source="score: int32" type=int32

}

class World {
/// @type.symbol symbol=World type=typeof World
/// @definition.class symbol=World
/// @definition.field symbol=World.player source="player: Player" key=player type=Player
/// @definition.method symbol=World.constructor slot=constructor role=constructor type=<World.constructor.P0: Place>() => Managed<World, World.constructor.P0>

    player: Player;
    /// @type.symbol symbol=World.player source="player: Player" type=Player
    /// @resolution.name source=Player target=Player

    constructor() {
    /// @generic.template symbol=World.constructor parameters=(P0: Place)
    /// @type.symbol symbol=World.constructor type=<World.constructor.P0: Place>() => Managed<World, World.constructor.P0>
    /// @type.symbol symbol=World.constructor.this type=World

        this.player = Player { score: 1 };
        /// @resolution.receiver source=this kind=this declaration=World type=World
        /// @resolution.place source=this placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.player kind=place
        /// @resolution.access source=this.player root=this keys=[player]
        /// @resolution.assignment source=this.player write="receiver=World, target=field(receiver=World, target=World.player, type=Player), type=Player" type=Player
        /// @resolution.name source=Player target=Player

    }
}

function read(world: World): int32 {
/// @type.symbol symbol=read type=(World) => int32
/// @type.symbol symbol=read.world source="world: World" type=World
/// @resolution.name source=World target=World

    const player = &readonly world.player;
    /// @type.symbol symbol=read.player source=player type=Borrowed<Player, "managed" & "local", "readonly">
    /// @resolution.pattern source=player kind=binding target=read.player
    /// @resolution.name source=world target=read.world
    /// @resolution.member source=world.player receiver=World type=Player kind=field target_receiver=World key=player target=World.player target_type=Player
    /// @resolution.place source=world placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=world root=read.world
    /// @resolution.place source=world.player placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=world.player root=read.world keys=[player]

    return player.score;
    /// @resolution.name source=player target=read.player
    /// @resolution.member source=player.score receiver=Borrowed<Player, "managed" & "local", "readonly"> type=int32 kind=field target_receiver=Borrowed<Player, "managed" & "local", "readonly"> key=score target=Player.score target_type=int32
    /// @resolution.place source=player placement="local" lifetime="managed" access="readonly"
    /// @resolution.access source=player root=read.player
    /// @resolution.place source=player.score placement="local" lifetime="managed" access="readonly"
    /// @resolution.access source=player.score root=read.player keys=[score]

}
"#);
}

/// A return borrow with no input region elides to the managed extent in local space.
#[test]
fn test_elide_an_inputless_return_borrow_to_the_managed_extent() {
    let session = TestSession::single(
        r#"
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function spawn(): &Player {
    let world = new World();
    return &world.player;
}

function name(): &readonly string {
    return "player";
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function spawn(): &'managed Player {
    let world: local World = new World();
    return &world.player;
}

function name(): &'managed readonly string {
    return "player" as &'managed readonly string;
}

=== dir ===
struct Player {
/// @type.symbol symbol=Player type=Player
/// @definition.struct symbol=Player
/// @definition.field symbol=Player.score source="score: int32" key=score type=int32

    score: int32;
    /// @type.symbol symbol=Player.score source="score: int32" type=int32

}

class World {
/// @type.symbol symbol=World type=typeof World
/// @definition.class symbol=World
/// @definition.field symbol=World.player source="player: Player" key=player type=Player
/// @definition.method symbol=World.constructor slot=constructor role=constructor type=<World.constructor.P0: Place>() => Managed<this, World.constructor.P0>

    player: Player;
    /// @type.symbol symbol=World.player source="player: Player" type=Player
    /// @resolution.name source=Player target=Player

    constructor() {
    /// @generic.template symbol=World.constructor parameters=(P0: Place)
    /// @type.symbol symbol=World.constructor type=<World.constructor.P0: Place>() => Managed<this, World.constructor.P0>
    /// @type.symbol symbol=World.constructor.this type=World

        this.player = Player { score: 1 };
        /// @resolution.receiver source=this kind=this declaration=World type=World
        /// @resolution.place source=this placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.player kind=place
        /// @resolution.access source=this.player root=this keys=[player]
        /// @resolution.assignment source=this.player write="receiver=World, target=field(receiver=World, target=World.player, type=Player), type=Player" type=Player
        /// @resolution.name source=Player target=Player

    }
}

function spawn(): &Player {
/// @type.symbol symbol=spawn type=() => Borrowed<Player, "managed" & "local", "mutable">
/// @resolution.name source=Player target=Player

    let world = new World();
    /// @type.symbol symbol=spawn.world source=world type=local World
    /// @resolution.pattern source=world kind=binding target=spawn.world
    /// @resolution.construct source="new World()" parameters=() return=local World kind=class target=World constructor=World.constructor
    /// @generic.instantiation id="World.constructor<\"local\">" template=World.constructor arguments=("local")
    /// @generic.instantiation id="World<\"local\">" template=World arguments=("local")
    /// @resolution.name source=World target=World

    return &world.player;
    /// @resolution.name source=world target=spawn.world
    /// @resolution.member source=world.player receiver=local World type=Player kind=field target_receiver=local World key=player target=World.player target_type=Player
    /// @resolution.place source=world placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=world root=spawn.world
    /// @resolution.place source=world.player placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=world.player root=spawn.world keys=[player]

}

function name(): &readonly string {
/// @type.symbol symbol=name type=() => Borrowed<string, "managed" & "local", "readonly">

    return "player";
}
"#, r#""#);
}
