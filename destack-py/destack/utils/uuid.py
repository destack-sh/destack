from typing import TYPE_CHECKING

try:
    if TYPE_CHECKING:
        from uuid import UUID, uuid4, uuid5
    else:
        from fastuuid import UUID, uuid4, uuid5
except ImportError:
    from uuid import UUID, uuid4, uuid5


__all__ = ["UUID", "uuid4", "uuid5"]
