from __future__ import annotations

import csv
import enum
import json
import re
import typing
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field
from itertools import chain
from pathlib import Path
from typing import Callable, Optional
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.error import ErrorType, ParseError, SemanticError
from bench.language.lex import lex, lex_string
from bench.language.type import (
    SYMBOL_CLASS_BY_TYPE,
    SYMBOL_FIELDS_NAMES_BY_TYPE,
    Build,
    BuildContent,
    CapabilityContent,
    Code,
    CodeContent,
    Data,
    DataContent,
    Expectation,
    ExpectationContent,
    File,
    InterpSymbol,
    Model,
    Module,
    Record,
    RequirementContent,
    SimpleTypeNode,
    SourceFile,
    Statement,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    Task,
    TaskContent,
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
from bench.utils.fractional import (
    INTEGER_ZERO,
    generate_key_between,
    generate_n_keys_between,
    increment_integer,
)
from bench.utils.func import dict_intersect

logger = structlog.get_logger(__name__)

UNGROUPED_STATEMENT_TYPES = (StatementType.DEFINITION, StatementType.REDEFINITION)


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


ET = ErrorType
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


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically matches (and ignores) indentation at the set level (mutable).
    """

    def __init__(self, tokens: list[Token], start_pos: int = 0, indent_level: int = 0):
        self._tokens = tokens
        self._start_pos = start_pos
        self._peek_pos = start_pos
        self._peek_indent_level = 0
        self._peek_has_indent = False
        self._marks = {}
        self.indent_level = indent_level
        self.reset(None)

    def mark(self) -> object:
        mark_token = object()
        self._marks[mark_token] = self.current_pos
        return mark_token

    def reset(self, mark_token: object | None):
        if mark_token:
            self._peek_pos = self._marks.pop(mark_token)
        self._advance()

    def advance(self, count: int = 1):
        """Advance the peek window by the given number of tokens (skipping indents appropriately)."""
        if count < 0:
            for _ in range(-count):
                self._advance(-1)
        else:
            for _ in range(count):
                self._advance(1)

    def eaten(self, mark_token: object | None) -> list[Token]:
        start_pos = self._marks[mark_token] if mark_token else self._start_pos
        return self._tokens[start_pos : max(0, self.current_pos)]

    @property
    def current_pos(self) -> int:
        return self._peek_pos - 1

    @property
    def previous(self) -> Optional[Token]:
        if self.current_pos <= self._start_pos:
            return None
        return self._tokens[self.current_pos - 1]

    def _advance(self, inc: int = 1):
        """Advances the peek position, skipping matching indentation."""

        # advance to next token (current pos is based on peek pos)
        self._peek_pos += inc

        # get next indent level
        self._peek_indent_level = 0
        for token in self._tokens[self.current_pos :]:
            if token.type != TT.INDENT:
                break
            self._peek_indent_level += inc

        # skip indentation if matching
        if self._peek_indent_level == self.indent_level:
            self._peek_pos += self._peek_indent_level

    @property
    def peek_indent_level(self) -> int:
        return self._peek_indent_level

    def peek(self) -> Optional[Token]:
        """Peeks the next token without eating it."""
        if self.current_pos >= len(self._tokens):
            return None
        return self._tokens[self.current_pos]

    def eat(self) -> Token:
        """Eats the next token."""
        if self.peek() is None:
            raise ParseError(ET.MISSING_TOKEN, self.previous)
        peek_token = self.peek()
        self._advance()
        return peek_token

    def peek_type(self, type: TT) -> Optional[Token]:
        token = self.peek()
        return token if token is not None and token.type == type else None

    def eat_type(self, type: TT) -> Token:
        token = self.eat()
        if token.type != type:
            raise ParseError(ET.UNEXPECTED_TOKEN_TYPE, token, type=type)
        return token

    def eat_newfile(self) -> Token:
        return self.eat_type(TT.NEWFILE)

    def eat_newline(self) -> Token:
        return self.eat_type(TT.NEWLINE)

    def eat_newline_or_eos(self) -> Optional[Token]:
        if self.peek() is None or self.peek_type(TT.NEWFILE) is not None:
            return
        return self.eat_newline()

    def eat_eos(self) -> Optional[Token]:
        if self.peek() is None:
            return
        return self.eat_type(TT.NEWFILE)

    def peek_has_newline_or_eos(self) -> bool:
        if self.peek() is None or self.peek_type(TT.NEWFILE) is not None:
            return True
        return self.peek_type(TokenType.NEWLINE) is not None

    def peek_indent(self) -> Optional[Token]:
        return self.peek_type(TT.INDENT)

    def eat_indent(self) -> Token:
        return self.eat_type(TT.INDENT)

    def peek_keyword(self, keyword: str | enum.Enum) -> Optional[Token]:
        token = self.peek_type(TT.KEYWORD)
        if token is None or token.value != keyword:
            return None
        return token

    def eat_keyword(self, keyword: str | enum.Enum) -> Token:
        token = self.eat_type(TT.KEYWORD)
        if token.value != keyword:
            raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, token, type=TT.KEYWORD, value=keyword)
        return token

    def peek_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Optional[Token]:
        token = self.peek_type(TT.KEYWORD)
        if token is None or not isinstance(token.value, keyword_cls):
            return None
        return token

    def eat_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Token:
        token = self.eat_type(TT.KEYWORD)
        if not isinstance(token.value, keyword_cls):
            raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, token, type=TT.KEYWORD, value=keyword_cls)
        return token

    def eat_identifier(self) -> Token:
        return self.eat_type(TT.IDENTIFIER)

    def peek_separator(self, separator: str | enum.Enum) -> Optional[Token]:
        if isinstance(separator, enum.Enum):
            separator = separator.value
        token = self.peek_type(TT.SEPARATOR)
        if token is None or token.value != separator:
            return None
        return token

    def eat_separator(self, separator: str | enum.Enum) -> Token:
        if isinstance(separator, enum.Enum):
            separator = separator.value
        token = self.eat_type(TT.SEPARATOR)
        if token.value != separator:
            raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, token, type=TT.SEPARATOR, value=separator)
        return token

    def peek_bracket(self, bracket: str) -> Optional[Token]:
        token = self.peek_type(TT.BRACKET)
        if token is None or token.value != bracket:
            return None
        return token

    def eat_bracket(self, bracket: str) -> Token:
        token = self.eat_type(TT.BRACKET)
        if token is None or token.value != bracket:
            raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, token, type=TT.BRACKET, value=bracket)
        return token

    def eat_space(self) -> Token:
        return self.eat_separator(" ")

    def eat_literal(self) -> Token:
        return self.eat_type(TT.LITERAL)

    def peek_description(self) -> Optional[Token]:
        return self.peek_type(TT.DESCRIPTION)

    def eat_description(self) -> Token:
        return self.eat_type(TT.DESCRIPTION)


def parser_from_string(string: str) -> TokenParser:
    return TokenParser(lex_string(string), start_pos=0, indent_level=0)


def _strip_literal_indent(text: str, indent_level: int) -> str:
    """Removes indentation up to the given level (4 spaces or 1 tab)."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if line.startswith(" " * 4 * indent_level):
            lines[i] = line[indent_level * 4 :]
        elif line.startswith("\t" * indent_level):
            lines[i] = line[indent_level:]
    return "\n".join(lines)


@dataclass
class FileParseState:
    file: File
    indent: int = 0
    ancestors: list[Statement] = field(default_factory=list)  # by indent
    statements_by_parent: dict[UUID | None, list[Statement]] = field(
        default_factory=lambda: defaultdict(list)
    )
    previous_errors: list[ParseError | None] = field(default_factory=list)

    def add_statement(self, statement: Statement):
        self.file.statements.append(statement)

        self.ancestors = self.ancestors[: self.indent]  # wipe ancestors with higher indent
        self.ancestors += [statement]  # replace ancestor at current indent

        parent_id = statement.parent.id if statement.parent else None
        self.statements_by_parent[parent_id].append(statement)

    def children(self, statement: Statement) -> list[Statement]:
        return self.statements_by_parent[statement.id]

    @property
    def root(self) -> bool:
        return self.indent == 0

    @property
    def parent(self) -> Statement | None:
        return self.ancestors[self.indent - 1] if self.indent > 0 else None

    @property
    def next_order_key(self) -> str:
        if self.parent is None:
            siblings = self.root_statements
        else:
            siblings = self.children(self.parent)
        if len(siblings) == 0:
            return INTEGER_ZERO
        else:
            return increment_integer(siblings[-1].order_key)

    @property
    def root_statements(self) -> list[Statement]:
        return self.statements_by_parent[None]


def preparse(
    tokens: list[Token], module: Module, on_error: Callable[[ParseError], None]
) -> list[File]:
    """Map tokens into files and statements with unresolved references."""

    if len(tokens) == 0:
        # empty stream
        return []

    tokens = TokenParser(tokens, start_pos=0, indent_level=0)
    files: dict[str, FileParseState] = OrderedDict()  # remember original file order
    # tokens[0] must be newfile
    initial_file: File = File(module=module, path=tokens.eat_newfile().value)
    local = FileParseState(file=initial_file)
    files[initial_file.path] = local

    while tokens.peek() is not None:
        # reset indent if previous token was on a different line
        if tokens.previous is not None and tokens.previous.line_number != tokens.peek().line_number:
            local.indent = 0

        if tokens.peek().type == TT.NEWFILE:
            path = tokens.eat_newfile().value
            if path not in files:  # begin new file state
                new_file = File(module=module, path=path)
                files[path] = FileParseState(file=new_file)
            local = files[path]
        elif tokens.peek().type == TT.INDENT:
            tokens.eat()
            local.indent += 1
        else:  # parse statement
            if len(local.ancestors) < local.indent:  # too much indentation
                raise ParseError(ET.UNEXPECTED_INDENT, tokens.peek(), indent=local.indent)

            errors: list[ParseError] = []
            tokens.indent_level = local.indent  # skip indent tokens at current level
            start_mark = tokens.mark()
            statement = _parse_statement(
                tokens,
                is_root=local.root,
                on_error=errors.append,
                file=local.file,
                parent=local.parent,
                order_key=local.next_order_key,
            )
            tokens.indent_level = 0  # skip only for statement parsing

            # handle and remember parse errors
            errors.sort(key=lambda e: e.position, reverse=True)  # get the deepest error
            likely_error = (
                errors[0] if errors and errors[0].position > errors[-1].position else None
            )
            local.previous_errors.append(likely_error)
            if statement is None:
                cause = likely_error or (
                    local.previous_errors[-2] if len(local.previous_errors) > 1 else None
                )
                error = ParseError(ET.INVALID_STATEMENT, tokens.peek(), cause=cause)
                on_error(error)
                continue

            statement._source = tokens.eaten(start_mark)

            # only comments/blanks can have comments/blanks as parents
            if (
                local.parent
                and local.parent.type in (StatementType.BLANK, StatementType.COMMENT)
                and statement.type not in (StatementType.BLANK, StatementType.COMMENT)
            ):
                # can't have children
                raise ParseError(ET.UNEXPECTED_INDENT, tokens.peek(), indent=local.indent)
            local.add_statement(statement)

    return [state.file for state in files.values()]


def _parse_comment(tokens: TokenParser, **kwargs) -> Statement:
    token = tokens.eat_type(TT.COMMENT)
    tokens.eat_newline_or_eos()
    return Statement(type=StatementType.COMMENT, text=token.value, **kwargs)


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


def _parse_typed_reference_slot(tokens: TokenParser) -> tuple[StatementPath, Token]:
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    path = _parse_reference_slot(tokens)
    return path, symbol_type


def _parse_reference_slot(tokens: TokenParser) -> StatementPath:
    # references can look like
    # 1. <name>
    # 2. .<path>.<name>
    # 3. <module>.<path>.<name>
    reference = tokens.eat_identifier()
    path_match = REFERENCE_REGEX.match(reference.value)
    if path_match is None:
        raise ParseError(ET.INVALID_TOKEN_VALUE, reference, value=reference.value, error="invalid")
    if path_match.group("path") is not None:
        if path_match.group("module_owner") is not None:
            path = f"{path_match.group('module_owner')}.{path_match.group('module_name')}.{path_match.group('path')}"
        else:
            path = f".{path_match.group('path')}"
    else:
        path = "."
    name = path_match.group("name")
    return StatementPath(path, name)


def _parse_import(tokens: TokenParser, **kwargs) -> Statement:
    """Parse an import statement."""
    tokens.eat_keyword(StatementType.IMPORT)
    tokens.eat_space()
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    reference = tokens.eat_identifier().value
    tokens.eat_space()
    if tokens.peek_keyword("as"):
        tokens.eat_keyword("as")
        tokens.eat_space()
        alias = tokens.eat_identifier().value
        tokens.eat_space()
    else:
        alias = reference
    tokens.eat_keyword("from")
    tokens.eat_space()
    source = tokens.eat_identifier()
    if (
        RELATIVE_REFERENCE_REGEX.match(source.value) is None
        and ABSOLUTE_IMPORT_SOURCE_REGEX.match(source.value) is None
    ):
        raise ParseError(ET.INVALID_TOKEN_VALUE, source, value=source.value, error="invalid")
    tokens.eat_newline_or_eos()
    return Statement(
        type=StatementType.IMPORT,
        name=alias,
        symbol_type=symbol_type.value,
        reference=StatementPath(source.value, reference),
        **kwargs,
    )


def _parse_definition(tokens: TokenParser, **kwargs) -> Statement:
    """Parses a symbol definition statement."""
    modifier = _parse_modifier_slot(tokens)
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    name = tokens.eat_identifier()

    definition = Statement(
        type=StatementType.DEFINITION,
        modifier=modifier,
        name=name.value,
        symbol_type=symbol_type.value,
        **kwargs,
    )
    definition.content = _parse_definition_content(tokens, symbol_type, name)
    tokens.eat_newline_or_eos()
    return definition


def _parse_definition_content(
    tokens: TokenParser, symbol_type: Token, name: Token
) -> SymbolContent:
    """Parses the content of a symbol definition including everything after [modifier] [type] [name]"""
    if symbol_type.value == SymbolType.CAPABILITY:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return CapabilityContent(description=description.value)
    elif symbol_type.value == SymbolType.TASK:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return TaskContent(
            name=name.value,
            description=description.value,
            tag=TypeTag.FUNCTION,
            type_nodes=type.type_nodes,
        )
    elif symbol_type.value == SymbolType.EXPECTATION:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return ExpectationContent(description=description.value)
    elif symbol_type.value == SymbolType.CODE:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        if lang not in ("python", "py", "x"):
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
        if lang == "py":
            lang = "python"
        code_text = _strip_literal_indent(literal.value, tokens.indent_level)
        return CodeContent(
            description=description,
            language=lang,  # noqa
            code=code_text,
            tag=TypeTag.FUNCTION,
            type_nodes=type.type_nodes,
            xblocks=[],  # not parsed yet
        )
    elif symbol_type.value == SymbolType.DATA:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_struct(tokens, name="element")
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        records = _parse_dataset_records(tokens, type, lang, literal)
        return DataContent(
            description=description,
            language=lang,
            records=records,
            tag=TypeTag.STRUCT,
            type_nodes=type.type_nodes,
        )
    elif symbol_type.value == SymbolType.BUILD:
        tokens.eat_separator(":")
        return BuildContent()

    raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, symbol_type, type=TT.KEYWORD, value=SymbolType)


def _parse_dataset_records(
    tokens: TokenParser, type: TypeNode, lang: str | None, literal: Token
) -> list[Record]:
    """Parses the language and records from a dataset literal."""
    value_str = _strip_literal_indent(literal.value, tokens.indent_level)
    try:
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        elif lang == "jsonl":
            records_data = [json.loads(line) for line in value_str.splitlines()]
        elif lang == "json":
            records_data = json.loads(value_str)
        elif lang == "csv":
            field_names = [element.name for element in type.type_nodes]
            csv_reader = csv.DictReader(
                value_str.splitlines(), quoting=csv.QUOTE_NONNUMERIC, fieldnames=field_names
            )
            records_data = list(csv_reader)
        else:
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
    except ParseError:
        raise  # re-raise since we don't want to catch our own errors
    except ValueError as e:
        raise ParseError(ET.INVALID_TOKEN_VALUE, literal, value=value_str, error=e)

    order_keys = generate_n_keys_between(None, None, len(records_data))
    records = [
        Record(data=data, order_key=order_key) for data, order_key in zip(records_data, order_keys)
    ]
    return records


def _parse_definition_requirement(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a requirement statement (special path because of name@value syntax)."""
    tokens.eat_keyword(SymbolType.REQUIREMENT)
    tokens.eat_space()
    dependency = tokens.eat_identifier()
    tokens.eat_separator("@")
    version = tokens.eat_identifier()
    tokens.eat_newline_or_eos()
    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.REQUIREMENT,
        name=dependency.value,
        **kwargs,
    )
    definition.content = RequirementContent(
        module_name=dependency.value, version=version.value, module_id=None
    )
    return definition


def _parse_definition_type(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a type statement (special path because of special syntax)."""
    tokens.eat_keyword(SymbolType.TYPE)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline()
    description = _parse_description_line_optional(tokens)
    struct = parse_type_struct_def(tokens, name=name.value)
    struct.description = description
    tokens.eat_newline_or_eos()
    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        content=struct,
        **kwargs,
    )
    return definition


def _parse_definition_enum(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a value enum statement (special path because of special syntax)"""
    tokens.eat_keyword(TypeTag.ENUM)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline()
    description = _parse_description_line_optional(tokens)

    # parse members (assumes literal members only)
    members: list[SimpleTypeNode] = []
    while tokens.peek_separator("-"):
        tokens.eat_separator("-")
        tokens.eat_space()
        member_name = tokens.eat_identifier().value
        member_description = None
        if tokens.peek_separator(" "):
            tokens.eat_space()
            member_description = tokens.eat_description().value
        last_order_key = members[-1].order_key if members else INTEGER_ZERO
        member = SimpleTypeNode(
            order_key=generate_key_between(None, last_order_key),
            name=member_name,
            tag=TypeTag.LITERAL,
            value=member_name,
            description=member_description,
        )
        members.append(member)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()

    enum_type_node = TypeContent(
        name=name.value,
        description=description,
        tag=TypeTag.ENUM,
        type_nodes=[*members],
    )
    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        content=enum_type_node,
        **kwargs,
    )
    return definition


def parse_simple_type_node(tokens: TokenParser) -> SimpleTypeNode:
    """Parse single tuple like <name>: <type>[ "<description>"]"""
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_space()
    return parse_simple_type_node_inline(tokens, name.value)


def parse_simple_type_node_inline(tokens: TokenParser, name: str | None) -> SimpleTypeNode:
    """Parse a type node type including description, handling simple nesting.."""

    flags = TypeFlag.Zero
    # parse array like [<type>] (one layer only)
    if tokens.peek_bracket("["):
        flags |= TypeFlag.IsArray
        tokens.eat_bracket("[")

    # parse actual type as either primitive or type reference
    reference = None
    value = None
    if tokens.peek_keyword_like(TypeTag):
        # not all value types are keywords, but only the valid ones are in KEYWORDS
        type = tokens.eat_keyword_like(TypeTag).value
    elif tokens.peek_type(TokenType.LITERAL):
        type = TypeTag.LITERAL
        value_token = tokens.eat_literal()
        try:
            value = json.loads(value_token.value)
        except ValueError as e:
            raise ParseError(ET.INVALID_TOKEN_VALUE, value_token, error=e, value=value)
    else:
        type = TypeTag.TYPE_REFERENCE
        reference = _parse_reference_slot(tokens)

    if flags & TypeFlag.IsArray:
        tokens.eat_bracket("]")

    if tokens.peek_type(TokenType.MARK_OPTIONAL):
        tokens.eat()
        flags |= TypeFlag.IsNullable

    if tokens.peek_separator(" "):
        tokens.eat_space()
        description = tokens.eat_description().value
    else:
        description = None

    return SimpleTypeNode(
        name=name,
        tag=type,
        value=value,
        reference=reference,
        description=description,
        flags=flags,
    )


def assign_type_node_oks(nodes: list[SimpleTypeNode]) -> list[SimpleTypeNode]:
    """Assign order keys to type nodes and return the list of nodes."""
    oks = generate_n_keys_between(None, None, len(nodes))
    for node, ok in zip(nodes, oks):
        node.order_key = ok
    return nodes


def parse_type_struct_def(tokens: TokenParser, name: str | None) -> TypeContent:
    # parse tuples like <tuple1>\n<tuple2>\n...
    struct = TypeContent(name=name, tag=TypeTag.STRUCT)
    while tokens.peek_separator("-") or tokens.peek_separator("&"):
        sep = tokens.eat()
        tokens.eat_space()
        if sep.value == "&":
            node = SimpleTypeNode(
                name=None,
                tag=TypeTag.TYPE_REFERENCE,
                reference=(_parse_reference_slot(tokens)),
                flags=TypeFlag.IsUnionWith,
            )
        else:
            node = parse_simple_type_node(tokens)
        struct.type_nodes.append(node)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()
        if not (tokens.peek_separator("-") or tokens.peek_separator("&")):
            tokens.advance(-1)  # go back one token to leave newline separator
            break
    assign_type_node_oks(struct.type_nodes)
    return struct


def parse_type_struct(
    tokens: TokenParser, name: str | None, is_output: bool = False
) -> TypeContent:
    """Parse an entire struct with possible unions like (...) & ... & ..."""

    def _parse_union_join():
        if tokens.peek_separator(" "):
            tokens.eat_separator(" ")
            if tokens.peek_separator("&"):
                tokens.eat_separator("&")
                tokens.eat_separator(" ")
                return True
            else:
                tokens.advance(-1)  # go back one token
        return False

    expect_union = True
    if tokens.peek_bracket("("):
        tokens.eat_bracket("(")
        struct = parse_type_struct_inline(tokens, name, is_output=is_output)
        tokens.eat_bracket(")")

        # parse start of union if there is one
        expect_union = _parse_union_join()
    else:
        struct = TypeContent(name=name, tag=TypeTag.STRUCT)

    while expect_union:
        node = SimpleTypeNode(
            name=None,
            tag=TypeTag.TYPE_REFERENCE,
            flags=TypeFlag.IsUnionWith | (TypeFlag.IsOutput if is_output else 0),
            reference=_parse_reference_slot(tokens),
        )
        struct.type_nodes.append(node)
        expect_union = _parse_union_join()

    assign_type_node_oks(struct.type_nodes)

    return struct


def parse_type_struct_inline(
    tokens: TokenParser, name: str | None, is_output: bool = False
) -> TypeContent:
    """Parse the inner part of an inline struct like name1: type1, name2: type2, ..."""
    struct = TypeContent(name=name, tag=TypeTag.STRUCT)
    while not tokens.peek_bracket(")"):
        tuple = parse_simple_type_node(tokens)
        if is_output:
            tuple.flags = tuple.flags | TypeFlag.IsOutput
        struct.type_nodes.append(tuple)
        if not tokens.peek_separator(","):
            break
        tokens.eat_separator(",")
        tokens.eat_space()
    assign_type_node_oks(struct.type_nodes)
    return struct


def parse_type_func(tokens: TokenParser, name: str | None) -> TypeContent:
    # parse signature like (<tuple1>, <tuple2>, ...) -> (<tuple1>, <tuple2>, ...)
    nodes = parse_type_struct(tokens, name=None).type_nodes
    if tokens.peek_separator(" "):
        tokens.eat_space()
        tokens.eat_separator("->")
        tokens.eat_space()
        nodes.extend(parse_type_struct(tokens, name=None, is_output=True).type_nodes)
    assign_type_node_oks(nodes)
    return TypeContent(name=name, tag=TypeTag.FUNCTION, type_nodes=nodes)


def _parse_redefinition(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a redefinition statement."""
    modifier = _parse_modifier_slot(tokens)
    symbol_type = tokens.eat_keyword_like(SymbolType)
    name = tokens.eat_identifier()
    tokens.eat_space()
    tokens.eat_separator("=")
    tokens.eat_space()
    reference = _parse_reference_slot(tokens)
    if modifier != StatementModifier.WITH:
        # only non-arg definitions can have children
        # this isn't great since it forbids re-aliasing but much easier to implement
        # since we would need to track the 'defining with contents' flag independently.
        tokens.eat_separator(":")
    tokens.eat_newline_or_eos()
    return Statement(
        type=StatementType.REDEFINITION,
        modifier=modifier,
        name=name.value,
        reference=reference,
        symbol_type=symbol_type.value if symbol_type else None,
        **kwargs,
    )


def _parse_redefinition_as_type_alias(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a type alias statement (special path because of special syntax)."""
    modifier = _parse_modifier_slot(tokens)
    tokens.eat_keyword(SymbolType.TYPE)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_space()
    tokens.eat_separator("=")
    tokens.eat_space()
    node = parse_simple_type_node_inline(tokens, name=None)
    tokens.eat_newline_or_eos()

    # like other "redefinitions", type aliases are just syntactic sugar for definitions
    statement = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        modifier=modifier,
        content=TypeContent(name=None, description=node.description, tag=node.tag),
        **kwargs,
    )
    return statement


def _parse_reference(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a reference statement."""
    modifier = _parse_modifier_slot(tokens)
    reference, symbol_type = _parse_typed_reference_slot(tokens)
    tokens.eat_newline_or_eos()
    return Statement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=reference.name,
        reference=reference,
        symbol_type=symbol_type.value,
        **kwargs,
    )


def _parse_description_line_optional(tokens: TokenParser) -> Optional[str]:
    """Parse an optional description line"""
    description = tokens.peek_description()
    if description is not None:
        tokens.eat_description()
        tokens.eat_newline()
    description = description.value if description else None
    return description


def _parse_modifier_slot(tokens: TokenParser) -> StatementModifier | None:
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError(
                ET.UNEXPECTED_TOKEN_VALUE, modifier, type=TT.KEYWORD, value=StatementModifier
            )
        tokens.eat_space()
    else:
        modifier = None
    return modifier


def _parse_blank(tokens: TokenParser, **kwargs) -> Statement:
    tokens.eat_newline()
    return Statement(type=StatementType.BLANK, **kwargs)


def _parse_statement(
    parser: TokenParser,
    is_root: bool,
    on_error: Callable[[ParseError], None],
    **statement_kwargs,
) -> Optional[Statement]:
    for _parse in [
        _parse_comment,
        _parse_definition_requirement,
        _parse_definition_type,
        _parse_definition_enum,
        _parse_definition,
        _parse_import,
        _parse_redefinition_as_type_alias,
        _parse_redefinition,
        _parse_reference,
        _parse_blank,
    ]:
        mark = parser.mark()
        try:
            statement = _parse(parser, **statement_kwargs)
            # check if there's a blank line after the end of a group
            could_be_group_end = (is_root and statement.ungrouped) or not is_root
            # we're at the end if there is an unintended token next
            if could_be_group_end and parser.peek_indent_level == 0:
                if not parser.peek_has_newline_or_eos():
                    on_error(ParseError(ET.EXPECTED_BLANK, parser.peek()))
            return statement
        except ParseError as e:
            e.parser = _parse.__name__
            e.position = parser.current_pos
            on_error(e)
            parser.reset(mark)
    return None


StmT = StatementType
SymT = SymbolType

SymbolContentT = typing.TypeVar("SymbolContentT", bound=SymbolContent)
SymbolT = typing.TypeVar("SymbolT", bound=InterpSymbol)


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
            return _error(ET.UNDEFINED_LOCAL_REFERENCE, path=reference)
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
        if isinstance(source_content, (DataContent, TaskContent, CodeContent)):
            # for typed symbols we need to create a type symbol as well
            symbol_kwargs["type"] = Type(
                name=statement.name,
                abstract=abstract,
                source=statement,
                tag=source_content.tag,
                type_nodes=source_content.type_nodes,
            )
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
        for child in node.type_nodes:
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
            # add all tasks in module for autobuilds :AutobuildTasks
            if symbol.source is not None and symbol.source.file.path == "instructors":
                for task in idx.symbols.values():
                    if (
                        isinstance(task, Task)
                        and task.source is not None
                        and task.source.file.module.id == symbol.source.file.module.id
                    ):
                        symbol.tasks.append(task)

    inlined_node_ids: set[UUID] = set()

    # inline union types (and extend expectations if they exist)
    def _inline_type_union_rec(node: TypeNode, path: list[TypeNode]) -> list[TypeNode]:
        if any(n.id == node.id for n in path):
            path = "->".join(str(n) for n in path + [node])
            _error(ET.CIRCULAR_UNION, node.source, path=path)
            return []
        if not isinstance(node, (Type, Task, Code, Data)) or node.id in inlined_node_ids:
            return node.type_nodes
        inlined_node_ids.add(node.id)
        if not any(n.flags & TypeFlag.IsUnionWith for n in node.type_nodes):
            return node.type_nodes  # skip, no unions
        path = path + [node]
        inlined_nodes = []
        node.self_type_nodes = deepcopy_types(node.type_nodes)  # retain originals
        for child in node.type_nodes:
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
        node.type_nodes = inlined_nodes
        return node.type_nodes

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
