from __future__ import annotations

import enum
import typing
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field
from itertools import chain
from pathlib import Path
from typing import Callable, Optional
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.error import IssueType, ParseError, SemanticError
from bench.language.lex import lex, lex_string
from bench.language.parse import ABSOLUTE_IMPORT_SOURCE_REGEX, preparse
from bench.language.type import (
    SYMBOL_CLASS_BY_TYPE,
    SYMBOL_FIELDS_NAMES_BY_TYPE,
    Build,
    Code,
    CodeContent,
    Data,
    Expectation,
    File,
    InterpSymbol,
    Model,
    Module,
    RequirementContent,
    SourceFile,
    Statement,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    Task,
    Token,
    TokenType,
    Type,
    TypeContent,
    TypeFlag,
    TypeNode,
    TypeTag,
    deepcopy_types,
    parse_statement_path,
)
from bench.runtime.lsp import parse_code
from bench.utils.func import dict_intersect

logger = structlog.get_logger(__name__)


StmT = StatementType
SymT = SymbolType

SymbolContentT = typing.TypeVar("SymbolContentT", bound=SymbolContent)
SymbolT = typing.TypeVar("SymbolT", bound=InterpSymbol)


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


ET = IssueType
TT = TokenType


def ignore_module_lookup(*args, **kwargs):
    return None


def lookup_in_error(*args, **kwargs):
    raise NotImplementedError("external module lookup disabled")


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


LookupFunc = Callable[
    [RequirementContent | None, StatementPath | UUID, LookupBy], typing.Union["Scope", None]
]


def parse(
    tokens: list[Token],
    module: Optional[Module] = None,
    lookup_in_module: LookupFunc = lookup_in_error,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> tuple[Module, ModuleIndex]:
    """Parse a stream of tokens into Bench AST (grouped into files in a module)."""
    if on_error == "raise":
        on_error = raise_error
    if module is None:
        module = Module(name="<local>", files=[])

    # first pass: extract files and statements
    files = preparse(tokens, module, on_error=on_error)
    module.files.extend(files)
    # second pass: resolve references
    idx = resolve(module, lookup_in_module=lookup_in_module, on_error=on_error)
    # third pass: create symbols
    interp(idx, on_error=on_error)

    return module, idx


def parse_string(
    string: str,
    module: Optional[Module] = None,
    lookup_in_module: LookupFunc = ignore_module_lookup,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> tuple[Module, ModuleIndex]:
    tokens = lex_string(string)
    return parse(tokens, module=module, lookup_in_module=lookup_in_module, on_error=on_error)


def parse_file(
    file_path: str,
    module: Optional[Module] = None,
    lookup_in_module: LookupFunc = ignore_module_lookup,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> tuple[Module, ModuleIndex]:
    source_file = SourceFile(path=file_path, content=Path(file_path).read_text())
    tokens = lex(source_file)
    return parse(tokens, module=module, lookup_in_module=lookup_in_module, on_error=on_error)


@dataclass(repr=False)
class Scope:
    """A scope in which statements are defined. May be at file- or statement-level."""

    name: str
    id: UUID  # id from file or statement
    parent: Scope | None
    file: File
    statement: Statement | None
    statements: OrderedDict[str, Statement] = field(default_factory=OrderedDict)
    symbols: OrderedDict[str, InterpSymbol] = field(default_factory=OrderedDict)
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
    def parameters(self) -> list[Statement]:
        return [s for s in self.statements.values() if s.is_parameter]

    @property
    def arguments(self) -> list[Statement]:
        return [s for s in self.statements.values() if s.is_argument]

    @property
    def proper_statements(self) -> list[Statement]:
        return [s for s in self.statements.values() if not s.is_argument and not s.is_parameter]

    @property
    def proper_symbols(self) -> list[InterpSymbol]:
        return [self.symbols[s.name] for s in self.proper_statements if s.name in self.symbols]


@dataclass(repr=False)
class ModuleIndex:
    """The index into a module's resolved and interpreted symbols and statements."""

    module: Module
    requirements_by_name: dict[str, RequirementContent] = field(default_factory=dict)
    files: dict[UUID, File] = field(default_factory=OrderedDict)
    statements: dict[UUID, Statement] = field(default_factory=OrderedDict)
    interpreted: bool = False
    scopes: OrderedDict[UUID, Scope] = field(default_factory=OrderedDict)
    scopes_by_name: OrderedDict[str, Scope] = field(default_factory=OrderedDict)
    scopes_by_ident: OrderedDict[UUID, Scope] = field(default_factory=OrderedDict)
    symbols: dict[UUID, InterpSymbol] = field(default_factory=OrderedDict)

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
    lookup_in_module: Callable[[RequirementContent, StatementPath], Statement | None],
    on_error: Callable[[SemanticError], None],
) -> ModuleIndex:
    """Resolve unresolved statement references in the given module."""

    idx = index_module(module, on_error=on_error)

    # During resolution external scopes and their statements will be imported,
    # so we copy the statements to avoid concurrent modification.
    # (we only need to resolve our own statements, not the imported ones)
    statements = list(idx.statements.values())
    # resolve references to other statements
    for statement in statements:
        if isinstance(statement.reference, Statement) or not statement.has_reference:
            continue  # need not be resolved
        statement.reference = resolve_statement_reference(
            reference=statement.reference,
            idx=idx,
            lookup_in_module=lookup_in_module,
            for_statement=statement,
            symbol_type=statement.symbol_type,
            on_error=on_error,
        )

    # resolve type references
    for statement in statements:
        if isinstance(statement.content, TypeContent):
            resolve_type_references_rec(
                statement, statement.content, lookup_in_module, idx, on_error
            )

    # parse and resolve code references
    # (parse here because it's unclear where else to put code parsing in the pipeline,
    #  as other references are already 'pre-parsed' in preparse or when loaded from data)
    for statement in statements:
        code = statement.content
        if isinstance(code, CodeContent):
            input_keys = (input.ident for input in code.inputs)
            code.parse = parse_code(code.code)
            for key, reference in code.parse.references.items():
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
                    code.references[key] = reference

    return idx


def resolve_type_references_rec(
    for_statement: Statement,
    type: TypeNode,
    lookup_in_module: Callable[[RequirementContent, StatementPath], Statement | None],
    idx: ModuleIndex,
    on_error: Callable[[SemanticError], None],
) -> None:
    """Resolves and imputes type references in a type node recursively."""
    for node in type.walk():
        if node.reference is None or isinstance(node.reference, Statement):
            continue  # nothing to resolve
        # normalize path to statement
        resolved_stmt = resolve_statement_reference(
            reference=node.reference,
            idx=idx,
            lookup_in_module=lookup_in_module,
            for_statement=for_statement,
            symbol_type=SymbolType.TYPE,
            on_error=on_error,
        )
        if resolved_stmt is None:
            continue  # error already reported
        if not isinstance(resolved_stmt.content, TypeNode):
            raise RuntimeError(f"resolved statement is not a type: {resolved_stmt})")
        node.reference = resolved_stmt
        node.source_reference = get_reference_as_path(node.reference, via=for_statement)


def resolve_statement_reference(
    reference: StatementPath | UUID | None,
    idx: ModuleIndex,
    lookup_in_module: LookupFunc,
    for_statement: Statement,
    on_error: Callable[[SemanticError], None],
    symbol_type: SymbolType | None = None,
    by: LookupBy = LookupBy.Name,
) -> Statement | None:
    def _error(_t: ET, cause: Exception | None = None, **error_args) -> None:
        on_error(SemanticError(_t, for_statement, cause, **error_args))

    if reference is None:
        return _error(ET.MISSING_REFERENCE)
    elif isinstance(reference, UUID):
        # resolve by id
        resolved_scope = idx.scopes.get(reference)
        if resolved_scope is None:
            resolved_scope = lookup_in_module(None, reference, by)
        if resolved_scope is None:
            return _error(ET.UNDEFINED_LOCAL_REFERENCE, path="<id>")
        idx.import_scope(resolved_scope)
    elif reference.path == ".":  # normalize relative :StatementReferencePath
        statement_scope = idx.scopes[for_statement.id]
        resolved = statement_scope.lookup_statement(reference.name, exclude=for_statement, by=by)
        if resolved is None:
            return _error(ET.UNDEFINED_LOCAL_REFERENCE, path=reference)
        resolved_scope = idx.scopes[resolved.id]
    elif reference.path.startswith("."):  # normalize local :StatementReferencePath
        resolved_scope = idx.get_scope(reference, by=by)
        if resolved_scope is None:
            return _error(ET.UNDEFINED_LOCAL_REFERENCE, path=reference)
    else:  # resolve by absolute path in external module
        # get source requirement for external module
        source = ABSOLUTE_IMPORT_SOURCE_REGEX.match(reference.path)
        if source is None:  # (should be caught in parse)
            raise RuntimeError(f"invalid import source at {for_statement}")
        requirement_name = f"{source.group('module_owner')}.{source.group('module_name')}"
        requirement = idx.requirements_by_name.get(requirement_name)
        if requirement is None:
            return _error(ET.UNKNOWN_IMPORT_SOURCE, source=requirement_name)
        # localize path to required module
        localized_path = StatementPath("." + source.group("path"), reference.name)
        try:  # use module lookup to resolve
            resolved_scope = lookup_in_module(requirement, localized_path, by=by)
        except Exception as e:
            return _error(
                ET.EXTERNAL_LOOKUP_FAILED, error=e, path=localized_path, module=requirement
            )
        if resolved_scope is None:
            return _error(ET.UNDEFINED_EXTERNAL_REFERENCE, path=localized_path, module=requirement)
        # import resolved scope (and contents) into index
        idx.import_scope(resolved_scope)

    # check if the reference has the correct type
    if symbol_type is not None and resolved_scope.statement.symbol_type != symbol_type:
        return _error(
            ET.REFERENCE_TYPE_MISMATCH,
            type=symbol_type,
            resolved=resolved_scope.statement,
        )
    return resolved_scope.statement  # successfully resolved


def index_module(
    module: Module,
    on_error: Callable[[SemanticError], None] | typing.Literal["raise"] = "raise",
) -> ModuleIndex:
    """Index the module's scopes and requirements."""
    if on_error == "raise":
        on_error = raise_error

    def _error(_t: ET, subject: Statement | File, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, subject, cause, **error_args))

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
                _error(ET.CIRCULAR_ANCESTRY, statement, path=".".join(reversed(path)))
                has_circular_ancestry = True
                break
            seen_ancestors.add(parent.id)
            parent = parent.parent
    if has_circular_ancestry:
        return idx  # bail

    # create scopes for files and statements (but don't populate nested statements them yet)
    statements_by_parent_id: dict[UUID, list[Statement]] = defaultdict(list)
    for file in module.files:
        idx.files[file.id] = file
        file_scope = Scope(
            id=file.id, name=file.path_without_extension, file=file, parent=None, statement=None
        )
        if file_scope.name and file_scope.name in idx.scopes_by_name:
            _error(ET.AMBIGUOUS_DEFINITION, file, path=file_scope.name)
            idx.add_scope(file_scope, anonymous=True)
        else:
            idx.add_scope(file_scope)

        for statement in file.statements:
            if statement.name is None:
                continue  # ignore blanks and comments
            # create scope for every regular statement
            idx.statements[statement.id] = statement
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
                    _error(ET.AMBIGUOUS_DEFINITION, statement, path=statement_scope.name)
                idx.add_scope(statement_scope, anonymous=True)
            else:
                idx.add_scope(statement_scope)

    # set parent scope to statement parent (if it exists)
    for statement in idx.statements.values():
        if statement.parent_id is not None:
            statement_scope = idx.scopes[statement.id]
            if statement.parent_id not in idx.scopes:
                _error(ET.UNEXPECTED_CHILDREN, statement)
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
                _error(ET.AMBIGUOUS_REQUIREMENT, statement, name=requirement_name)
                continue
            idx.requirements_by_name[requirement_name] = typing.cast(
                RequirementContent, statement.content
            )

    return idx


def interp(
    idx: ModuleIndex, on_error: Callable[[SemanticError], None] | typing.Literal["raise"] = "raise"
) -> ModuleIndex:
    """Interpret the module's statements as symbols (populating the module's symbol tables)."""
    if idx.interpreted:
        raise RuntimeError(f"module already interpreted: {idx}")

    if on_error == "raise":
        on_error = raise_error

    def _error(_t: ET, subject: Statement | File, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, subject, cause, **error_args))

    # create symbol shells for proper statements (without symbol-specific fields)
    # include imported scopes (we want their symbols too, and they may be abstract)
    for scope in idx.scopes.values():
        if scope.statement is None or not scope.statement.is_real:
            continue
        statement = scope.statement

        # TODO @Feature: implement abstraction/variable templating :Variables
        abstract = False
        if scope.parameters or scope.arguments:
            _error(ET.UNEXPECTED_PARAMETERS, statement)
            continue
        if statement.underlying_definition is None:
            # definition is not available, probably due to some reference or load error
            # we ignore here since this is already an error upstream
            continue

        base_symbol = InterpSymbol(
            id=statement.id,
            name=statement.name,
            abstract=abstract,
            modifier=statement.modifier,
            source=statement,
        )
        source_content = statement.underlying_definition.content
        symbol_cls = SYMBOL_CLASS_BY_TYPE[statement.symbol_type]
        symbol_keys = SYMBOL_FIELDS_NAMES_BY_TYPE[statement.symbol_type]
        # intersect because source_content may be a full InterpSymbol (see below)
        symbol_kwargs = dict_intersect(source_content.__dict__, symbol_keys)
        symbol_kwargs.update(base_symbol.__dict__)
        symbol = symbol_cls(**symbol_kwargs)  # type: ignore

        # replace source content with symbol (if it's a definition)
        if statement.content is not None:
            statement.content = symbol

        scope.parent.add_symbol(symbol)
        idx.symbols[statement.id] = symbol

    # set symbol reference and definition sites
    for symbol in idx.symbols.values():
        # reference symbol is just the symbol that was directly referenced as a statement
        if symbol.source.has_reference:
            symbol.reference = idx.symbols[symbol.source.reference.id]

        # :SymbolDefinitionReference
        # definition site is the first non-abstract reference or definition
        # excluding plain references without parameters
        definition_stmt = symbol.source.underlying_definition
        symbol.definition = idx.symbols[definition_stmt.id]
        if symbol.definition.abstract:
            raise NotImplementedError("TODO @Incomplete: implement abstraction :Variables")

    # interp type nodes
    interped_type_nodes: set[UUID] = set()

    def _interp_type_rec(node: TypeNode):
        if node.id in interped_type_nodes:
            return
        interped_type_nodes.add(node.id)
        # impute type reference
        if isinstance(node.reference, Statement):
            node.reference = idx.symbols[node.reference.id]
            if node.reference.tag == TypeTag.TYPE_REFERENCE:
                _interp_type_rec(node.reference)
            if not isinstance(node.reference, Type) or node.reference.tag == TypeTag.TYPE_REFERENCE:
                raise ValueError(f"type reference is not resolved: {node}")
            node.tag = node.reference.tag
            if node.source_reference is None:
                node.source_reference = node.reference.name
        for child in node.fields:
            _interp_type_rec(child)

    # first impute all the references
    for symbol in idx.symbols.values():
        if isinstance(symbol, TypeContent):
            _interp_type_rec(symbol)

    # interp symbol contents using related symbols
    # this should probably set/work with :InstructionOps?
    for id, symbol in idx.symbols.items():
        statement = idx.statements[id]
        if statement.type == StatementType.DEFINITION:
            scope = idx.scopes[id]
        else:  # borrow scope from reference (imported references import their scope)
            scope = idx.scopes[statement.reference_id]

        if isinstance(symbol, (Type, Task, Expectation)):
            for child in scope.proper_symbols:
                if child.source.is_expect:
                    symbol.expectations.append(child)
        if isinstance(symbol, Task):
            for child in scope.proper_symbols:
                if isinstance(child, (Task, Code)):
                    symbol.steps.append(child)
        if isinstance(symbol, Code):
            # replace code references with symbols
            for key, reference in symbol.references.items():
                symbol.context[key] = idx.symbols[reference.id]
        if isinstance(symbol, Build):
            for child in scope.proper_symbols:
                if isinstance(child, Model):
                    symbol.models.append(child)
                elif isinstance(child, Task):
                    symbol.tasks.append(child)

    inlined_node_ids: set[UUID] = set()

    # inline union types (and extend expectations if they exist)
    def _inline_type_union_rec(node: TypeNode, path: list[TypeNode]) -> list[TypeNode]:
        if any(n.id == node.id for n in path):
            path = "->".join(str(n) for n in path + [node])
            _error(ET.CIRCULAR_UNION, node.source, path=path)
            return []
        if not isinstance(node, (Type, Task, Code, Data)) or node.id in inlined_node_ids:
            return node.fields  # not a type or already inlined
        inlined_node_ids.add(node.id)
        if not any(n.flags & TypeFlag.IsUnionWith for n in node.fields):
            node.fields = node.fields
            return node.fields  # skip, not a union
        path = path + [node]
        inlined_nodes = []
        node.self_fields = deepcopy_types(node.fields)  # retain originals
        for child in node.fields:
            if not child.flags & TypeFlag.IsUnionWith:
                inlined_nodes.append(child)
                continue
            if not isinstance(child.reference, Type):
                continue  # ignore unresolved
            # inline child's type nodes
            for to_inline in _inline_type_union_rec(child.reference, path):
                existing = first((n for n in inlined_nodes if n.name == to_inline.name), None)
                # check if type is compatible if overlapping
                if existing is not None and (
                    existing.tag != to_inline.tag
                    or existing.source_reference != to_inline.source_reference
                ):
                    # TODO @Robustness: check union type compatibility properly
                    path = "->".join(str(n) for n in path)
                    _error(ET.MISMATCHED_UNION, node, node=existing, other=to_inline, path=path)
                    continue
                inlined_nodes.append(to_inline)
            if isinstance(node, Type):  # extend expectations
                node.expectations.extend(child.reference.expectations)
        node.fields = inlined_nodes
        return node.fields

    for node_id in interped_type_nodes:
        if node_id in idx.symbols:
            type = typing.cast(TypeNode, idx.symbols[node_id])
            _inline_type_union_rec(type, [])

    idx.interpreted = True
    return idx


def get_reference_as_path(
    reference: Statement | InterpSymbol, via: Statement | None
) -> StatementPath:
    if isinstance(reference, InterpSymbol):
        reference = reference.source
    if via is None or reference.file.id == via.file.id:
        return StatementPath(".", reference.name)
    elif reference.file.module.name == via.file.module.name:
        return StatementPath("." + reference.file.path_without_extension, reference.name)
    else:
        return StatementPath(
            reference.file.module.name + "." + reference.file.path_without_extension, reference.name
        )
