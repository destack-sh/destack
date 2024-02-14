from typing import TYPE_CHECKING, Collection, Iterable
from uuid import UUID

from bench.language.const import NodeType, SortOp, StructType
from bench.language.expression import S
from bench.language.node import Node, Struct, p_runtime, struct

if TYPE_CHECKING:
    pass


class NodeVisitor:
    def __init__(self):
        self._reference_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        return f"{len(self._reference_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {str(self)}>"

    @property
    def references(self) -> Collection["Node"]:
        return self._reference_by_ck.values()

    def visit_reference(self, node: "Node"):
        self._reference_by_ck[node.ck] = node


@struct(StructType.PROJECTION)
class Projection(Struct):
    """
    A projection into the graph.
    NOTE 'Projecting' is not quite right / complete yet, consider:
     - How do we filter and LoD this?
     - When do we inline out-of-line descendants (like Comments or local Records)?
        (esp. considering some out-of-line nodes would need to be fetched async)
     - How do we alias shadowed and anonymous nodes?
     - How do we make projections reproducible and inspectable in the editor?
    """

    _nodes_by_ck: dict[UUID, Node] = p_runtime(default_factory=dict)

    @property
    def nodes(self) -> Iterable[Node]:
        return self._nodes_by_ck.values()

    def project_node(
        self, origin: Node | Collection[Node], ancestors_up_to: NodeType, max_distance: int
    ) -> dict[UUID, Node]:
        """Collects the entire inline lineage including references up to max_distance"""
        origins = (origin,) if isinstance(origin, Node) else tuple(origin)
        if not origins:
            return {}

        # walk parents
        seen_by_ck: dict[UUID, Node] = {}
        for origin in origins:
            parent = origin
            while parent is not None and parent.metatype != ancestors_up_to:
                seen_by_ck[parent.ck] = parent
                parent = parent.parent

        visitor = NodeVisitor()

        def _walk_node_descendants_dfs(n: Node, depth: int) -> None:
            seen_by_ck[n.ck] = n
            if depth >= max_distance:
                return
            n._visit_self(visitor)
            for child in n._root_graph.iter_descendants(n):
                _walk_node_descendants_dfs(child, depth + 1)

        # walk descendants
        for origin in origins:
            _walk_node_descendants_dfs(origin, 0)
        # walk references
        for i in range(max_distance):
            new_references = tuple(ref for ref in visitor.references if ref.ck not in seen_by_ck)
            if not new_references:
                break
            for ref in new_references:
                seen_by_ck[ref.ck] = ref
                _walk_node_descendants_dfs(ref, i + 1)

        self._nodes_by_ck.update(seen_by_ck)
        return seen_by_ck

    async def project_records(self, nodes: Collection[Node], limit: int) -> dict[UUID, Node]:
        from bench.language.database import HasDatabase

        seen_by_ck: dict[UUID, Node] = {}
        for node in nodes:
            if node.metatype != NodeType.BLOCK or HasDatabase not in node._components:
                continue
            # sort by ck for consistency
            records = (
                await node.records.sort(S(SortOp.ASCENDING, field_key="ck")).first(limit).tolist()
            )
            for record in records:
                seen_by_ck[record.ck] = record

        self._nodes_by_ck.update(seen_by_ck)
        return seen_by_ck
