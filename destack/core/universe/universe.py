from datetime import UTC, datetime
from typing import TYPE_CHECKING, final

from destack.core import (
    EPSILON,
    EPSILON_EXPONENT,
    VERSION,
    Entity,
    Message,
    NodeReference,
    NodeType,
    Region,
    RuntimePlatform,
    StructType,
    declare_action,
    declare_constant,
    declare_entity,
    declare_message,
    declare_property,
)
from destack.registry import (
    ENUM_DEFINITION_BY_TYPE,
    HANDLE_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

from ..utils.uuid import UUID

if TYPE_CHECKING:
    from destack import Space, User

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


@declare_message(StructType.UNIVERSE_SIGNUP_REQUEST)
class UniverseSignupRequest(Message):
    name: str = declare_property(101)
    email: str = declare_property(102)
    password: str = declare_property(103)


@declare_message(StructType.UNIVERSE_SIGNUP_RESPONSE)
class UniverseSignupResponse(Message):
    user: "User" = declare_property(101)


@declare_message(StructType.UNIVERSE_SPAWN_REQUEST)
class UniverseSpawnRequest(Message):
    name: str = declare_property(101)
    region: Region = declare_property(102)
    slug: str = declare_property(103)


@declare_message(StructType.UNIVERSE_SPAWN_RESPONSE)
class UniverseSpawnResponse(Message):
    space: "Space" = declare_property(101)


@declare_entity(NodeType.UNIVERSE, is_final=True, is_singleton=True)
@final
class Universe(Entity):
    """The Destack computational universe."""

    VERSION = declare_constant(
        1,
        value=VERSION,
        description="The current version of Destack.",
    )
    EPSILON = declare_constant(
        2,
        value=EPSILON,
        description="The float epsilon used for floating point comparisons.",
    )
    EPISLON_EXPONENT = declare_constant(
        3,
        value=EPSILON_EXPONENT,
        description="The exponent of the epsilon used for floating point comparisons.",
    )
    BEGINNING_OF_DATETIME = declare_constant(
        4,
        value=datetime(1, 1, 1, tzinfo=UTC),
        description="The beginning of time (1 AD, 00:00:00 UTC).",
    )

    NODES = declare_constant(
        10,
        value=lambda: list(NODE_DEFINITION_BY_TYPE.values()),
        description="All Node definitions.",
    )
    STRUCTS = declare_constant(
        12,
        value=lambda: list(STRUCT_DEFINITION_BY_TYPE.values()),
        description="All Struct definitions.",
    )
    HANDLES = declare_constant(
        13,
        value=lambda: list(HANDLE_DEFINITION_BY_TYPE.values()),
        description="All Handle definitions.",
    )
    ENUMS = declare_constant(
        14,
        value=lambda: list(ENUM_DEFINITION_BY_TYPE.values()),
        description="All Enum definitions.",
    )

    UNIVERSE_ID = declare_constant(
        100,
        value=_UNIVERSE_ID,
        description="The system Universe ID.",
    )
    SPACE_ID = declare_constant(
        101,
        value=_UNIVERSE_SPACE_ID,
        description="The system Space ID.",
    )
    SPACE = declare_constant(
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
    META_SNAPSHOT_ID = declare_constant(
        103,
        value=_META_SNAPSHOT_ID,
        description="The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    META_BRANCH_ID = declare_constant(
        105,
        value=_META_BRANCH_ID,
        description="The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    ROOT_BRANCH_ID = declare_constant(
        107,
        value=_ROOT_BRANCH_ID,
        description="The 'root' Branch.id, the Branch all other Branches originate from.",
    )
    HEAD_SNAPSHOT_ID = declare_constant(
        109,
        value=_HEAD_SNAPSHOT_ID,
        description="The 'head' Snapshot.id, the current active Snapshot.",
    )

    ACTOR = declare_constant(
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
    CLIENT = declare_constant(
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

    @declare_action(100, platforms=(RuntimePlatform.SYSTEM,))
    async def signup(self, request: "UniverseSignupRequest") -> "UniverseSignupResponse":
        """Sign up a new user."""
        ...

    @declare_action(101, platforms=(RuntimePlatform.SYSTEM,))
    async def spawn(self, request: "UniverseSpawnRequest") -> "UniverseSpawnResponse":
        """Create a new Space with a root Branch, meta Snapshot and head Snapshot."""
        ...
