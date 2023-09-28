import itertools
from typing import TYPE_CHECKING, Collection, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import StatementReference
from bench.language.module import (
    Module,
    ModuleNode,
    NodeVisitor,
    ScopeNode,
    node_component,
    nproperty,
)

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node_component
class HasReference(ModuleNode):
    """A reference to another statement."""

    reference: Union["Statement", StatementReference, None] = nproperty(default=None)

    def _clear_inner(self) -> None:
        self.reference = (
            self.reference.ck if isinstance(self.reference, ModuleNode) else self.reference
        )

    def _interp_inner(self, scope: ScopeNode) -> None:
        if self.reference is None:
            return
        resolved = self.reference
        if not isinstance(self.reference, ModuleNode):
            resolved = scope.lookup(self.reference)
        if resolved is None:
            self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path=self.py_ident)
        else:
            self.reference = resolved

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        if isinstance(self.reference, ModuleNode):
            visitor.visit_reference(self.reference)


class ModuleView:
    def __init__(self, module: Module, origin: ModuleNode):
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
        child_visitor = NodeVisitor()
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
