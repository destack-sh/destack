#
# Queries
#
from typing import (
    TYPE_CHECKING,
    cast,
    override,
)

import structlog
from opentelemetry import trace

from bench import pb2
from bench.language.core import (
    GraphData,
    Node,
    NodeType,
    QueryType,
    bittuple,
    repr_enums,
    repr_scope,
)
from bench.language.registry import CHILD_NODE_TYPES
from bench.pb2 import AnyNodeData, GraphScopeData

from .connection import Connection, GetConnection, GetResultData
from .engine import ConnectionOptions, Connector, Engine

if TYPE_CHECKING:
    from bench.language import LegacyQuery, Session

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class MemoryEngine(Engine["MemoryConnector"]):
    """A read-only Engine that reads from an in-memory graph."""

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        graph: "GraphData",
        include_removed: bool,
    ):
        super().__init__(name, scope, node_types)
        self.graph = graph
        self._include_removed = include_removed

    def __str__(self):
        return f"'{self.name}' [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}, graph={self.graph!r}]"

    @property
    def is_readonly(self) -> bool:
        return True

    @property
    def include_removed(self) -> bool:
        return self._include_removed

    async def connector(self, session: "Session"):
        return MemoryConnector(self, session)


class MemoryConnector(Connector[MemoryEngine]):
    """A read-only Connector to an in-memory graph."""

    def __init__(self, engine: "MemoryEngine", session: "Session"):
        super().__init__(engine, session)

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    async def reset(self):
        pass  # nothing to do

    @override
    async def close(self):
        pass  # nothing to do

    @override
    def _get_connection_cls(
        self, query: "LegacyQuery", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return MemoryGetConnection
        else:
            raise RuntimeError(f"unsupported memory read {query!r}")


class MemoryGetConnection[T: Node](GetConnection[MemoryConnector, T]):
    """Search an in-memory Connector."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> GetResultData:
        from bench.language import GraphData, NodeReference

        loaded_graph = self.connector.engine.graph
        visited_graph = GraphData(
            scope=self.connector.engine.scope, node_types=self.connector.engine.node_types
        )

        # get roots
        assert query._roots is not None, f"{query!r} has no roots"
        roots: list[AnyNodeData] = []
        for root_ptr in query._roots:
            root_id = str(root_ptr.id)
            if root_id in visited_graph:
                continue  # dedupe
            root = loaded_graph.get(root_id)
            if root is not None:
                roots.append(root)
                visited_graph.add(root)

        # select ancestors
        ancestor_types = query._ancestor_types or ()
        if len(ancestor_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_ancestors"):
                current_parents = roots
                while current_parents:
                    next_parents = []
                    for node in current_parents:
                        if (
                            node.parent_ptr is not None
                            and node.parent_ptr.id is not None
                            and node.parent_ptr.id not in visited_graph
                            and node.parent_ptr.node_type in ancestor_types
                        ):
                            parent = loaded_graph.get(node.parent_ptr.id)
                            assert parent is not None, f"missing parent {node.parent_ptr!r}"
                            visited_graph.add(parent)
                            next_parents.append(parent)
                    current_parents = next_parents

        # select descendants
        descendant_types = query._descendant_types or ()
        if len(descendant_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_descendants"):
                child_types_by_parent: dict[pb2.NodeType, tuple[NodeType, ...]] = {
                    cast(pb2.NodeType, node_type): tuple(
                        t for t in CHILD_NODE_TYPES[node_type] if t in descendant_types
                    )
                    for node_type in query.all_node_types
                }
                current_parents = roots
                while current_parents:
                    next_parents: list[AnyNodeData] = []
                    for node in current_parents:
                        child_types = child_types_by_parent[cast(pb2.NodeType, node.metatype)]
                        for child_type in child_types:
                            children = loaded_graph.get_descendants(node, child_type)
                            visited_graph.extend(children)
                            for child in children:
                                if loaded_graph.has_descendants(child):
                                    next_parents.append(child)
                    current_parents = next_parents

        return GetResultData(
            graph=visited_graph,
            roots_ptr=[NodeReference._ref_data_from_node_data(r) for r in roots],
            epoch=None,
            connection_token=None,
        )
