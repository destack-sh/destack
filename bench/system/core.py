import abc
from dataclasses import dataclass
from typing import TYPE_CHECKING, ClassVar

from bench.language import Bench, Node, NodeType
from bench.proto.wire import EditData
from bench.utils.func import bittuple
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    pass


@dataclass(slots=True)
class GraphDiff[T: Node]:
    """A simplified diff of edited Nodes. Here archive/soft-delete => remove."""

    edits: list[EditData]
    edited_types: bittuple[NodeType]
    added: list[T]
    updated: list[T]
    removed: list[T]

    def __str__(self):
        return f"added={self.added!r}, updated={self.updated!r}, removed={self.removed!r}"

    def __repr__(self):
        return f"<GraphDiff {self!s}>"

    def trim_to(self, node_types: bittuple[NodeType]) -> "GraphDiff[T]":
        """Trims the diff to only include the given node types."""
        return GraphDiff(
            edits=self.edits,
            edited_types=self.edited_types & node_types,
            added=[node for node in self.added if node.metatype in node_types],
            updated=[node for node in self.updated if node.metatype in node_types],
            removed=[node for node in self.removed if node.metatype in node_types],
        )


class HostPlugin[T: Node](abc.ABC):
    """A plugin on the Host system of a Bench."""

    node_types: ClassVar[bittuple[NodeType]]

    def __init__(self, bench: "Bench"):
        self._bench = bench

    def __str__(self) -> str:
        return ""

    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str} in '{self._bench.slug}'>"
        else:
            return f"<{self.__class__.__name__} in '{self._bench.slug}'>"

    async def start(self, tasks: TaskManager) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    def on_graph_commit(self, diff: GraphDiff[T]) -> None:
        pass
