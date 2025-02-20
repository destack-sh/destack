import asyncio
from typing import TYPE_CHECKING, Callable, Mapping, Sequence, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.core import Node, NodeReference
from bench.language.core.node import TYPE_BASE_NODE_TYPES
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.utils.func import group_by

from .capture import capture
from .connection import GetConnection, SearchConnection
from .engine import WatchGetUpdate

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

if TYPE_CHECKING:
    from bench.language import TypeBaseNode


class NodeLink:
    """
    A live 'link' between some related Nodes and a Connection. Synchronizes Connection into Nodes.
    The related Nodes must be of the same type and base.
    Useful when we get a Node from one source and the need to make it live after the fact.
    """

    def __init__(self, nodes: Sequence[Node | NodeReference], base: "TypeBaseNode | None") -> None:
        assert len(nodes) > 0, f"no nodes to link for {self!r}"
        self.nodes = nodes
        self.base = base
        self.nodes_ptr: tuple[NodeReference, ...] = tuple(n.to_ref() for n in nodes)
        self.node_cls = NODE_CLASS_BY_TYPE[self.nodes_ptr[0].node_type]
        self.nodes_by_id: dict[UUID, Node] = {
            node.id: node for node in nodes if isinstance(node, Node)
        }
        self._connection: GetConnection | SearchConnection | None = None
        self._subs: list[Callable[[], None]] = []
        capture(self)

    def __str__(self):
        return f"{self.nodes_ptr} <-> {self._connection}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_active(self) -> bool:
        """Whether the link is still active."""
        return (
            self._connection is not None and self._connection.is_live and self._connection.is_open
        )

    def on_update(self, sub: Callable[[], None]):
        """Subscribe to updates from the Connection."""
        self._subs.append(sub)
        return lambda: self._subs.remove(sub)

    def _apply_update(self, update: WatchGetUpdate):
        """'Apply' the updates from a live connection to our graphs (patching nodes in place)."""
        logger.trace("link.apply_update", link=self, update=update)
        touched_our_nodes = False
        for live_node in update.updated.values():
            our_node = self.nodes_by_id.get(live_node.id)
            if our_node is not None:
                touched_our_nodes = True
                our_node._patch_from(live_node)
        if touched_our_nodes:
            for sub in self._subs:
                sub()

    async def link(self):
        """Link given Nodes to a live Connection."""
        assert self._connection is None, f"{self!r} is already linked"
        node_cls = NODE_CLASS_BY_TYPE[self.nodes_ptr[0].node_type]
        query = node_cls.select_all()
        query._base_type = self.base
        live_nodes, connection = await query.get_connection(self.nodes_ptr, live=True)
        for live_node in live_nodes:  # patch immediately
            our_node = self.nodes_by_id.get(live_node.id)
            if our_node is not None:
                our_node._link = self
                our_node._patch_from(live_node)
        connection.on_update(lambda c, u: self._apply_update(cast(WatchGetUpdate, u)))
        self._connection = connection

    def close(self):
        """Close the link (and associated connection)."""
        logger.trace("link.close", link=self)
        if self._connection is not None:
            self._connection.close()
            self._connection.detach()

    async def wait_closed(self):
        """Wait for the link to fully close."""
        if self._connection is not None:
            await self._connection.wait_closed()


async def synchronize_nodes(nodes: Sequence[Node | NodeReference]) -> Sequence[NodeLink]:
    """Link given Nodes to live Connections."""
    if not nodes:
        return []
    supergraph = nodes[0]._supergraph

    # group nodes
    nodes_ptr: tuple[NodeReference, ...] = tuple(n.to_ref() for n in nodes)
    nodes_by_id: Mapping[UUID, Node] = {node.id: node for node in nodes if isinstance(node, Node)}
    nodes_by_type_and_base = group_by(nodes_ptr, lambda node: (node.node_type, node.base_ck))

    # link them
    links: list[NodeLink] = []
    for (_, link_base_ck), link_nodes_ptr in nodes_by_type_and_base.items():
        # base
        if link_base_ck is not None:
            base = cast("TypeBaseNode | None", supergraph.get(link_base_ck))
            if base is None or base.metatype not in TYPE_BASE_NODE_TYPES:
                raise ValueError(f"missing base {link_base_ck!r} for {link_nodes_ptr!r}")
        else:
            base = None

        # nodes
        link_nodes = [
            nodes_by_id.get(cast(UUID, node_ptr.id), node_ptr) for node_ptr in link_nodes_ptr
        ]
        link = NodeLink(link_nodes, base=base)
        links.append(link)
    await asyncio.gather(*(link.link() for link in links))

    return links
