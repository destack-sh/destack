from collections import defaultdict
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import MNT
from bench.language.module import (
    Module,
    ModuleNode,
    NodeList,
    NodeVisitor,
    NRel,
    Scope,
    nchildren,
    node,
    nproperty,
)
from bench.utils.fractional import generate_n_keys_between
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node(mnt=MNT.File, tracked=["name"])
class File(ModuleNode, Scope):
    name: str = nproperty()
    module: Optional[Module] = None
    parent: Union["File", Module] = None

    children: NodeList[Union["File", "Statement"]] = nchildren(MNT.File, NRel.INLINE)
    statements: NodeList["Statement"] = nchildren(MNT.Statement, NRel.INLINE | NRel.FLAT)

    def __post_init__(self):
        super().__post_init__()
        if self.parent is None:
            self.parent = self.module

    def __str__(self):
        return f"{self.path} '{self.name}' ({len(self.statements)} statements)"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def path(self) -> str:
        if isinstance(self.parent, Module):
            return f"{self.parent.name}.{self.py_ident}"
        elif isinstance(self.parent, File):
            return f"{self.parent.path}.{self.py_ident}"
        else:
            return self.py_ident  # detached file

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.PATH)

    def _visit(self, visitor: NodeVisitor) -> None:
        for statement in self._statements_by_parent_id.get(self.id, []):
            visitor.visit_child(statement)

    def append_statement(self, *statements: "Statement"):
        """Appends the statements to this file."""
        # nocheckin: replace append_statement with NodeList.append
        last_ok = self.statements[-1].order_key if self.statements else None
        oks = generate_n_keys_between(last_ok, None, len(statements))
        for ok, statement in zip(oks, statements):
            if statement.parent is not None and statement.parent != self:
                raise ValueError(f"statement {statement} belongs to {statement.parent}")
            statement.order_key = ok
            statement.parent = self
            statement.file = self
            self.module._on_added(statement)
            statement._index()
            for descendant in statement.walk_descendants():
                descendant.file = self
                self.statements.append(descendant)
                self.module._on_added(descendant)

    append = append_statement  # alias for File

    def _assign_oks(self):
        for statements in self._statements_by_parent_id.values():
            oks = generate_n_keys_between(None, None, len(statements))
            for ok, statement in zip(oks, statements):
                statement.order_key = ok

    def _clear(self):
        """Resets this scope and all child scopes."""
        super()._clear()
        for statement in self.statements:
            statement._clear()

    def _interp_inner(self, scope: "Scope"):
        for statement in self.statements:
            statement._interp(statement)
