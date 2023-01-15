from __future__ import annotations

import enum
import json
import re
import typing
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field
from typing import Callable, Optional
from uuid import UUID

import structlog

from bench.language.lex import SourceFile, Token, TokenType, get_location_range_pointer, lex
from bench.language.type import (
    Capability,
    Code,
    Compilation,
    Dataset,
    Expectation,
    File,
    Module,
    Requirement,
    Runconfig,
    Statement,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    Task,
    Type,
    TypeNode,
    TypeTag,
    Value,
    parse_statement_path,
    statement_path_as_str,
)
from bench.utils.record import RecordList

logger = structlog.get_logger(__name__)

UNGROUPED_STATEMENT_TYPES = (StatementType.DEFINITION, StatementType.REDEFINITION)


def raise_error(error: ValueError):
    raise error


def do_nothing(*args, **kwargs):
    pass


ErrorT = typing.TypeVar("ErrorT", bound=ValueError)


class ErrorCollector(typing.Generic[ErrorT]):
    def __init__(self, on_error: Callable[[ErrorT], None]):
        self.on_error = on_error
        self.errors: list[ErrorT] = []

    def __call__(self, error: ErrorT):
        self.errors.append(error)
        self.on_error(error)


class ParseErrorType(enum.Enum):
    MISSING_TOKEN = 0, "expected another token"
    UNEXPECTED_TOKEN_TYPE = 1, "expected token type {type}"
    UNEXPECTED_TOKEN_VALUE = 2, "expected token value {value}"
    UNEXPECTED_INDENT = 3, "expected indent <= {indent}"
    EXPECTED_BLANK = 4, "expected blank line"
    MISSING_EXTRA = 5, "expected token value extra {extra}"
    UNEXPECTED_EXTRA = 6, "unexpected token value extra {extra}={value}"
    INVALID_TOKEN_VALUE = 6, "invalid token value {value} {error}"
    INVALID_STATEMENT = 7, "invalid statement"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


PE = ParseErrorType
TT = TokenType


class ParseError(ValueError):
    def __init__(
        self,
        _t: ParseErrorType,
        token: Optional[Token],
        cause: Optional[Exception] = None,
        parser: Optional[str] = None,
        position: Optional[int] = -1,
        **error_args,
    ):
        super().__init__(self._to_message(_t, error_args, token, cause))
        self.type = _t
        self.token = token
        self.cause = cause
        self.position = position
        self.parser = parser
        self.error_args = error_args

    def _to_message(
        self, type: ParseErrorType, error_args: dict, token: Optional[Token], cause: Optional[dict]
    ):
        # noinspection StrFormat
        return "{0}: {1}{2}{3}".format(
            type.name,
            type.description.format(**error_args),
            ParseError.token_context(token),
            (f"\npossible cause: {cause}" if cause else ""),
        )

    @staticmethod
    def token_context(token: Optional[Token]) -> str:
        if token is None:
            return ""
        else:
            context = get_location_range_pointer(
                token.source_file,
                token.line_number,
                token.start_column,
                token.line_number + token.line_span,
                token.end_column,
            )
            return f" at {token}\n{context}"


def ignore_module_lookup(*args, **kwargs):
    return None


def error_module_lookup(*args, **kwargs):
    raise NotImplementedError("external module lookup disabled")


def parse(
    tokens: list[Token],
    module: Optional[Module] = None,
    lookup_module: Callable[[Requirement, StatementPath], Statement | None] = error_module_lookup,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> Module:
    """Parse a stream of tokens into Bench AST (grouped into files in a module)."""
    if on_error == "raise":
        on_error = raise_error
    if module is None:
        module = Module(name="<local>", files=[])

    # first pass: extract files and statements
    files = preparse(tokens, module, on_error=on_error)
    module.files.extend(files)

    # second pass: resolve statements into AST
    resolve(module, lookup_module=lookup_module, on_error=on_error)

    return module


def parse_string(
    string: str,
    module: Optional[Module] = None,
    lookup_module: Callable[[Requirement, StatementPath], Statement | None] = ignore_module_lookup,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> Module:
    tokens = lex(SourceFile(path="<string>", content=string))
    return parse(tokens, module=module, lookup_module=lookup_module, on_error=on_error)


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically matches (and ignores) indentation at the set level (may be changed).
    """

    def __init__(self, tokens: list[Token], start_pos: int, indent_level: int):
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
            raise ParseError(PE.MISSING_TOKEN, self.previous)
        peek_token = self.peek()
        self._advance()
        return peek_token

    def peek_type(self, type: TT) -> Optional[Token]:
        token = self.peek()
        return token if token is not None and token.type == type else None

    def eat_type(self, type: TT) -> Token:
        token = self.eat()
        if token.type != type:
            raise ParseError(PE.UNEXPECTED_TOKEN_TYPE, token, type=type)
        return token

    def eat_newfile(self) -> Token:
        return self.eat_type(TT.NEWFILE)

    def eat_newline(self) -> Token:
        return self.eat_type(TT.NEWLINE)

    def eat_newline_or_eos(self) -> Optional[Token]:
        if self.peek() is None or self.peek_type(TT.NEWFILE) is not None:
            return
        return self.eat_newline()

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
            raise ParseError(PE.UNEXPECTED_TOKEN_VALUE, token, type=TT.KEYWORD, value=keyword)
        return token

    def peek_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Optional[Token]:
        token = self.peek_type(TT.KEYWORD)
        if token is None or not isinstance(token.value, keyword_cls):
            return None
        return token

    def eat_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Token:
        token = self.eat_type(TT.KEYWORD)
        if not isinstance(token.value, keyword_cls):
            raise ParseError(PE.UNEXPECTED_TOKEN_VALUE, token, type=TT.KEYWORD, value=keyword_cls)
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
            raise ParseError(PE.UNEXPECTED_TOKEN_VALUE, token, type=TT.SEPARATOR, value=separator)
        return token

    def peek_bracket(self, bracket: str) -> Optional[Token]:
        token = self.peek_type(TT.BRACKET)
        if token is None or token.value != bracket:
            return None
        return token

    def eat_bracket(self, bracket: str) -> Token:
        token = self.eat_type(TT.BRACKET)
        if token is None or token.value != bracket:
            raise ParseError(PE.UNEXPECTED_TOKEN_VALUE, token, type=TT.BRACKET, value=bracket)
        return token

    def eat_space(self) -> Token:
        return self.eat_separator(" ")

    def eat_literal(self) -> Token:
        return self.eat_type(TT.LITERAL)

    def peek_description(self) -> Optional[Token]:
        return self.peek_type(TT.DESCRIPTION)

    def eat_description(self) -> Token:
        return self.eat_type(TT.DESCRIPTION)


def _clean_literal_indent(text: str, indent_level: int) -> str:
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
    def index(self) -> int:
        if self.parent is None:
            return len(self.root_statements)
        else:
            return len(self.children(self.parent))

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

    parser = TokenParser(tokens, start_pos=0, indent_level=0)
    states: dict[str, FileParseState] = OrderedDict()  # remember original file order
    # tokens[0] must be newfile
    initial_file: File = File(module=module, path=parser.eat_newfile().value)
    local = FileParseState(file=initial_file)
    states[initial_file.path] = local

    while parser.peek() is not None:
        # reset indent if previous token was on a different line
        if parser.previous is not None and parser.previous.line_number != parser.peek().line_number:
            local.indent = 0

        if parser.peek().type == TT.NEWFILE:
            path = parser.eat_newfile().value
            if path not in states:  # begin new file state
                new_file = File(module=module, path=path)
                states[path] = FileParseState(file=new_file)
            local = states[path]
        elif parser.peek().type == TT.INDENT:
            parser.eat()
            local.indent += 1
        else:  # parse statement
            if len(local.ancestors) < local.indent:  # too much indentation
                raise ParseError(ParseErrorType.UNEXPECTED_INDENT, parser.peek())

            errors: list[ParseError] = []
            parser.indent_level = local.indent  # skip indent tokens at current level
            start_mark = parser.mark()
            statement = _parse_statement(
                parser,
                is_root=local.root,
                on_error=errors.append,
                file=local.file,
                parent=local.parent,
                index=local.index,
            )
            parser.indent_level = 0  # skip only for statement parsing

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
                error = ParseError(PE.INVALID_STATEMENT, parser.peek(), cause=cause)
                on_error(error)
                continue

            statement._source = parser.eaten(start_mark)
            local.add_statement(statement)

    return [state.file for state in states.values()]


def _parse_comment(tokens: TokenParser, **kwargs) -> Statement:
    token = tokens.eat_type(TT.COMMENT)
    tokens.eat_newline_or_eos()
    return Statement(type=StatementType.COMMENT, text=token.value, **kwargs)


# import source must either be in current module (.*) or absolute (<owner>.<name>.*)
RELATIVE_IMPORT_SOURCE_REGEX = re.compile(r"^\.(?P<path>[\w.-]+)$")
ABSOLUTE_IMPORT_SOURCE_REGEX = re.compile(
    r"^(?P<owner>[\w-]+)\.(?P<name>[\w-]+)\.(?P<path>[\w.-]+)$"
)


def is_valid_import_source(source: str) -> bool:
    return (
        RELATIVE_IMPORT_SOURCE_REGEX.match(source) is not None
        or ABSOLUTE_IMPORT_SOURCE_REGEX.match(source) is not None
    )


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
    if not is_valid_import_source(source.value):
        raise ParseError(PE.INVALID_TOKEN_VALUE, source, value=source.value, error="invalid")
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
    definition.content = _parse_definition_content(tokens, symbol_type, name, definition)
    tokens.eat_newline_or_eos()
    return definition


def _parse_definition_content(
    tokens: TokenParser, symbol_type: Token, name: Token, definition: Statement
) -> SymbolContent:
    if symbol_type.value == SymbolType.CAPABILITY:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return Capability(description=description.value, definition=definition)
    elif symbol_type.value == SymbolType.TASK:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        func_type = parse_type_node_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return Task(description=description.value, func_type=func_type, definition=definition)
    elif symbol_type.value == SymbolType.EXPECTATION:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return Expectation(description=description.value, definition=definition)
    elif symbol_type.value == SymbolType.CODE:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        func_type = parse_type_node_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        if lang is None:
            raise ParseError(PE.MISSING_EXTRA, literal, extra="lang")
        if lang != "python":
            raise ParseError(PE.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
        code_text = _clean_literal_indent(literal.value, tokens.indent_level)
        return Code(
            language=lang,
            builtin_id=None,
            code=code_text,
            func_type=func_type,
            definition=definition,
        )
    elif symbol_type.value == SymbolType.DATASET:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        element_type = parse_type_node_struct_inline(tokens, name="element")
        tokens.eat_separator(":")
        tokens.eat_newline()
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        try:
            if lang is None:
                raise ParseError(PE.MISSING_EXTRA, literal, extra="lang")
            elif lang == "jsonl":
                records = []
                for value_line in literal.value.strip().splitlines():
                    records.append(json.loads(value_line.strip()))
            elif lang == "json":
                records = json.loads(literal.value)
            else:
                raise ParseError(PE.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
            return Dataset(
                records=RecordList(records), element_type=element_type, definition=definition
            )
        except json.JSONDecodeError as e:
            raise ParseError(PE.INVALID_TOKEN_VALUE, literal, error=e)
    elif symbol_type.value == SymbolType.VALUE:
        tokens.eat_separator(":")
        tokens.eat_newline()
        literal = tokens.eat_literal()
        try:  # parse as json
            value = json.loads(literal.value)
            return Value(value=value, definition=definition)
        except json.JSONDecodeError as e:
            raise ParseError(PE.INVALID_TOKEN_VALUE, literal, error=e)
    elif symbol_type.value == SymbolType.COMPILATION:
        tokens.eat_separator(":")
        return Compilation(definition=definition)
    # we don't parse SymbolType.REQUIREMENT here because it looks different, see below
    elif symbol_type.value == SymbolType.RUNCONFIG:
        tokens.eat_separator(":")
        return Runconfig(definition=definition)

    raise ParseError(PE.UNEXPECTED_TOKEN_VALUE, symbol_type, type=TT.KEYWORD, value=SymbolType)


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
    definition.content = Requirement(
        name=dependency.value, version=version.value, definition=definition
    )
    return definition


def _parse_definition_type(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a type statement (special path because of special syntax)."""
    tokens.eat_keyword(SymbolType.TYPE)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    definition = Statement(
        type=StatementType.DEFINITION, symbol_type=SymbolType.TYPE, name=name.value, **kwargs
    )
    tokens.eat_newline()
    description = tokens.peek_description()
    if description is not None:
        tokens.eat_description()
        tokens.eat_newline()
    struct = parse_type_node_struct(tokens, name=definition.name)
    tokens.eat_newline_or_eos()
    definition.content = Type(
        definition=definition,
        node=struct,
        description=description.value if description else None,
    )
    return definition


def parse_type_node_named(tokens: TokenParser) -> TypeNode:
    """parse single tuple like <name>: <type>[ "<description>"]"""
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_space()
    return parse_type_node(tokens, name.value)


def parse_type_node(tokens: TokenParser, name: str | None, packing: bool = False) -> TypeNode:
    """Parse a type node type including description, handling nested types."""
    # TODO @Cleanup: parse_type_node_type seems more complex than it should be,
    #  especially the nested back-tracking for unions/intersections

    # parse array like [<type>] with recursive descent
    if tokens.peek_bracket("["):
        tokens.eat_bracket("[")
        node = parse_type_node(tokens, name=None)
        tokens.eat_bracket("]")
        return TypeNode(name=name, type=TypeTag.ARRAY, children=[node])

    start_mark = tokens.mark()  # for back-tracking
    # parse actual type as either primitive or type reference
    if tokens.peek_keyword_like(TypeTag):
        # not all value types are keywords, but only the valid ones are in KEYWORDS
        type = tokens.eat_keyword_like(TypeTag).value
        reference = None
    else:
        type = TypeTag.TYPE_REFERENCE
        reference = tokens.eat_identifier().value

    if not tokens.peek_separator(" "):  # type is done
        return TypeNode(name=name, type=type, reference=reference, source_reference=reference)

    tokens.eat_space()
    if tokens.peek_description():  # description completes type declaration
        description = tokens.eat_description().value
        return TypeNode(
            name=name,
            type=type,
            reference=reference,
            source_reference=reference,
            description=description,
        )
    # parse post-packed types like unions and intersection with back-tracking
    elif tokens.peek_separator("|") or tokens.peek_separator("&"):
        if packing:  # inner type is done, so this must refer to parent packing
            tokens.advance(-1)  # go back one token to leave whitespace separator
            return TypeNode(name=name, type=type, reference=reference, source_reference=reference)
        # otherwise we're starting to pack a new union/intersection
        packing_separator = tokens.eat().value
        packing_type = TypeTag.UNION if packing_separator == "|" else TypeTag.INTERSECTION
        tokens.reset(start_mark)  # back-track and reparse all children in one go
        parent = TypeNode(name=name, type=packing_type, children=[])
        while True:
            node = parse_type_node(tokens, name=None, packing=True)
            parent.children.append(node)
            # if we got a description, we are done
            if node.description is not None:  # hoist description to parent
                parent.description = node.description
                node.description = None
                break
            # otherwise, try to parse another node
            if tokens.peek_separator(" "):
                tokens.eat_space()
                tokens.eat_separator(packing_separator)
                tokens.eat_space()
            else:
                break
        return parent
    else:
        raise ParseError(PE.UNEXPECTED_TOKEN_TYPE, tokens.peek(), type="| or &")


def parse_type_node_struct(tokens: TokenParser, name: str) -> TypeNode:
    # parse tuples like <tuple1>\n<tuple2>\n...
    struct = TypeNode(name=name, type=TypeTag.STRUCT, children=[])
    while True:
        tuple = parse_type_node_named(tokens)
        struct.children.append(tuple)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()
        if not tokens.peek_type(TokenType.IDENTIFIER):
            tokens.advance(-1)  # go back one token to leave newline separator
            break
    return struct


def parse_type_node_struct_inline(tokens: TokenParser, name: str) -> TypeNode:
    tokens.eat_bracket("(")
    struct = TypeNode(name=name, type=TypeTag.STRUCT, children=[])
    while not tokens.peek_bracket(")"):
        tuple = parse_type_node_named(tokens)
        struct.children.append(tuple)
        if not tokens.peek_separator(","):
            break
        tokens.eat_separator(",")
        tokens.eat_space()
    tokens.eat_bracket(")")
    return struct


def parse_type_node_func(tokens: TokenParser, name: str) -> TypeNode:
    # parse signature like (<tuple1>, <tuple2>, ...) -> <return_tuple>
    input = parse_type_node_struct_inline(tokens, "input")
    if tokens.peek_separator(" "):
        tokens.eat_space()
        tokens.eat_separator("->")
        tokens.eat_space()
        output = parse_type_node(tokens, "output")
    else:
        output = TypeNode(name="output", type=TypeTag.NULL)
    return TypeNode(name=name, type=TypeTag.FUNCTION, children=[input, output])


def _parse_redefinition(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a redefinition statement."""
    modifier = _parse_modifier_slot(tokens)
    name, symbol_type = _parse_reference_slot(tokens)
    tokens.eat_space()
    tokens.eat_separator("=")
    tokens.eat_space()
    reference_name, other_symbol_type = _parse_reference_slot(tokens)
    # defined symbol type and referenced symbol type must match
    if symbol_type.value != other_symbol_type.value:
        raise ParseError(
            PE.UNEXPECTED_TOKEN_VALUE, reference_name, type=TT.IDENTIFIER, value=symbol_type
        )
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
        reference=StatementPath(".", reference_name.value),
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
    node = parse_type_node(tokens, name=None)  # name corresponds to statement, not type node
    tokens.eat_newline_or_eos()

    # like other "redefinitions", type aliases are just syntactic sugar for definitions
    statement = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        modifier=modifier,
        **kwargs,
    )
    statement.content = Type(definition=statement, node=node)
    return statement


def _parse_reference(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a reference statement."""
    modifier = _parse_modifier_slot(tokens)
    name, symbol_type = _parse_reference_slot(tokens)
    tokens.eat_newline_or_eos()
    return Statement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=name.value,
        reference=StatementPath(".", name.value),
        symbol_type=symbol_type.value,
        **kwargs,
    )


def _parse_modifier_slot(tokens: TokenParser) -> StatementModifier | None:
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError(
                PE.UNEXPECTED_TOKEN_VALUE, modifier, type=TT.KEYWORD, value=StatementModifier
            )
        tokens.eat_space()
    else:
        modifier = None
    return modifier


def _parse_reference_slot(tokens: TokenParser) -> tuple[Token, Token]:
    # references can be either to symbol types or compile and run statements
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    name = tokens.eat_identifier()
    return name, symbol_type


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
                    on_error(ParseError(PE.EXPECTED_BLANK, parser.peek()))
            return statement
        except ParseError as e:
            e.parser = _parse.__name__
            e.position = parser.current_pos
            on_error(e)
            parser.reset(mark)
    return None


class SemanticErrorType(enum.Enum):
    UNKNOWN_IMPORT_SOURCE = 0, "unspecified import module source {source}"
    UNDEFINED_LOCAL_REFERENCE = 1, "undefined reference {path}"
    UNDEFINED_EXTERNAL_REFERENCE = 2, "undefined external reference {path} in module {module}"
    EXTERNAL_LOOKUP_FAILED = 3, "failed to lookup reference {path} in module {module}: {error}"
    REFERENCE_TYPE_MISMATCH = 4, "reference {resolved} is not of type {resolved}"
    AMBIGUOUS_DEFINITION = 5, "multiple definitions for {path}"
    AMBIGUOUS_REQUIREMENT = 6, "multiple requirements for {name}"
    UNEXPECTED_PARENT = 7, "unexpected parent {parent}"
    EXPECTED_PARENT = 8, "expected a parent"
    EXPECTED_PROPER_CHILDREN = 9, "expected proper children"
    UNEXPECTED_CHILDREN = 10, "unexpected children"
    EXPECTED_PARAMETERS = 11, "expected parameters of type {type}"
    UNEXPECTED_PARAMETERS = 12, "unexpected parameters"
    EXPECTED_ARGUMENTS = 13, "expected arguments of type {type}"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


StmT = StatementType
SymT = SymbolType
SE = SemanticErrorType


class SemanticError(ValueError):
    def __init__(
        self,
        _t: SemanticErrorType,
        statement: Optional[Statement],
        cause: Optional[Exception] = None,
        **error_args,
    ):
        super().__init__(self._format_message(_t, error_args, statement, cause))
        self.type = _t
        self.statement = statement
        self.related_statements = {k: v for k, v in error_args.items() if isinstance(v, Statement)}
        self.cause = cause

    def _format_message(
        self,
        error_type: SemanticErrorType,
        error_args: dict,
        statement: Statement,
        cause: Exception | None,
    ) -> str:
        # convert error args as needed
        # StatementPath with statement_path_as_str
        error_args = {
            k: statement_path_as_str(v) if isinstance(v, StatementPath) else v
            for k, v in error_args.items()
        }
        # noinspection StrFormat
        message = error_type.description.format(**error_args)
        cause_context = f"\ncause: {cause.__class__.__name__} {cause}" if cause is not None else ""
        return message + SemanticError.statement_context(statement) + cause_context

    @staticmethod
    def statement_context(statement: Optional[Statement]) -> str:
        if statement is None:
            return ""
        elif statement._source is None:
            return "<source unavailable>"
        else:
            source = typing.cast(list[Token], statement._source)
            source_context = get_location_range_pointer(
                source[0].source_file,
                source[0].line_number,
                source[0].start_column,
                source[-1].line_number + source[-1].line_span,
                source[-1].end_column,
            )
            return f" at\n> {statement}\n{source[0].source_file.path}:{source[0].line_number}\n{source_context}"


SymbolContentT = typing.TypeVar("SymbolContentT", bound=SymbolContent)


@dataclass(repr=False)
class ModuleIndex:
    module: Module
    requirements_by_name: dict[str, Requirement] = field(default_factory=dict)
    statements_by_id: dict[UUID, Statement] = field(default_factory=OrderedDict)
    statements_by_path: dict[StatementPath, Statement] = field(default_factory=dict)
    statements_by_parent: dict[UUID, list[Statement]] = field(
        default_factory=lambda: defaultdict(list)
    )

    def statement(self, path: StatementPath | str) -> Statement:
        if isinstance(path, str):
            path = parse_statement_path(path)
        return self.statements_by_path[path]

    def symbol(
        self, path: StatementPath | str, symbol_t: typing.Type[SymbolContentT]
    ) -> SymbolContentT:
        statement = self.statement(path)
        if statement.content is None:
            raise ValueError(f"statement {statement} has no content")
        if not isinstance(statement.content, symbol_t):
            raise ValueError(f"statement {statement} has content of type {type(statement.content)}")
        return typing.cast(SymbolContentT, statement.content)

    def children(self, statement: Statement) -> list[Statement]:
        return self.statements_by_parent[statement.id]


def resolve(
    module: Module,
    lookup_module: Callable[[Requirement, StatementPath], Statement | None],
    on_error: Callable[[SemanticError], None],
) -> ModuleIndex:
    """Resolve unresolved references in the given module."""

    idx = index_module(module, on_error=on_error)

    # resolve references to other statements
    for statement in idx.statements_by_id.values():
        resolve_statement_reference(statement, idx, lookup_module, on_error)

    # resolve references in types (to other statements)
    for statement in idx.statements_by_id.values():
        # (on all statements that have types in their content)
        if isinstance(statement.content, Type):
            resolve_type_references(statement, statement.content.node, idx, on_error)
        elif isinstance(statement.content, Dataset):
            resolve_type_references(statement, statement.content.element_type, idx, on_error)
        elif isinstance(statement.content, Task):
            resolve_type_references(statement, statement.content.func_type, idx, on_error)
        elif isinstance(statement.content, Code):
            resolve_type_references(statement, statement.content.func_type, idx, on_error)

    # check other semantic issues
    # TODO @Cleanup: not sure where to put non-resolution semantic checking
    #  And what about lints and such? Probably separate.. but where?
    for statement in idx.statements_by_id.values():
        check_statement(statement, idx, on_error)

    return idx


def check_statement(
    statement: Statement, idx: ModuleIndex, on_error: Callable[[SemanticError], None]
):
    def _error(
        _t: SemanticErrorType, statement: Statement, cause: Exception | None = None, **error_args
    ):
        on_error(SemanticError(_t, statement, cause, **error_args))

    all_children = idx.statements_by_parent[statement.id]
    parameters = [s for s in all_children if s.is_parameter]
    arguments = [s for s in all_children if s.is_argument]
    proper_children = [s for s in all_children if not s.is_parameter and not s.is_argument]

    # check that arguments and parameters have a parent
    if (statement.is_argument or statement.is_parameter) and statement.parent is None:
        _error(SE.EXPECTED_PARENT, statement)

    # check that compile definition has proper children and model parameters
    if statement.type == StmT.DEFINITION and statement.symbol_type == SymT.COMPILATION:
        if len(proper_children) == 0:
            _error(SE.EXPECTED_PROPER_CHILDREN, statement)

    # check that runconfig has arguments
    if statement.type == StmT.DEFINITION and statement.symbol_type == SymT.RUNCONFIG:
        if len(proper_children) == 0:
            _error(SE.EXPECTED_PROPER_CHILDREN, statement, type="any")


def resolve_statement_reference(
    statement: Statement,
    idx: ModuleIndex,
    lookup_module: Callable[[Requirement, StatementPath], Statement | None],
    on_error: Callable[[SemanticError], None],
) -> None:
    def _error(_t: SemanticErrorType, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, statement, cause, **error_args))

    if not isinstance(statement.reference, StatementPath) or statement.is_parameter:
        return  # need not be resolved

    # normalize path to resolve file-local references (with .)
    normalized_path = statement.reference
    if statement.reference.path == ".":  # current file
        normalized_path = StatementPath(
            f".{statement.file.path_without_extension}", statement.reference.name
        )

    if normalized_path.path.startswith("."):  # resolve in local module
        resolved = idx.statements_by_path.get(normalized_path)
        if resolved is None:
            _error(SE.UNDEFINED_LOCAL_REFERENCE, path=normalized_path)
            return
    else:  # resolve in external module
        # get source requirement
        source = ABSOLUTE_IMPORT_SOURCE_REGEX.match(normalized_path.path)
        if source is None:  # (should be caught in parse)
            raise RuntimeError(f"invalid import source at {statement}")
        requirement_name = f"{source.group('owner')}.{source.group('name')}"
        requirement = idx.requirements_by_name.get(requirement_name)
        if requirement is None:
            _error(SE.UNKNOWN_IMPORT_SOURCE, source=requirement_name)
            return
        # localize path to requirement module
        localized_path = StatementPath("." + source.group("path"), normalized_path.name)
        try:  # use module lookup to resolve
            resolved = lookup_module(requirement, localized_path)
        except Exception as e:
            _error(SE.EXTERNAL_LOOKUP_FAILED, error=e, path=localized_path, module=requirement)
            return
        if resolved is None:
            _error(SE.UNDEFINED_EXTERNAL_REFERENCE, path=localized_path, module=requirement.name)
            return

    statement.reference = resolved
    # check if the reference has the correct type
    if resolved.symbol_type != statement.symbol_type:
        _error(SE.REFERENCE_TYPE_MISMATCH, type=statement.symbol_type, resolved=resolved)


def resolve_type_references(
    statement: Statement,
    node: TypeNode,
    idx: ModuleIndex,
    on_error: Callable[[SemanticError], None],
) -> None:
    def _error(_t: SemanticErrorType, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, statement, cause, **error_args))

    # walk through child nodes
    if node.children is not None:
        for child in node.children:
            resolve_type_references(statement, child, idx, on_error)

    if not isinstance(node.reference, str):
        return  # nothing to resolve

    # normalize path to statement
    normalized_path = StatementPath("." + statement.file.path_without_extension, node.reference)
    resolved_stmt = idx.statements_by_path.get(normalized_path)
    if resolved_stmt is None or resolved_stmt.underlying_definition is None:
        _error(SE.UNDEFINED_LOCAL_REFERENCE, path=normalized_path)
        return
    if resolved_stmt.symbol_type != SymT.TYPE:
        _error(SE.REFERENCE_TYPE_MISMATCH, type=SymT.TYPE, resolved=resolved_stmt)
        return

    # get type node from statement
    # right now we get the underlying definition directly, ignoring intermediate arguments
    resolved_type = typing.cast(Type, resolved_stmt.underlying_definition.content)
    node.reference = resolved_type.node

    # impute type references (also in place)
    impute_type_references(node, keep_references=True)


def impute_type_references(node: TypeNode, keep_references: bool = True) -> None:
    """
    Replace all references with their definitions.
    Note that without references, perfect source reconstruction is impossible.
    """
    if node.type != TypeTag.TYPE_REFERENCE:
        return

    if not isinstance(node.reference, TypeNode):
        raise ValueError(f"type reference is not resolved: {node}")

    # error? if reference is an unresolved reference
    if node.reference.type == TypeTag.TYPE_REFERENCE:
        # TODO @Incomplete: could just impute that as well? but then we'd need to break circles?
        raise ValueError(f"reference is unresolved type reference: {node}")

    node.type = node.reference.type
    node.children = node.reference.children
    if node.children is not None:
        for node in node.children:
            impute_type_references(node, keep_references)
    if not keep_references:
        node.reference = None


def index_module(
    module: Module,
    on_error: Callable[[SemanticError], None] | typing.Literal["raise"] = "raise",
) -> ModuleIndex:
    if on_error == "raise":
        on_error = raise_error

    def _error(
        _t: SemanticErrorType, statement: Statement, cause: Exception | None = None, **error_args
    ):
        on_error(SemanticError(_t, statement, cause, **error_args))

    idx = ModuleIndex(module=module)

    # collect files and statements
    for file in module.files:
        for statement in file.statements:
            idx.statements_by_id[statement.id] = statement
            if statement.parent is not None:
                idx.statements_by_parent[statement.parent.id].append(statement)
            if statement.referable and statement.parent is None:
                # TODO @Feature: make non-top-level statements referable
                #  Will need to change resolution with StatementPaths in indexed module.
                statement_path = StatementPath(f".{file.path_without_extension}", statement.name)
                if statement_path in idx.statements_by_path:
                    _error(SE.AMBIGUOUS_DEFINITION, statement, path=statement_path)
                    continue
                idx.statements_by_path[statement_path] = statement
    # sort statements by parent by index (for deterministic resolution)
    for statements in idx.statements_by_parent.values():
        statements.sort(key=lambda s: s.index)

    # collect requirements
    for statement in idx.statements_by_id.values():
        if statement.symbol_type == SymT.REQUIREMENT:
            requirement_name = statement.name
            if requirement_name in idx.requirements_by_name:
                _error(SE.AMBIGUOUS_REQUIREMENT, statement, name=requirement_name)
                continue
            idx.requirements_by_name[requirement_name] = typing.cast(Requirement, statement.content)

    return idx
