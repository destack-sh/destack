from datetime import UTC, datetime
from typing import TYPE_CHECKING, final

from ..builtin import (
    EPSILON,
    EPSILON_EXPONENT,
    UUID,
    VERSION,
    ActionType,
    Entity,
    Float64,
    Message,
    NodeType,
    Region,
    RuntimePlatform,
    String,
    StructType,
    declare_action,
    declare_constant,
    declare_entity,
    declare_message,
    declare_property,
)
from ..common import NodeReference, infer_type
from ..definition import SchemaDefinition

if TYPE_CHECKING:
    from destack import Space, User


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


@declare_entity(
    NodeType.UNIVERSE,
    is_final=True,
    is_singleton=True,
)
@final
class Universe(Entity):
    """The Destack computational universe."""

    VERSION: String = declare_constant(
        1,
        value=VERSION,
        description="The current version of Destack.",
    )
    EPSILON: Float64 = declare_constant(
        2,
        value=EPSILON,
        description="The float epsilon used for floating point comparisons.",
    )
    EPISLON_EXPONENT: int = declare_constant(
        3,
        value=EPSILON_EXPONENT,
        description="The exponent of the epsilon used for floating point comparisons.",
    )
    BEGINNING_OF_DATETIME: datetime = declare_constant(
        4,
        value=datetime(1, 1, 1, tzinfo=UTC),
        description="The beginning of time (1 AD, 00:00:00 UTC).",
    )

    SCHEMA: SchemaDefinition = declare_constant(
        10,
        # NOTE: Universe.SCHEMA is set later to prevent circular references in schema
        #  (Universe.SCHEMA is a constant inside the NodeDefinition for Universe)
        value=None,
        type=infer_type(SchemaDefinition),
    )

    UNIVERSE_ID: UUID = declare_constant(
        20,
        value=_UNIVERSE_ID,
        description="The system Universe ID.",
    )
    SPACE_ID: UUID = declare_constant(
        21,
        value=_UNIVERSE_SPACE_ID,
        description="The system Space ID.",
    )
    SPACE: NodeReference = declare_constant(
        22,
        description="The system Space.",
        value=lambda: NodeReference(
            type=NodeType.SPACE,
            id=_UNIVERSE_SPACE_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    META_SNAPSHOT_ID: UUID = declare_constant(
        23,
        value=_META_SNAPSHOT_ID,
        description="The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    META_BRANCH_ID: UUID = declare_constant(
        25,
        value=_META_BRANCH_ID,
        description="The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    ROOT_BRANCH_ID: UUID = declare_constant(
        27,
        value=_ROOT_BRANCH_ID,
        description="The 'root' Branch.id, the Branch all other Branches originate from.",
    )
    HEAD_SNAPSHOT_ID: UUID = declare_constant(
        29,
        value=_HEAD_SNAPSHOT_ID,
        description="The 'head' Snapshot.id, the current active Snapshot.",
    )

    GOD: NodeReference = declare_constant(
        40,
        description="God Himself, the creator of the Universe.",
        value=lambda: NodeReference(
            type=NodeType.ENTITY,
            id=_UNIVERSE_ACTOR_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    GOD_HANDSET: NodeReference = declare_constant(
        41,
        description="God's terminal, for when He needs to do something.",
        value=lambda: NodeReference(
            type=NodeType.CLIENT,
            id=_UNIVERSE_CLIENT_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )

    @declare_action(
        100,
        type=ActionType.UNARY_IN_UNARY_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def signup(self, request: "UniverseSignupRequest") -> "UniverseSignupResponse":
        """Sign up a new user."""
        raise NotImplementedError

    @declare_action(
        101,
        type=ActionType.UNARY_IN_UNARY_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def spawn(self, request: "UniverseSpawnRequest") -> "UniverseSpawnResponse":
        """Create a new Space with a root Branch, meta Snapshot and head Snapshot."""
        raise NotImplementedError
