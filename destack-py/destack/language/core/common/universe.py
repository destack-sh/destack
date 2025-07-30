from typing import TYPE_CHECKING, final

from destack.language.core import (
    BEGINNING_OF_TIME,
    EPSILON,
    EPSILON_EXPONENT,
    VERSION,
    Entity,
    Message,
    NodeReference,
    NodeType,
    PlatformType,
    Region,
    StructType,
    builtin_action,
    builtin_constant,
    builtin_entity,
    builtin_message,
    builtin_property,
)
from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Space, User

# pyright: reportIncompatibleVariableOverride=false

# universe and space constants
_UNIVERSE_ID = UUID(int=1)
_UNIVERSE_SPACE_ID = UUID(int=2)
_UNIVERSE_ACTOR_ID = UUID(int=10)
_UNIVERSE_CLIENT_ID = UUID(int=11)

_META_SNAPSHOT_ID = UUID(int=20)
_META_BRANCH_ID = UUID(int=21)
_HEAD_SNAPSHOT_ID = UUID(int=30)
_ROOT_BRANCH_ID = UUID(int=31)


@builtin_message(StructType.UNIVERSE_SIGNUP_REQUEST)
class UniverseSignupRequest(Message):
    name: str = builtin_property(101)
    email: str = builtin_property(102)
    password: str = builtin_property(103)


@builtin_message(StructType.UNIVERSE_SIGNUP_RESPONSE)
class UniverseSignupResponse(Message):
    user: "User" = builtin_property(101)


@builtin_message(StructType.UNIVERSE_SPAWN_REQUEST)
class UniverseSpawnRequest(Message):
    name: str = builtin_property(101)
    region: Region = builtin_property(102)
    slug: str = builtin_property(103)


@builtin_message(StructType.UNIVERSE_SPAWN_RESPONSE)
class UniverseSpawnResponse(Message):
    space: "Space" = builtin_property(101)


@builtin_entity(NodeType.UNIVERSE, is_final=True, is_singleton=True)
@final
class Universe(Entity):
    """The Destack computational universe."""

    VERSION = builtin_constant(
        1,
        value=VERSION,
        description="The current version of Destack.",
    )
    EPSILON = builtin_constant(
        2,
        value=EPSILON,
        description="The float epsilon used for floating point comparisons.",
    )
    EPISLON_EXPONENT = builtin_constant(
        3,
        value=EPSILON_EXPONENT,
        description="The exponent used for floating point comparisons.",
    )
    BEGINNING_OF_TIME = builtin_constant(
        4,
        value=BEGINNING_OF_TIME,
        description="The beginning of time. (1970-01-01T00:00:00+00:00)",
    )

    NODES = builtin_constant(
        10,
        value=lambda: list(NODE_DEFINITION_BY_TYPE.values()),
        description="All Node definitions.",
    )
    STRUCTS = builtin_constant(
        12,
        value=lambda: list(STRUCT_DEFINITION_BY_TYPE.values()),
        description="All Struct definitions.",
    )
    ENUMS = builtin_constant(
        13,
        value=lambda: list(ENUM_DEFINITION_BY_TYPE.values()),
        description="All Enum definitions.",
    )

    UNIVERSE_ID = builtin_constant(
        100,
        value=_UNIVERSE_ID,
        description="The system Universe ID.",
    )
    SPACE_ID = builtin_constant(
        101,
        value=_UNIVERSE_SPACE_ID,
        description="The system Space ID.",
    )
    SPACE = builtin_constant(
        102,
        description="The system Space.",
        value=lambda: NodeReference(
            type=NodeType.SPACE,
            id=_UNIVERSE_SPACE_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    META_SNAPSHOT_ID = builtin_constant(
        103,
        value=_META_SNAPSHOT_ID,
        description="The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    META_BRANCH_ID = builtin_constant(
        105,
        value=_META_BRANCH_ID,
        description="The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    ROOT_BRANCH_ID = builtin_constant(
        107,
        value=_ROOT_BRANCH_ID,
        description="The 'root' Branch.id, the Branch all other Branches originate from.",
    )
    HEAD_SNAPSHOT_ID = builtin_constant(
        109,
        value=_HEAD_SNAPSHOT_ID,
        description="The 'head' Snapshot.id, the current active Snapshot.",
    )

    ACTOR = builtin_constant(
        120,
        description="God Himself, the creator of the Universe.",
        value=lambda: NodeReference(
            type=NodeType.ENTITY,
            id=_UNIVERSE_ACTOR_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    CLIENT = builtin_constant(
        121,
        description="God's terminal, for when He needs to do something.",
        value=lambda: NodeReference(
            type=NodeType.CLIENT,
            id=_UNIVERSE_CLIENT_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )

    @builtin_action(100, platforms=(PlatformType.SYSTEM,))
    async def signup(self, request: "UniverseSignupRequest") -> "UniverseSignupResponse":
        """Sign up a new user."""
        ...

    @builtin_action(101, platforms=(PlatformType.SYSTEM,))
    async def spawn(self, request: "UniverseSpawnRequest") -> "UniverseSpawnResponse":
        """Create a new Space with a root Branch, meta Snapshot and head Snapshot."""
        ...
