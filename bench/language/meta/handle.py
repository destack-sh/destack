from typing import TYPE_CHECKING

from bench.language.core import BenchNode, NodeType, StringFormat, node_, p_system
from bench.pb2 import HandleData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(BenchNode[HandleData]):
    """A Bench @handle."""

    slug: str = p_system(30, unique=True, format=StringFormat.SLUG)

    @property
    def is_attached(self) -> bool:
        return self.parent is not None  # bench may not be present if it's not in a bench
