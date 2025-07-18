from collections.abc import Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    ACTIVE_SPACE,
    Entity,
    Enum,
    EnumType,
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsScriptable,
    IsStarable,
    NodeType,
    Region,
    ValueFactory,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Database, Folder, Handle, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SPACE_STATUS)
class SpaceStatus(Enum):
    """The status of a Space"""

    CREATING = 1
    ACTIVE = 10


@builtin_node(NodeType.SPACE)
class Space(
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    IsScriptable,
    Entity,
):
    """
    A Space is the home of your personal software studio.
    """

    space: "Space" = builtin_property(
        5,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.SELF,
        description="The Space this Node is in.",
    )
    if TYPE_CHECKING:
        space_ptr: Optional[NodeReference] = None

    slug: str = builtin_property(102, is_repr=True)

    status: SpaceStatus = builtin_property(110, is_repr=True)
    handle: Optional["Handle"] = builtin_property(111)
    system_folder: Optional["Folder"] = builtin_property(112, description="The system Folder.")
    home_folder: Optional["Folder"] = builtin_property(113, description="The home Folder.")
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        root_folder_ptr: Optional[NodeReference] = None
        home_folder_ptr: Optional[NodeReference] = None

    # infra
    region: Region = builtin_property(120)
    galaxy_name: str | None = builtin_property(121)  # -> Galaxy?
    database: Optional["Database"] = builtin_property(122)
    # search, analytics, vault, cache, ...
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None

    @contextmanager
    def active(self: "Space") -> Generator["Space", None, None]:
        """Set this Space as the active Space."""
        token = ACTIVE_SPACE.set(self)
        try:
            ACTIVE_SPACE.set(self)
            yield self
        finally:
            ACTIVE_SPACE.reset(token)
