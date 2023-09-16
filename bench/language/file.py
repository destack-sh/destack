from collections import defaultdict
from dataclasses import field
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import MNT
from bench.language.module import Module, ModuleNode, ModuleVisitor, Scope, node
from bench.utils.fractional import generate_n_keys_between
from bench.utils.utils import IdentifierType, required_field, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node(mnt=MNT.File, tracked=["name"])
class File(ModuleNode, Scope):
    name: str = required_field()
    module: Optional[Module] = None
    parent: Union["File", Module] = None
    children: list[Union["File", "Statement"]] = field(default_factory=list)
    statements: list["Statement"] = field(default_factory=list)
    # index
    _statements_by_parent_id: dict[UUID | None, list["Statement"]] | None = None

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

    def _visit(self, visitor: ModuleVisitor) -> None:
        for statement in self._statements_by_parent_id.get(self.id, []):
            visitor.visit_child(statement)

    def append_statement(self, *statements: "Statement"):
        """Appends the statements to this file."""
        last_ok = self.statements[-1].order_key if self.statements else None
        oks = generate_n_keys_between(last_ok, None, len(statements))
        for ok, statement in zip(oks, statements):
            if statement.parent is not None and statement.parent != self:
                raise ValueError(f"statement {statement} belongs to {statement.parent}")
            statement.order_key = ok
            statement.parent = self
            statement.file = self
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

    def _index(self):
        """Indexes all statements in this file into the scope."""
        # per parent (incl. root = None) sort by order key
        sorted_statements = []
        self._statements_by_parent_id: dict[UUID, list[Statement]] = defaultdict(list)
        for statement in self.statements:
            self._statements_by_parent_id[statement.parent_id].append(statement)

        def walk_dfs(statement: "Statement"):
            sorted_statements.append(statement)
            children = self._statements_by_parent_id.get(statement.id)
            if children is not None:
                for child in sorted(children, key=lambda s: s.order_key):
                    walk_dfs(child)

        roots = self._statements_by_parent_id.get(self.id, [])
        for statement in sorted(roots, key=lambda s: s.order_key):
            walk_dfs(statement)

        if len(sorted_statements) != len(self.statements):
            raise RuntimeError(f"invalid order: {len(sorted_statements)} != {len(self.statements)}")
        self.statements = sorted_statements
        self.children = [s for s in self.statements if s.parent == self]

        for statement in self.statements:
            statement._index()
            self._add_child_scope(statement, by_name=statement.parent == self)

    def _interp(self, scope: "Scope"):
        for statement in self.statements:
            statement._interp(statement)
