from __future__ import annotations

import csv
import enum
import json
import re
import typing
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field
from typing import Callable, Optional
from uuid import UUID

import structlog

from bench.language.error import ErrorType, ParseError, SemanticError
from bench.language.lex import lex_string
from bench.language.type import (
    CapabilityContent,
    CodeContent,
    CompilationContent,
    DatasetContent,
    ExpectationContent,
    File,
    InterpSymbol,
    LiteralValue,
    Module,
    RequirementContent,
    RunconfigContent,
    Statement,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    TaskContent,
    Token,
    TokenType,
    TypeContent,
    TypeNode,
    TypeTag,
    ValueContent,
    parse_statement_path,
)

logger = structlog.get_logger(__name__)

UNGROUPED_STATEMENT_TYPES = (StatementType.DEFINITION, StatementType.REDEFINITION)


def raise_error(error: ValueError):
    raise error


def do_nothing(*args, **kwargs):
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


def parse(
    tokens: list[Token],
    module: Optional[Module] = None,
    lookup_in_module: Callable[
        [RequirementContent, StatementPath], Statement | None
    ] = lookup_in_error,
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

    # second pass: resolve statement references
    idx = resolve(module, lookup_in_module=lookup_in_module, on_error=on_error)

    # third pass: create symbols
    interp(idx)

    return module


def parse_string(
    string: str,
    module: Optional[Module] = None,
    lookup_in_module: Callable[
        [RequirementContent, StatementPath], Statement | None
    ] = ignore_module_lookup,
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> Module:
    tokens = lex_string(string)
    return parse(tokens, module=module, lookup_in_module=lookup_in_module, on_error=on_error)


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically matches (and ignores) indentation at the set level (mutable).
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

        if statement.type not in (StatementType.BLANK, StatementType.COMMENT):
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
                raise ParseError(ET.UNEXPECTED_INDENT, parser.peek())

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
                error = ParseError(ET.INVALID_STATEMENT, parser.peek(), cause=cause)
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
        type = parse_type_node_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return TaskContent(description=description.value, type_node=type)
    elif symbol_type.value == SymbolType.EXPECTATION:
        on_location = None
        if tokens.peek_separator(" "):
            tokens.eat_space()
            tokens.eat_keyword("on")
            tokens.eat_space()
            # very simple location parser (single identifier)
            on_location = tokens.eat_identifier().value
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return ExpectationContent(description=description.value, on=on_location)
    elif symbol_type.value == SymbolType.CODE:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_node_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        if lang not in ("python", "bpl"):
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
        code_text = _clean_literal_indent(literal.value, tokens.indent_level)
        return CodeContent(
            description=description,
            language=lang,  # noqa
            builtin_id=None,
            code=code_text,
            type_node=type,
        )
    elif symbol_type.value == SymbolType.DATASET:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        tokens.eat_bracket("(")
        type = parse_type_node_struct_inline(tokens, name="element")
        tokens.eat_bracket(")")
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        records = _parse_dataset_records(tokens, type, lang, literal)
        return DatasetContent(
            description=description,
            language=lang,
            records=records,
            type_node=type,
        )
    elif symbol_type.value == SymbolType.VALUE:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        try:  # parse as json?
            value = json.loads(literal.value)
            return ValueContent(description=description, value=value)
        except json.JSONDecodeError as e:
            raise ParseError(ET.INVALID_TOKEN_VALUE, literal, error=e)
    elif symbol_type.value == SymbolType.COMPILATION:
        tokens.eat_separator(":")
        return CompilationContent(source_mappings=[])
    # we don't parse SymbolType.REQUIREMENT here because it looks different, see below
    elif symbol_type.value == SymbolType.RUNCONFIG:
        tokens.eat_separator(":")
        return RunconfigContent()

    raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, symbol_type, type=TT.KEYWORD, value=SymbolType)


def _parse_dataset_records(
    tokens: TokenParser, type: TypeNode, lang: str | None, literal: Token
) -> list[dict[str, LiteralValue]]:
    """Parses the language and records from a dataset literal."""
    value_str = _clean_literal_indent(literal.value, tokens.indent_level)
    try:
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        elif lang == "jsonl":
            records = [json.loads(line) for line in value_str.splitlines()]
        elif lang == "json":
            records = json.loads(value_str)
        elif lang == "csv":
            field_names = [element.name for element in type.children]
            csv_reader = csv.DictReader(
                value_str.splitlines(), quoting=csv.QUOTE_NONNUMERIC, fieldnames=field_names
            )
            records = list(csv_reader)
        else:
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
    except ParseError:
        raise  # re-raise since we don't want to catch our own errors
    except ValueError as e:
        raise ParseError(ET.INVALID_TOKEN_VALUE, literal, value=value_str, error=e)
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
    definition.content = RequirementContent(name=dependency.value, version=version.value)
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
    description = _parse_description_line_optional(tokens)
    struct = parse_type_node_struct(tokens, name=definition.name)
    tokens.eat_newline_or_eos()
    definition.content = TypeContent(
        type_node=struct,
        description=description,
    )
    return definition


def _parse_definition_enum(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a value enum statement (special path because of special syntax)"""
    tokens.eat_keyword(TypeTag.ENUM)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_space()
    tokens.eat_separator("::")
    tokens.eat_space()
    if tokens.peek_bracket("("):
        tokens.eat_bracket("(")
        member_type_node = parse_type_node_struct_inline(tokens, name=None)
        tokens.eat_bracket(")")
    else:
        member_type_node = parse_type_node_inline(tokens, name=None)
    tokens.eat_separator(":")
    tokens.eat_newline()
    description = _parse_description_line_optional(tokens)

    # parse members (assumes literal members only)
    members: list[TypeNode] = []
    while True:
        member_name = tokens.eat_identifier().value
        tokens.eat_space()
        tokens.eat_separator("=")
        tokens.eat_space()
        member_literal = tokens.eat_literal()
        try:
            member_value = json.loads(member_literal.value)
        except ValueError as e:
            raise ParseError(
                ET.INVALID_TOKEN_VALUE, member_literal, error=e, value=member_literal.value
            )

        member_description = None
        if tokens.peek_separator(" "):
            tokens.eat_space()
            member_description = tokens.eat_description().value
        member = TypeNode(
            name=member_name,
            type=TypeTag.LITERAL,
            value=member_value,
            description=member_description,
        )
        members.append(member)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()
        if not tokens.peek_type(TokenType.IDENTIFIER):
            tokens.advance(-1)  # go back one token to leave newline separator
            break

    tokens.eat_newline_or_eos()
    enum_type_node = TypeNode(name=None, type=TypeTag.ENUM, children=[member_type_node, *members])

    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        content=TypeContent(
            type_node=enum_type_node,
            description=description,
        ),
        **kwargs,
    )
    return definition


def parse_type_node_named(tokens: TokenParser) -> TypeNode:
    """parse single tuple like <name>: <type>[ "<description>"]"""
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_space()
    return parse_type_node_inline(tokens, name.value)


def parse_type_node_inline(
    tokens: TokenParser, name: str | None, packing: bool = False
) -> TypeNode:
    """Parse a type node type including description, handling nested types."""
    # TODO @Cleanup: parse_type_node_type seems more complex than it should be,
    #  especially the nested back-tracking for unions/intersections

    # parse array like [<type>] with recursive descent
    if tokens.peek_bracket("["):
        tokens.eat_bracket("[")
        node = parse_type_node_inline(tokens, name=None)
        tokens.eat_bracket("]")
        description = None
        if tokens.peek_separator(" "):
            tokens.eat_space()
            if tokens.peek_description():
                description = tokens.eat_description().value
            else:  # turn back, wasn't for us to eat
                tokens.advance(-1)
        return TypeNode(name=name, type=TypeTag.ARRAY, description=description, children=[node])

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
            node = parse_type_node_inline(tokens, name=None, packing=True)
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
        raise ParseError(ET.UNEXPECTED_TOKEN_TYPE, tokens.peek(), type="| or &")


def parse_type_node_struct(tokens: TokenParser, name: str | None) -> TypeNode:
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


def parse_type_node_struct_inline(tokens: TokenParser, name: str | None) -> TypeNode:
    struct = TypeNode(name=name, type=TypeTag.STRUCT, children=[])
    while not tokens.peek_bracket(")"):
        tuple = parse_type_node_named(tokens)
        struct.children.append(tuple)
        if not tokens.peek_separator(","):
            break
        tokens.eat_separator(",")
        tokens.eat_space()
    return struct


def parse_type_node_func(tokens: TokenParser, name: str | None) -> TypeNode:
    # parse signature like (<tuple1>, <tuple2>, ...) -> <return_tuple>
    tokens.eat_bracket("(")
    input = parse_type_node_struct_inline(tokens, "input")
    tokens.eat_bracket(")")
    if tokens.peek_separator(" "):
        tokens.eat_space()
        tokens.eat_separator("->")
        tokens.eat_space()
        output = parse_type_node_inline(tokens, "output")
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
            ET.UNEXPECTED_TOKEN_VALUE, reference_name, type=TT.IDENTIFIER, value=symbol_type
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
    node = parse_type_node_inline(tokens, name=None)  # name corresponds to statement, not type node
    tokens.eat_newline_or_eos()

    # like other "redefinitions", type aliases are just syntactic sugar for definitions
    statement = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        modifier=modifier,
        content=TypeContent(description=None, type_node=node),
        **kwargs,
    )
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

    def lookup_statement(self, name: str) -> Statement | None:
        """Lookup the statement recursively in this scope and its parents."""
        if name in self.statements:
            return self.statements[name]
        if self.parent is not None:
            return self.parent.lookup_statement(name)
        return None

    def __str__(self):
        return f"{self.name} ({'file' if self.statement is None else 'statement'})"

    def __repr__(self):
        return f"<Scope {self.name}>"


@dataclass(repr=False)
class ModuleIndex:
    """The index into a module's resolved and interpreted symbols and statements."""

    module: Module
    requirements_by_name: dict[str, RequirementContent] = field(default_factory=dict)
    statements_by_id: dict[UUID, Statement] = field(default_factory=OrderedDict)
    scopes: OrderedDict[UUID, Scope] = field(default_factory=OrderedDict)
    scopes_by_name: OrderedDict[str, Scope] = field(default_factory=OrderedDict)

    def add_scope(self, scope: Scope) -> None:
        self.scopes[scope.id] = scope
        self.scopes_by_name[scope.name] = scope

    def get_statement(self, path: StatementPath | str) -> Statement | None:
        if isinstance(path, str):
            path = parse_statement_path(path)
        scope = self.scopes_by_name.get(path.path[1:])  # skip initial dot
        if scope is None:
            return None
        return scope.statements.get(path.name)

    def symbol(
        self, path: StatementPath | str, symbol_t: typing.Type[SymbolT] | None = None
    ) -> SymbolT:
        if isinstance(path, str):
            path = parse_statement_path(path)
        raise NotImplementedError


def resolve(
    module: Module,
    lookup_in_module: Callable[[RequirementContent, StatementPath], Statement | None],
    on_error: Callable[[SemanticError], None],
) -> ModuleIndex:
    """Resolve unresolved references in the given module."""

    idx = index_module(module, on_error=on_error)

    # resolve references to other statements
    for statement in idx.statements_by_id.values():
        resolve_statement_reference(statement, idx, lookup_in_module, on_error)

    # resolve references in types (to other statements)
    for statement in idx.statements_by_id.values():
        if isinstance(statement.content, (TypeContent, DatasetContent, TaskContent, CodeContent)):
            resolve_type_references(statement, statement.content.type_node, idx, on_error)

    return idx


def resolve_statement_reference(
    statement: Statement,
    idx: ModuleIndex,
    lookup_in_module: Callable[[RequirementContent, StatementPath], Statement | None],
    on_error: Callable[[SemanticError], None],
) -> None:
    def _error(_t: ET, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, statement, cause, **error_args))

    if not isinstance(statement.reference, StatementPath) or statement.is_parameter:
        return  # need not be resolved

    # normalize path to resolve file-local references (with .)
    # :StatementReferencePath
    reference_path, reference_name = statement.reference
    if reference_path == ".":  # resolve relative to this statement
        statement_scope = idx.scopes[statement.id]
        resolved = statement_scope.lookup_statement(reference_name)
        if resolved is None:
            _error(ET.UNDEFINED_LOCAL_REFERENCE, path=statement.reference)
            return
    elif reference_path.startswith("."):  # resolve by "absolute" path in local module
        parent_scope = idx.scopes_by_name[reference_path[1:]]
        resolved = parent_scope.lookup_statement(reference_name)
        if resolved is None:
            _error(ET.UNDEFINED_LOCAL_REFERENCE, path=statement.reference)
            return
    else:  # resolve by absolute path in external module
        # get source requirement
        source = ABSOLUTE_IMPORT_SOURCE_REGEX.match(reference_path)
        if source is None:  # (should be caught in parse)
            raise RuntimeError(f"invalid import source at {statement}")
        requirement_name = f"{source.group('owner')}.{source.group('name')}"
        requirement = idx.requirements_by_name.get(requirement_name)
        if requirement is None:
            _error(ET.UNKNOWN_IMPORT_SOURCE, source=requirement_name)
            return
        # localize path to requirement module
        localized_path = StatementPath("." + source.group("path"), reference_name)
        try:  # use module lookup to resolve
            resolved = lookup_in_module(requirement, localized_path)
        except Exception as e:
            _error(ET.EXTERNAL_LOOKUP_FAILED, error=e, path=localized_path, module=requirement)
            return
        if resolved is None:
            _error(ET.UNDEFINED_EXTERNAL_REFERENCE, path=localized_path, module=requirement.name)
            return

    statement.reference = resolved
    # check if the reference has the correct type
    if resolved.symbol_type != statement.symbol_type:
        _error(ET.REFERENCE_TYPE_MISMATCH, type=statement.symbol_type, resolved=resolved)


def resolve_type_references(
    statement: Statement,
    node: TypeNode,
    idx: ModuleIndex,
    on_error: Callable[[SemanticError], None],
) -> None:
    """Resolves (but does not impute) type references in a type node."""

    def _error(_t: ET, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, statement, cause, **error_args))

    # walk through child nodes
    if node.children is not None:
        for child in node.children:
            resolve_type_references(statement, child, idx, on_error)

    if not isinstance(node.reference, str):
        return  # nothing to resolve

    # normalize path to statement
    resolved_stmt = idx.scopes[statement.id].lookup_statement(node.reference)
    if resolved_stmt is None or resolved_stmt.underlying_definition is None:
        _error(ET.UNDEFINED_LOCAL_REFERENCE, path=node.reference)
        return
    if resolved_stmt.symbol_type != SymT.TYPE:
        _error(ET.REFERENCE_TYPE_MISMATCH, type=SymT.TYPE, resolved=resolved_stmt)
        return

    # get type node from statement
    # right now we get the underlying definition directly, ignoring intermediate arguments
    resolved_type = typing.cast(TypeContent, resolved_stmt.underlying_definition.content)
    node.reference = resolved_type.type_node

    impute_type_reference(node, keep_references=True)


def impute_type_reference(node: TypeNode, keep_references: bool = True) -> None:
    """
    Replace this nodes field in-place with the values of the referenced type node.
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
    if not keep_references:
        node.reference = None


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

    # create scopes for files and statements (but don't populate nested statements them yet)
    statements_by_parent_id: dict[UUID, list[Statement]] = defaultdict(list)
    for file in module.files:
        file_scope = Scope(
            id=file.id, name=file.path_without_extension, file=file, parent=None, statement=None
        )
        if file_scope.name in idx.scopes_by_name:
            _error(ET.AMBIGUOUS_DEFINITION, file, path=file_scope.name)
            continue
        idx.add_scope(file_scope)

        for statement in file.statements:
            if statement.name is None:
                continue  # ignore blanks and comments
            # create scope for every regular statement
            idx.statements_by_id[statement.id] = statement
            statements_by_parent_id[statement.parent_id or statement.file.id].append(statement)
            path = file_scope.name + ":" + statement.infile_path
            statement_scope = Scope(
                id=statement.id,
                name=path,
                file=file,
                parent=file_scope,
                statement=statement,
            )
            if statement_scope.name in idx.scopes_by_name:
                _error(ET.AMBIGUOUS_DEFINITION, statement, path=statement_scope.name)
                continue
            idx.add_scope(statement_scope)

    # set parent scope to statement parent (if it exists)
    for statement in idx.statements_by_id.values():
        if statement.parent_id is not None:
            statement_scope = idx.scopes[statement.id]
            parent_scope = idx.scopes[statement.parent_id]
            statement_scope.parent = parent_scope

    # populate scopes with expanded statements
    for statements in statements_by_parent_id.values():
        # (first sort all statements by index ascending inside their parent)
        statements.sort(key=lambda s: s.index)
        for statement in statements:
            scope = idx.scopes[statement.id]
            scope.parent.statements[statement.name] = statement

    # collect requirements
    for statement in idx.statements_by_id.values():
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
):
    """Interpret the module's statements as symbols."""
    if on_error == "raise":
        on_error = raise_error

    def _error(_t: ET, statement: Statement, cause: Exception | None = None, **error_args):
        on_error(SemanticError(_t, statement, cause, **error_args))
