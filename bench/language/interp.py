from __future__ import annotations

import enum
import re
import typing
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field
from itertools import chain
from typing import Callable
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.issue import Error, IssueType
from bench.language.type import (
    Code,
    Dataset,
    File,
    Module,
    Statement,
    StatementPath,
    StatementType,
    Symbol,
    SymbolType,
    Task,
    Type,
    TypeFlag,
    TypeNode,
    deepcopy_types,
    parse_statement_path,
    Requirement,
    HasType,
)
from bench.runtime.server.lsp import parse_code

logger = structlog.get_logger(__name__)

StmT = StatementType
SymT = SymbolType

SymbolT = typing.TypeVar("SymbolT", bound=Symbol)


def raise_error(error: ValueError):
    raise error


def ignore_error(*args, **kwargs):
    pass


ErrorT = typing.TypeVar("ErrorT", bound=ValueError)


class ErrorCollector(typing.Generic[ErrorT]):
    def __init__(self, on_error: Callable[[ErrorT], None] | None = None):
        self.on_error = on_error
        self.errors: list[ErrorT] = []

    def __call__(self, error: ErrorT):
        self.errors.append(error)
        if self.on_error:
            self.on_error(error)


IT = IssueType


def ignore_module_lookup(*args, **kwargs):
    return None


def lookup_in_error(*args, **kwargs):
    raise NotImplementedError("external module lookup disabled")


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


LookupFunc = Callable[
    [Requirement | None, StatementPath | UUID, LookupBy], typing.Union["Scope", None]
]


@dataclass(repr=False, slots=True)
class Scope:
    """A scope in which statements are defined. May be at file- or statement-level."""

    name: str
    id: UUID  # id from file or statement
    parent: Scope | None
    file: File
    statement: Statement | None
    statements: OrderedDict[str, Statement] = field(default_factory=OrderedDict)
    symbols: OrderedDict[str, Symbol] = field(default_factory=OrderedDict)
    names_by_identifier: dict[str, str] = field(default_factory=dict)

    def __str__(self):
        return f"{self.name} ({'file' if self.statement is None else 'statement'})"

    def __repr__(self):
        return f"<Scope {self}>"

    def add_statement(self, statement: Statement):
        """Adds a statement to this scope (ignoring duplicates)."""
        self.statements[statement.name] = statement
        self.names_by_identifier[statement.ident] = statement.name

    def add_symbol(self, symbol: SymbolT):
        """Adds a symbol to this scope (ignoring duplicates)."""
        self.symbols[symbol.name] = symbol
        self.names_by_identifier[symbol.ident] = symbol.name

    @property
    def module_id(self):
        return self.file.module.id

    def lookup_statement(
        self, name: str, by: LookupBy, exclude: Statement | None = None
    ) -> Statement | None:
        """Lookup the statement recursively in this scope and its parents."""
        if by == LookupBy.Name:
            if name in self.statements and self.statements[name] is not exclude:
                return self.statements[name]
        elif by == LookupBy.PyIdent:
            if name in self.names_by_identifier:
                name = self.names_by_identifier[name]
                if name in self.statements and self.statements[name] is not exclude:
                    return self.statements[name]
        else:
            raise ValueError(f"unexpected lookup type: {by}")
        if self.parent is not None:
            return self.parent.lookup_statement(name, by=by, exclude=exclude)
        return None

    @property
    def proper_statements(self) -> list[Statement]:
        return [s for s in self.statements.values() if not s.is_argument and not s.is_parameter]

    @property
    def proper_symbols(self) -> list[Symbol]:
        return [self.symbols[s.name] for s in self.proper_statements if s.name in self.symbols]


@dataclass(repr=False)
class ModuleIndex:
    """The index into a module's resolved and interpreted symbols and statements."""

    module: Module
    requirements_by_name: dict[str, Requirement] = field(default_factory=dict)
    files: dict[UUID, File] = field(default_factory=OrderedDict)
    statements: dict[UUID, Statement] = field(default_factory=OrderedDict)
    interpreted: bool = False
    scopes: OrderedDict[UUID, Scope] = field(default_factory=OrderedDict)
    scopes_by_name: OrderedDict[str, Scope] = field(default_factory=OrderedDict)
    scopes_by_ident: OrderedDict[UUID, Scope] = field(default_factory=OrderedDict)
    symbols: dict[UUID, Symbol] = field(default_factory=OrderedDict)

    def __str__(self):
        statements_str = f"statements={len(self.statements)}"
        requirements_str = f"requirements={len(self.requirements_by_name)}"
        symbols_str = f"symbols={len(self.symbols)}" if self.interpreted else "<not interpreted>"
        return f"index for {self.module} ({statements_str}, {requirements_str}, {symbols_str})"

    def __repr__(self):
        return f"<ModuleIndex {self.module}>"

    def add_scope(self, scope: Scope, anonymous: bool = False) -> None:
        self.scopes[scope.id] = scope
        if not anonymous:
            self.scopes_by_name[scope.name] = scope

    def import_scope(self, scope: Scope) -> None:
        # only make imported scopes and their contents available by id (to avoid name collisions)
        self.scopes[scope.id] = scope
        if scope.statement is not None:
            self.statements[scope.statement.id] = scope.statement

    def get_statement(self, path: StatementPath | str, by: LookupBy) -> Statement | None:
        if isinstance(path, str):
            path = parse_statement_path(path)
        scope = self.scopes_by_name.get(path.path[1:])  # skip initial dot
        if scope is None:
            return None
        return scope.statements.get(path.name)

    def get_scope(self, path: StatementPath | str, by: LookupBy) -> Scope | None:
        statement = self.get_statement(path, by=by)
        if statement is None:
            return None
        return self.scopes.get(statement.id)

    def symbols_of_type(self, symbol_t: typing.Type[SymbolT]) -> list[SymbolT]:
        return [s for s in self.symbols.values() if isinstance(s, symbol_t)]

    def symbol_by_name(
        self,
        name: str,
        symbol_t: typing.Type[SymbolT] | None = None,
        filter: Callable[[SymbolT], bool] = None,
    ) -> SymbolT:
        matching_symbols = [
            s
            for s in self.symbols.values()
            if s.name == name
            and s.is_definition
            and s.is_root
            and (symbol_t is None or isinstance(s, symbol_t))
            and (filter is None or filter(s))
        ]
        if len(matching_symbols) == 1:
            return matching_symbols[0]
        elif len(matching_symbols) > 1:
            raise KeyError(f"multiple symbols with name {name} found")
        else:
            raise KeyError(f"no symbol {name} found")

    def symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: typing.Type[SymbolT] | None = None,
        filter: Callable[[SymbolT], bool] | None = None,  # ignored if path is UUID
    ) -> SymbolT:
        if not self.interpreted:
            raise RuntimeError(f"module index is not interpreted: {self}")
        if isinstance(path, UUID):
            return self.symbol_by_id(path, symbol_t=symbol_t)
        elif isinstance(path, str):
            # usually str is a statement path, but we also allow plain names for convenience
            if ":" not in path:  # try to lookup definition at root by name
                return self.symbol_by_name(path, symbol_t=symbol_t, filter=filter)
            path = parse_statement_path(path)
        scope = self.scopes_by_name.get(path.path[1:])  # skip initial dot
        if scope is None:
            raise KeyError(f"no scope found for path {path.path}")
        symbol = scope.symbols.get(path.name)
        if symbol is None:
            raise KeyError(f"no symbol {path.name} found in {scope}")
        if symbol_t is not None and not isinstance(symbol, symbol_t):
            raise TypeError(f"symbol {symbol} is not of type {symbol_t}")
        if filter is not None and not filter(symbol):
            raise KeyError(f"symbol {symbol} does not match filter")
        return symbol

    def get_symbol(
        self, path: StatementPath | UUID | str, symbol_t: typing.Type[SymbolT] | None = None
    ) -> SymbolT | None:
        try:
            return self.symbol(path, symbol_t=symbol_t)
        except KeyError:
            return None

    def symbol_by_id(
        self, symbol_id: UUID, symbol_t: typing.Type[SymbolT] | None = None
    ) -> SymbolT:
        return self.get_symbol_by_id(symbol_id, symbol_t=symbol_t, required=True)

    def get_symbol_by_id(
        self, symbol_id: UUID, symbol_t: typing.Type[SymbolT] | None = None, required: bool = False
    ) -> SymbolT | None:
        if not self.interpreted:
            raise RuntimeError(f"module index is not interpreted: {self}")
        symbol = self.symbols.get(symbol_id)
        if symbol is None:
            if required:
                raise KeyError(f"no symbol found for id {symbol_id}")
            return None
        if symbol_t is not None and not isinstance(symbol, symbol_t):
            raise TypeError(f"symbol {symbol} is not of type {symbol_t}")
        return symbol


def sort(module: Module):
    """Sorts the modules statements in-place according to parent & order keys."""

    for file in module.files:
        # per parent (incl. root = None) sort by order key
        sorted_statements = []
        statements_by_parent_id: dict[UUID | None, list[Statement]] = defaultdict(list)
        for statement in file.statements:
            statements_by_parent_id[statement.parent_id].append(statement)

        def walk_dfs(statement: Statement):
            sorted_statements.append(statement)
            children = statements_by_parent_id.get(statement.id)
            if children is not None:
                for child in sorted(children, key=lambda s: s.order_key):
                    walk_dfs(child)

        roots = statements_by_parent_id.get(None, [])
        for statement in sorted(roots, key=lambda s: s.order_key):
            walk_dfs(statement)

        file.statements = sorted_statements


def resolve(
    module: Module,
    lookup_in_module: Callable[[Requirement, StatementPath], Statement | None],
    on_error: Callable[[Error], None],
) -> ModuleIndex:
    """Resolve unresolved statement references in the given module."""

    idx = index_module(module, on_error=on_error)

    # copy to avoid concurrent modification when statements are imported
    statements = list(idx.statements.values())
    # resolve references to other statements
    for statement in statements:
        symbol = statement.symbol

        if isinstance(symbol, HasType):
            for node in symbol.walk():
                if node.reference is None or isinstance(node.reference, Statement):
                    continue  # nothing to resolve
                # normalize path to statement
                resolved_stmt = resolve_statement_reference(
                    reference=node.reference,
                    idx=idx,
                    lookup_in_module=lookup_in_module,
                    for_statement=statement,
                    symbol_type=SymbolType.TYPE,
                    on_error=on_error,
                )
                if resolved_stmt is None:
                    continue  # error already reported
                if not isinstance(resolved_stmt.symbol, TypeNode):
                    raise RuntimeError(f"resolved statement is not a type: {resolved_stmt})")
                node.reference = resolved_stmt.symbol

        if isinstance(symbol, Code):
            input_keys = (input.ident for input in symbol.type.inputs)
            symbol.parse = parse_code(symbol.code)
            for key, reference in symbol.parse.references.items():
                if key in input_keys:
                    continue  # input arguments are obviously not resolved
                reference = resolve_statement_reference(
                    reference=reference,
                    idx=idx,
                    lookup_in_module=lookup_in_module,
                    for_statement=statement,
                    on_error=on_error,  # not sure?
                    by=LookupBy.PyIdent,
                )
                if reference is not None:
                    symbol.references[key] = reference.symbol

    return idx


# identifier names can be escaped as '<name with space>'
# references can look like
# 1. <name>
# 2. .<path>.<name>
# 3. <module_owner>.<module_name>.<path>.<name>
REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?(\.(?P<path>[\w.\- ]+)\.)?(?P<name>[\w\- ]+)$"
)
# import source must either be in current module (.*) or absolute (<owner>.<name>.*)
RELATIVE_REFERENCE_REGEX = re.compile(r"^\.(?P<path>[\w.\- ]+)$")
ABSOLUTE_IMPORT_SOURCE_REGEX = re.compile(
    r"^(?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+)\.(?P<path>[\w.\- ]+)$"
)


def resolve_statement_reference(
    reference: StatementPath | UUID | None,
    idx: ModuleIndex,
    lookup_in_module: LookupFunc,
    for_statement: Statement,
    on_error: Callable[[Error], None],
    symbol_type: SymbolType | None = None,
    by: LookupBy = LookupBy.Name,
) -> Statement | None:
    def _error(_t: IT, cause: Exception | None = None, **error_args) -> None:
        on_error(Error(_t, for_statement, cause, **error_args))

    if reference is None:
        return _error(IT.MISSING_REFERENCE)
    elif isinstance(reference, UUID):
        # resolve by id
        resolved_scope = idx.scopes.get(reference)
        if resolved_scope is None:
            resolved_scope = lookup_in_module(None, reference, by)
        if resolved_scope is None:
            return _error(IT.UNDEFINED_REFERENCE, path="<id>")
        idx.import_scope(resolved_scope)
    elif reference.path == ".":  # normalize relative :StatementReferencePath
        statement_scope = idx.scopes[for_statement.id]
        resolved = statement_scope.lookup_statement(reference.name, exclude=for_statement, by=by)
        if resolved is None:
            return _error(IT.UNDEFINED_REFERENCE, path=reference)
        resolved_scope = idx.scopes[resolved.id]
    elif reference.path.startswith("."):  # normalize local :StatementReferencePath
        resolved_scope = idx.get_scope(reference, by=by)
        if resolved_scope is None:
            return _error(IT.UNDEFINED_REFERENCE, path=reference)
    else:  # resolve by absolute path in external module
        # get source requirement for external module
        source = ABSOLUTE_IMPORT_SOURCE_REGEX.match(reference.path)
        if source is None:  # (should be caught in parse)
            raise RuntimeError(f"invalid import source at {for_statement}")
        requirement_name = f"{source.group('module_owner')}.{source.group('module_name')}"
        requirement = idx.requirements_by_name.get(requirement_name)
        if requirement is None:
            return _error(IT.UNKNOWN_IMPORT_SOURCE, source=requirement_name)
        # localize path to required module
        localized_path = StatementPath("." + source.group("path"), reference.name)
        try:  # use module lookup to resolve
            resolved_scope = lookup_in_module(requirement, localized_path, by=by)
        except Exception as e:
            return _error(
                IT.EXTERNAL_LOOKUP_FAILED, error=e, path=localized_path, module=requirement
            )
        if resolved_scope is None:
            return _error(IT.UNDEFINED_EXTERNAL_REFERENCE, path=localized_path, module=requirement)
        # import resolved scope (and contents) into index
        idx.import_scope(resolved_scope)

    # check if the reference has the correct type
    if symbol_type is not None and resolved_scope.statement.symbol_type != symbol_type:
        return _error(
            IT.REFERENCE_TYPE_MISMATCH,
            type=symbol_type,
            resolved=resolved_scope.statement,
        )
    return resolved_scope.statement  # successfully resolved


def index_module(
    module: Module,
    on_error: Callable[[Error], None] | typing.Literal["raise"] = "raise",
) -> ModuleIndex:
    """Index the module's scopes and requirements."""
    if on_error == "raise":
        on_error = raise_error

    def _error(_t: IT, subject: Statement | File, cause: Exception | None = None, **error_args):
        on_error(Error(_t, subject, cause, **error_args))

    idx = ModuleIndex(module=module)

    # check for circular ancestry errors
    # :CircularAncestry
    has_circular_ancestry = False
    for statement in chain.from_iterable(file.statements for file in module.files):
        if statement.parent_id is None:
            continue
        seen_ancestors = set()
        path = [f"{statement.id}:{statement.name or '<empty>'}"]
        parent = statement.parent
        while parent is not None:
            path.append(f"{parent.id}:{parent.name or '<empty>'}")
            if parent.id in seen_ancestors:
                _error(IT.CIRCULAR_ANCESTRY, statement, path=".".join(reversed(path)))
                has_circular_ancestry = True
                break
            seen_ancestors.add(parent.id)
            parent = parent.parent
    if has_circular_ancestry:
        return idx  # bail

    # create scopes for files and statements (but don't populate nested statements yet)
    statements_by_parent_id: dict[UUID, list[Statement]] = defaultdict(list)
    for file in module.files:
        idx.files[file.id] = file
        file_scope = Scope(
            id=file.id, name=file.path_without_extension, file=file, parent=None, statement=None
        )
        if file_scope.name and file_scope.name in idx.scopes_by_name:
            _error(IT.AMBIGUOUS_DEFINITION, file, path=file_scope.name)
            idx.add_scope(file_scope, anonymous=True)
        else:
            idx.add_scope(file_scope)

        for statement in file.statements:
            if statement.type != StatementType.SYMBOL:
                continue  # ignore non-symbols
            idx.statements[statement.id] = statement
            idx.symbols[statement.id] = statement.symbol
            statements_by_parent_id[statement.parent_id or statement.file.id].append(statement)
            path = file_scope.name + ":" + statement.infile_path
            statement_scope = Scope(
                id=statement.id,
                name=path,
                file=file,
                parent=file_scope,
                statement=statement,
            )
            if not statement.name or statement_scope.name in idx.scopes_by_name:
                if statement.name:
                    _error(IT.AMBIGUOUS_DEFINITION, statement, path=statement_scope.name)
                idx.add_scope(statement_scope, anonymous=True)
            else:
                idx.add_scope(statement_scope)

    # set parent scope to statement parent (if it exists)
    for statement in idx.statements.values():
        if statement.parent_id is not None:
            statement_scope = idx.scopes[statement.id]
            if statement.parent_id not in idx.scopes:
                continue
            parent_scope = idx.scopes[statement.parent_id]
            statement_scope.parent = parent_scope

    # populate scopes with expanded statements
    for statements in statements_by_parent_id.values():
        # (first sort all statements by index ascending inside their parent)
        statements.sort(key=lambda s: s.order_key)
        for statement in statements:
            scope = idx.scopes[statement.id]
            scope.parent.add_statement(statement)

    # collect requirements
    for statement in idx.statements.values():
        if statement.symbol_type == SymT.REQUIREMENT:
            requirement_name = statement.name
            if requirement_name in idx.requirements_by_name:
                _error(IT.AMBIGUOUS_REQUIREMENT, statement, name=requirement_name)
                continue
            idx.requirements_by_name[requirement_name] = typing.cast(Requirement, statement.symbol)

    return idx


def interp(
    idx: ModuleIndex, on_error: Callable[[Error], None] | typing.Literal["raise"] = "raise"
) -> ModuleIndex:
    """Interpret the module's statements as symbols (populating the module's symbol tables)."""
    if idx.interpreted:
        raise RuntimeError(f"module already interpreted: {idx}")

    if on_error == "raise":
        on_error = raise_error

    def _error(_t: IT, subject: Statement | File, cause: Exception | None = None, **error_args):
        on_error(Error(_t, subject, cause, **error_args))

    inlined_node_ids: set[UUID] = set()

    # inline union types (and extend expectations if they exist)
    def _inline_type_union_rec(node: TypeNode, path: list[TypeNode]) -> list[TypeNode]:
        if any(n.id == node.id for n in path):
            path = "->".join(str(n) for n in path + [node])
            _error(IT.CIRCULAR_UNION, node.source, path=path)
            return []
        if not isinstance(node, (Type, Task, Code, Dataset)) or node.id in inlined_node_ids:
            return node.fields  # not a type or already inlined
        inlined_node_ids.add(node.id)
        if not any(n.flags & TypeFlag.IsUnionWith for n in node.fields):
            node.fields = node.fields
            return node.fields  # skip, not a union
        path = path + [node]
        resolved_fields = []
        node.fields = deepcopy_types(node.fields)  # retain originals
        for child in node.fields:
            if not child.flags & TypeFlag.IsUnionWith:
                resolved_fields.append(child)
                continue
            if not isinstance(child.reference, Type):
                continue  # ignore unresolved
            # inline child's type nodes
            for to_inline in _inline_type_union_rec(child.reference, path):
                existing = first((n for n in resolved_fields if n.name == to_inline.name), None)
                # check if type is compatible if overlapping
                if existing is not None and (
                    existing.tag != to_inline.tag
                    or existing.flags != to_inline.flags
                    or existing.hint != to_inline.hint
                ):
                    # TODO @Robustness: check union type compatibility properly
                    path = "->".join(str(n) for n in path)
                    _error(IT.MISMATCHED_UNION, node, node=existing, other=to_inline, path=path)
                    continue
                resolved_fields.append(to_inline)
            if isinstance(node, Type):  # extend expectations
                node.expectations.extend(child.reference.expectations)
        node.resolved_fields = resolved_fields
        return node.fields

    for symbol in idx.symbols.values():
        if isinstance(symbol, HasType):
            type = typing.cast(TypeNode, symbol)
            _inline_type_union_rec(type, [])

    idx.interpreted = True
    return idx


def get_reference_as_path(reference: Statement | Symbol, via: Statement | None) -> StatementPath:
    if isinstance(reference, Symbol):
        reference = reference.source
    if via is None or reference.file.id == via.file.id:
        return StatementPath(".", reference.name)
    elif reference.file.module.name == via.file.module.name:
        return StatementPath("." + reference.file.path_without_extension, reference.name)
    else:
        return StatementPath(
            reference.file.module.name + "." + reference.file.path_without_extension, reference.name
        )
