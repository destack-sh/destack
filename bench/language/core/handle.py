from typing import TYPE_CHECKING, Union

from bench.language.core.const import (
    NodeType,
)
from bench.language.core.node import BenchNode, node_
from bench.language.core.property import (
    p_node_parent,
    p_system,
)
from bench.language.core.validation import (
    SLUG_CONSTRAINT,
)
from bench.proto.wire import (
    HandleData,
)

if TYPE_CHECKING:
    from bench.language import Bench, Organization, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE, roots=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH))
class Handle(BenchNode[HandleData]):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_node_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True, constraint=SLUG_CONSTRAINT)

    @property
    def is_attached(self) -> bool:
        return self.parent is not None  # bench may not be present if it's not in a bench
