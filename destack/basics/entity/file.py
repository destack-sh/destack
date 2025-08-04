from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    TraitType,
    UInt64,
    declare_entity,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import File, Space


FILE_HASH_LENGTH = 64  # 256 bits


@declare_entity(NodeType.FILE, traits=(TraitType.RESOURCE,))
class File(Entity):
    """
    A generic File stored somewhere.
    """

    parent: Optional["Space"] = declare_property_parent()

    # meta
    mime_type: str | None = declare_property(120, is_repr=True)
    size: UInt64 | None = declare_property(122, is_repr=True)
    sha256: str | None = declare_property(123)
