from typing import TYPE_CHECKING, final

from destack.language.core import (
    BEGINNING_OF_TIME,
    EPSILON,
    EPSILON_EXPONENT,
    VERSION,
    Entity,
    NodeReference,
    NodeType,
    PlatformType,
    Region,
    builtin_action,
    builtin_constant,
    builtin_node,
)
from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import User
    from destack.language.core.common.space import CreateSpaceResult

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


@builtin_node(NodeType.UNIVERSE, is_final=True, is_singleton=True)
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
    async def signup_user(
        self,
        id: UUID | None,
        name: str,
        email: str,
        password: str,
    ) -> "User":
        """Sign up a new user."""
        ...

    @builtin_action(101, platforms=(PlatformType.SYSTEM,))
    async def create_space(
        self,
        name: str,
        region: Region,
        slug: str,
        owned_by: "Entity | NodeReference",
    ) -> "CreateSpaceResult":
        """Create a new Space with a root Branch, meta Snapshot and head Snapshot."""
        ...
