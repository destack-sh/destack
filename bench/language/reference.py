import itertools
from typing import TYPE_CHECKING, Collection, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import StatementReference
from bench.language.module import Module, Node, NodeVisitor, ScopeNode, node_component, nproperty
from bench.utils.utils import identity

if TYPE_CHECKING:
    from bench.language.statement import Statement


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


class ModuleView:
    def __init__(self, module: Module, origin: Node):
        self.module = module
        self.origin = origin
        self._nodes_by_distance: list[list[Node]] = []

    @property
    def nodes(self) -> Collection[Node]:
        return itertools.chain.from_iterable(self._nodes_by_distance)

    @property
    def nodes_by_distance(self) -> list[list[Node]]:
        return self._nodes_by_distance

    def collect(self) -> None:
        self._nodes_by_distance = []

        # TODO @Task: gather module view more intelligently (prevent reference jungle)
        seen: dict[UUID, Node] = {}
        child_visitor = NodeVisitor()
        to_visit = [self.origin]
        while to_visit:
            self._nodes_by_distance.append(to_visit)
            for n in to_visit:
                seen[n.ck] = n
                n._visit_self(child_visitor)
            to_visit = [
                n
                for n in itertools.chain(child_visitor.subtree, child_visitor.references)
                if n.ck not in seen
            ]
