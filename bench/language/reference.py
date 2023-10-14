from typing import TYPE_CHECKING, Collection, Iterable, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import INTERP_NODE_TYPES, MNT, StatementReference
from bench.language.module import Node, ScopeNode, node_component, nproperty
from bench.utils.utils import identity

if TYPE_CHECKING:
    from bench.language.statement import IsTyped, Statement


@node_component
class HasReference(Node):
    """A reference to another statement."""

    reference: Union["Statement", StatementReference, None] = nproperty(default=None, copy=identity)

    def _clear_inner(self) -> None:
        self._set_untracked(
            "reference",
            self.reference.ck if isinstance(self.reference, Node) else self.reference,
        )

    def _interp_inner(self, scope: ScopeNode) -> None:
        if self.reference is None:
            return
        resolved = self.reference
        if not isinstance(self.reference, Node):
            resolved = scope.lookup(self.reference)
        if resolved is None:
            path = getattr(self, "py_ident", repr(self))
            self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path=path)
        elif isinstance(self.reference, str):
            # user code set a string reference, need to track change
            self.reference = resolved
        else:
            self._set_untracked("reference", resolved)

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        if isinstance(self.reference, Node):
            visitor.visit_reference(self.reference)


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


class NodeView:
    """
    A view into a (partial?) module graph, composed of multiple viewports.
    TODO @Architecture: unified node view walking :NodeViews
    This is not quite right, but we haven't figured out 'federated' tree walking yet.
     - How do we filter and level of detail across descendants and references?
     - How and when do we inline out-of-line descendants (like Records or Comments)?
        (esp. considering there may be thousands of records, need to fetch async)
     - How do we alias shadowed and anonymous nodes?
    """

    def __init__(self, scope: ScopeNode):
        self.scope = scope
        self._nodes_by_ck: dict[UUID, Node] = {}  # in order of discovery

    @property
    def nodes(self) -> Iterable[Node]:
        return reversed(self._nodes_by_ck.values())  # see reverse below

    def view_from_node(
        self,
        origin: Node | Collection[Node],
        ancestors_to: MNT,
        max_distance: int,
        exclude: set[MNT] = INTERP_NODE_TYPES,
    ) -> dict[UUID, Node]:
        """Collects the entire inline lineage including references up to max_distance"""
        origins = [origin] if isinstance(origin, Node) else list(origin)
        if not origins:
            return {}

        # walk parents
        seen_by_ck: dict[UUID, Node] = {}
        for origin in origins:
            parent = origin
            while parent is not None and parent.mnt != ancestors_to:
                seen_by_ck[parent.ck] = parent
                parent = parent.parent

        # walk descendants and referents DFS
        def _walk_node_dfs(n: Node, depth: int):
            seen_by_ck[n.ck] = n
            if depth >= max_distance:
                return
            visitor = NodeVisitor()
            n._visit_self(visitor)
            children = n._local_root_tree.get_descendants(n.ck) if n.__has_scope__ else []
            if children or visitor.references:
                # reverse so we retain original in-node order when we reverse across all
                for ref in reversed(children + list(visitor.references)):
                    if ref.ck not in seen_by_ck and ref.mnt not in exclude:
                        _walk_node_dfs(ref, depth + 1)

        for origin in origins:
            _walk_node_dfs(origin, 0)

        self._nodes_by_ck.update(seen_by_ck)
        return seen_by_ck

    async def view_records(self, nodes: Collection[Node], limit: int) -> dict[UUID, Node]:
        from bench.language.database import HasDatabase

        databases = []
        for node in nodes:
            if node.mnt == MNT.STATEMENT and HasDatabase in node._components:
                databases.append(node)
        seen_by_ck: dict[UUID, Node] = {}
        for database in databases:
            records = await database.records.limit(limit).atolist()
            for record in records:
                seen_by_ck[record.ck] = record

        self._nodes_by_ck.update(seen_by_ck)
        return seen_by_ck

    def view_from_value(
        self, value: dict, type: "IsTyped", is_output: bool = None
    ) -> dict[UUID, Node]:
        from bench.language.text import Text
        from bench.language.typing import walk_value

        seen_by_ck: dict[UUID, Node] = {}

        for n in walk_value(value, type, is_output):
            # there's definitely a more efficient way to do this
            # also see HasValue._visit_inner and :NodesAsValues
            if isinstance(n, Node):
                seen_by_ck[n.ck] = n
            elif isinstance(n, Text):
                for mention in n.mentions:
                    if isinstance(mention.reference, Node):
                        seen_by_ck[mention.reference.ck] = mention.reference

        self._nodes_by_ck.update(seen_by_ck)
        return seen_by_ck
