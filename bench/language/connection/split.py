#
# Queries
#
from typing import (
    TYPE_CHECKING,
    Collection,
    override,
)

import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language.connection.memory import MemoryEngine
from bench.language.core import (
    BenchError,
    C,
    ConditionalType,
    GraphData,
    Node,
    NodeType,
    QueryType,
    repr_scope,
)
from bench.language.registry import CHILD_NODE_TYPES, DESCENDANT_NODE_TYPES, NODE_CLASS_BY_TYPE
from bench.pb2 import BenchData, GraphScopeData
from bench.utils.func import group_by

from .connection import Connection, GetConnection, SearchConnection
from .engine import (
    ConnectionOptions,
    Connector,
    Engine,
    GetOptions,
    GetResultData,
    NullEngine,
    SearchOptions,
    SearchResultData,
    scope_includes,
)

if TYPE_CHECKING:
    from bench.language import LegacyQuery

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Architecture: Engines/Connectors/Connections are an annoying mess :RichGraph


class SplitConnector(Connector[NullEngine]):
    """A read-only connector splits queries across connectors."""

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
            return SplitGetConnection
        elif query._type == QueryType.SEARCH:
            return SplitSearchConnection
        else:
            raise RuntimeError(f"unsupported split read {query!r}")


class SplitConnection(Connection):
    def _get_best_match_engine(
        self,
        scope: GraphScopeData,
        required_types: set[NodeType],
        match_types: set[NodeType],
        *,
        is_readonly: bool,
        include_removed: bool,
        include_memory: bool,
    ) -> tuple[Engine, set[NodeType]]:
        """Get the engine with best coverage of required node types from candidates."""
        candidate_engines = [
            engine
            for engine in self.session._engines
            if (
                (is_readonly or not engine.is_readonly)
                and (not include_removed or engine.include_removed)
                and (include_memory or not isinstance(engine, MemoryEngine))
                and scope_includes(engine.scope, scope)
                and any(t in engine.node_types for t in required_types)
            )
        ]

        if not candidate_engines:
            types_str = "|".join(t.bench_name for t in required_types)
            raise BenchError(
                f"no engine for [scope={repr_scope(scope)}, node_types={types_str}] in {self!r}"
            )

        # Find engine with most overlap between required types and candidate types
        best_engine = max(
            candidate_engines,
            key=lambda e: len(set(e.node_types) & match_types),
        )
        covered_types = set(best_engine.node_types) & match_types
        return best_engine, covered_types

    async def _read_descendants(
        self,
        scope: GraphScopeData,
        combined_graph: GraphData,
        remaining_types: set[NodeType],
        query: "LegacyQuery",
    ) -> set[NodeType]:
        """Read descendants for nodes in the graph, returns covered types."""
        from bench.language import LegacyQuery

        engine, covered_types = self._get_best_match_engine(
            scope,
            remaining_types,
            remaining_types,
            is_readonly=True,
            include_removed=query.include_removed,
            include_memory=query.include_memory,
        )
        connector = await self.session._get_connector(engine)

        # descend into potential parents (for potential children)
        nodes_by_type = group_by(combined_graph.nodes, lambda n: NodeType(n.metatype)).items()
        for parent_type, parents in nodes_by_type:
            # get valid child types for this parent from covered types
            child_types = set(CHILD_NODE_TYPES[parent_type]) & covered_types
            for child_type in child_types:
                # get parents
                parent_ids = [n.id for n in parents if n.id]
                if not parent_ids:
                    continue

                # get children
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                descendant_query = LegacyQuery(
                    type=QueryType.SEARCH,
                    node_type=child_type,
                    filter=C(
                        ConditionalType.IN,
                        property=child_cls.get_property("parent_id"),
                        value=parent_ids,
                    ),
                    descendant_types=list(set(DESCENDANT_NODE_TYPES[child_type]) & covered_types),
                    include_removed=query.include_removed,
                    select=query._select,
                )
                descendant_connection = await connector.search(
                    descendant_query,
                    SearchOptions(live=False, mode="packed", count=False),
                )
                combined_graph.extend(descendant_connection.result_data.graph.nodes)

        return covered_types

    async def _read_ancestors(
        self,
        scope: GraphScopeData,
        combined_graph: GraphData,
        remaining_types: set[NodeType],
        query: "LegacyQuery",
    ) -> set[NodeType]:
        """Read ancestors for roots in the graph, returns covered types."""

        from bench.language import LegacyQuery, NodeReference
        from bench.proto import wiring

        # get parent references from roots
        inner_roots = combined_graph.find_roots()
        inner_roots_parents_by_id = {
            n.parent_ptr.id: n.parent_ptr for n in inner_roots if n.parent_ptr.id
        }
        inner_roots_types = {NodeType(n.node_type) for n in inner_roots_parents_by_id.values()}
        next_ancestor_types = inner_roots_types.intersection(remaining_types)
        if not inner_roots_parents_by_id or not next_ancestor_types:
            return set()

        # get best engine for remaining ancestors
        engine, covered_types = self._get_best_match_engine(
            scope,
            next_ancestor_types,
            next_ancestor_types,
            is_readonly=True,
            include_removed=query.include_removed,
            include_memory=query.include_memory,
        )
        connector = await self.session._get_connector(engine)

        # get parents by type
        inner_roots_parents_by_type = group_by(
            inner_roots_parents_by_id.values(), lambda n: NodeType(n.node_type)
        ).items()
        for parent_type, parents in inner_roots_parents_by_type:
            if parent_type not in covered_types:
                continue

            ancestor_query = LegacyQuery(
                type=QueryType.GET,
                node_type=parent_type,
                roots=[
                    wiring.unpack_builtin_object(p, supergraph=None, expect=NodeReference)
                    for p in parents
                ],
                ancestor_types=list(covered_types),
                include_removed=query.include_removed,
                select=query._select,
            )
            ancestor_connection = await connector.get(ancestor_query, self.options)
            combined_graph.extend(ancestor_connection.result_data.graph.nodes)

        return covered_types

    async def _do_read_remainder(
        self,
        query: "LegacyQuery",
        initial_result: GetResultData | SearchResultData,
        initial_types: Collection[NodeType],
    ) -> GraphData:
        """Fetch the surrounding ancestor/descendant nodes for a split Query."""

        combined_graph = GraphData(
            scope=self.scope,
            node_types=tuple(query.all_node_types),
            nodes=initial_result.graph.nodes,
        )
        remaining_ancestors = set(query._ancestor_types or ()) - set(initial_types)
        remaining_descendants = set(query._descendant_types or ()) - set(initial_types)

        # read until there is nothing more to read
        while remaining_ancestors or remaining_descendants:
            if not combined_graph.find_roots():
                break

            # get descendants
            if remaining_descendants:
                bench = first((n for n in combined_graph.nodes if isinstance(n, BenchData)), None)
                descendants_scope = GraphScopeData(bench_id=bench.id) if bench else self.scope
                covered = await self._read_descendants(
                    descendants_scope, combined_graph, remaining_descendants, query
                )
                remaining_descendants -= covered

            # get ancestors
            if remaining_ancestors:
                covered = await self._read_ancestors(
                    self.scope, combined_graph, remaining_ancestors, query
                )
                if not covered:
                    break  # no more parents to traverse
                remaining_ancestors -= covered

        return combined_graph


class SplitSearchConnection[T: Node](SearchConnection[SplitConnector, T], SplitConnection):
    """Search across multiple connections."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> SearchResultData:
        # search engine for initial query
        engine, covered_types = self._get_best_match_engine(
            self.scope,
            {query._node_type},
            set(query.all_node_types),
            is_readonly=True,
            include_removed=query.include_removed,
            include_memory=query.include_memory,
        )
        connector = await self.session._get_connector(engine)
        connection = await connector.search(
            query.trim_to(covered_types),
            SearchOptions(live=False, mode="packed", count=self.options.count),
        )
        result = connection.result_data
        if not query._ancestor_types and not query._descendant_types:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, covered_types)
        combined_result = SearchResultData(
            graph=combined_graph,
            roots=result.roots,
            roots_ptr=result.roots_ptr,
            total=result.total,
            epoch=result.epoch,
            connection_token=result.connection_token,
        )
        return combined_result


class SplitGetConnection[T: Node](GetConnection[SplitConnector, T], SplitConnection):
    """Get across multiple connections."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> GetResultData:
        # get engine for initial query
        engine, covered_types = self._get_best_match_engine(
            self.scope,
            {query._node_type},
            set(query.all_node_types),
            is_readonly=True,
            include_removed=query.include_removed,
            include_memory=query.include_memory,
        )
        connector = await self.session._get_connector(engine)
        connection = await connector.get(
            query.trim_to(covered_types), GetOptions(live=False, mode="packed")
        )
        result = connection.result_data
        if not query._ancestor_types and not query._descendant_types:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, covered_types)
        combined_result = GetResultData(
            graph=combined_graph,
            roots_ptr=result.roots_ptr,
            epoch=result.epoch,
            connection_token=result.connection_token,
        )
        return combined_result
