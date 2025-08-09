from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    TraitType,
    UInt64,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import File


FILE_HASH_LENGTH = 64  # 256 bits


@declare_entity(NodeType.FILE, traits=(TraitType.RESOURCE,))
class File(Entity):
    """
    A generic File is a remote asset stored somewhere.
    """

    # meta
    mime_type: str | None = declare_property(120, is_repr=True)
    size: UInt64 | None = declare_property(122, is_repr=True)
    sha256: str | None = declare_property(123)
