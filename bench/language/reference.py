import itertools
from typing import TYPE_CHECKING, Collection, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import StatementReference
from bench.language.module import Module, ModuleNode, ModuleVisitor, Scope, node

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node(tracked=["reference"])
class HasReference(ModuleNode):
    """A reference to another statement."""

    reference: Union["Statement", StatementReference, None] = None

    def _interp(self, scope: Scope) -> None:
        resolved = None
        if not isinstance(self.reference, ModuleNode):
            resolved = scope.lookup(self.reference)
        if resolved is None:
            self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
        else:
            self.reference = resolved

    def _visit(self, visitor: "ModuleVisitor") -> None:
        if isinstance(self.reference, ModuleNode):
            visitor.visit_reference(self.reference)


class ModuleView:
    def __init__(self, module: Module, origin: ModuleNode):
        super().__init__()
        self.module = module
        self.origin = origin
        self._nodes_by_distance: list[list[ModuleNode]] = []

    @property
    def nodes(self) -> Collection[ModuleNode]:
        return itertools.chain.from_iterable(self._nodes_by_distance)

    @property
    def nodes_by_distance(self) -> list[list[ModuleNode]]:
        return self._nodes_by_distance

    def collect(self) -> None:
        self._nodes_by_distance = []

        # TODO @Task: gather module view more intelligently (prevent reference jungle)
        seen: dict[UUID, ModuleNode] = {}
        child_visitor = ModuleVisitor()
        to_visit = [self.origin]
        while to_visit:
            self._nodes_by_distance.append(to_visit)
            for n in to_visit:
                seen[n.ck] = n
                n._visit(child_visitor)
            to_visit = [
                n
                for n in itertools.chain(child_visitor.subtree, child_visitor.references)
                if n.ck not in seen
            ]
