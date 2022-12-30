from __future__ import annotations

import dataclasses
import enum
import json
import typing
from collections import OrderedDict, defaultdict
from typing import Callable, Optional
from uuid import UUID

import structlog

from bench.language.lex import Token, TokenType, get_location_range_pointer
from bench.language.schema import parse_bsl
from bench.language.types import (
    Code,
    Dataset,
    Expectation,
    File,
    Requirement,
    Schema,
    Statement,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    Task,
    Value,
    statement_path_as_str,
)
from bench.utils.record import RecordList

logger = structlog.get_logger(__name__)

UNGROUPED_STATEMENT_TYPES = (
    StatementType.DEFINITION,
    StatementType.REDEFINITION,
    StatementType.COMPILATION,
    StatementType.RUNCONFIG,
)


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


class ParseError(ValueError):
    def __init__(
        self,
        message: str,
        token: Optional[Token],
        cause: Optional[Exception] = None,
        context: Optional[dict] = None,
        parser: Optional[str] = None,
        position: Optional[int] = -1,
    ):
        super().__init__(self._to_message(message, token, cause))
        self.token = token
        self.cause = cause
        self.context = context
        self.position = position
        self.parser = parser

    def _to_message(self, message: str, token: Optional[Token], cause: Optional[dict]):
        return (
            message
            + ParseError.token_context(token)
            + (f"\npossible cause: {cause}" if cause else "")
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


def parse(
    tokens: list[Token],
    on_error: typing.Literal["raise"] | Callable[[ParseError | SemanticError], None] = "raise",
) -> list[File]:
    """
    Parse a stream of tokens into Bench AST (grouped into files).
    """
    if len(tokens) == 0:
        return []
    if on_error == "raise":
        on_error = raise_error

    # first pass: extract files and statements
    files = preparse(tokens, on_error=on_error)
    # second pass: resolve statements into AST by resolving references
    resolve(files, on_error=on_error)
    return files


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically matching indentation at the configured level.
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

    def _advance(self):
        """Advances the peek position, skipping matching indentation."""

        # (note that we can't use eat/peek here because they use _advance)
        def _get(type: Optional[TokenType] = None) -> Optional[Token]:
            if self.current_pos >= len(self._tokens):
                return None
            token = self._tokens[self.current_pos]
            if token.type != type:
                return None
            return token

        # advance to next token (current pos is based on peek pos)
        self._peek_pos += 1

        # get next indent level
        self._peek_indent_level = 0
        for token in self._tokens[self.current_pos :]:
            if token.type != TokenType.INDENT:
                break
            self._peek_indent_level += 1

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
            raise ParseError("expected token after", self.previous)
        peek_token = self.peek()
        self._advance()
        return peek_token

    def peek_type(self, type: TokenType) -> Optional[Token]:
        token = self.peek()
        return token if token is not None and token.type == type else None

    def eat_type(self, type: TokenType) -> Token:
        token = self.eat()
        if token.type != type:
            raise ParseError(f"expected {type.value}", token)
        return token

    def eat_newfile(self) -> Token:
        return self.eat_type(TokenType.NEWFILE)

    def eat_newline(self) -> Token:
        return self.eat_type(TokenType.NEWLINE)

    def eat_newline_or_eof(self) -> Optional[Token]:
        if self.peek() is None or self.peek_type(TokenType.NEWFILE) is not None:
            return
        return self.eat_newline()

    def peek_indent(self) -> Optional[Token]:
        return self.peek_type(TokenType.INDENT)

    def eat_indent(self) -> Token:
        return self.eat_type(TokenType.INDENT)

    def peek_keyword(self, keyword: str | enum.Enum) -> Optional[Token]:
        token = self.peek_type(TokenType.KEYWORD)
        if token is None or token.value != keyword:
            return None
        return token

    def eat_keyword(self, keyword: str | enum.Enum) -> Token:
        token = self.eat_type(TokenType.KEYWORD)
        if token.value != keyword:
            raise ParseError(f"expected {TokenType.KEYWORD.value} {keyword}", token)
        return token

    def peek_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Optional[Token]:
        token = self.peek_type(TokenType.KEYWORD)
        if token is None or not isinstance(token.value, keyword_cls):
            return None
        return token

    def eat_keyword_like(self, keyword_cls: typing.Type[enum.Enum]) -> Token:
        token = self.eat_type(TokenType.KEYWORD)
        if not isinstance(token.value, keyword_cls):
            raise ParseError(f"expected {TokenType.KEYWORD.value} {keyword_cls}", token)
        return token

    def eat_identifier(self) -> Token:
        return self.eat_type(TokenType.IDENTIFIER)

    def eat_separator(self, separator: str | enum.Enum) -> Token:
        if isinstance(separator, enum.Enum):
            separator = separator.value
        token = self.eat_type(TokenType.SEPARATOR)
        if token.value != separator:
            raise ParseError(f"expected {TokenType.SEPARATOR.value} {separator}", token)
        return token

    def eat_space(self) -> Token:
        return self.eat_separator(" ")

    def eat_literal(self) -> Token:
        return self.eat_type(TokenType.LITERAL)


def _clean_literal_indent(text: str, indent_level: int) -> str:
    """Removes indentation up to the given level (4 spaces or 1 tab)."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if line.startswith(" " * 4 * indent_level):
            lines[i] = line[indent_level * 4 :]
        elif line.startswith("\t" * indent_level):
            lines[i] = line[indent_level:]
    return "\n".join(lines)


@dataclasses.dataclass
class FileParseState:
    file: File
    indent: int = 0
    ancestors: list[Statement] = dataclasses.field(default_factory=list)  # by indent
    statements_by_parent: dict[UUID | None, list[Statement]] = dataclasses.field(
        default_factory=lambda: defaultdict(list)
    )
    previous_errors: list[ParseError | None] = dataclasses.field(default_factory=list)

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


def preparse(tokens: list[Token], on_error: Callable[[ParseError], None]) -> list[File]:
    """Map tokens into files and statements with unresolved references."""

    parser = TokenParser(tokens, start_pos=0, indent_level=0)
    states: dict[str, FileParseState] = OrderedDict()  # remember original file order
    initial_file: File = File(path=parser.eat_newfile().value)  # tokens[0] must be newfile
    local = FileParseState(file=initial_file)
    states[initial_file.path] = local

    while parser.peek() is not None:
        # reset indent if previous token was on a different line
        if parser.previous is not None and parser.previous.line_number != parser.peek().line_number:
            local.indent = 0

        if parser.peek().type == TokenType.NEWFILE:
            path = parser.eat_newfile().value
            if path not in states:
                states[path] = FileParseState(file=File(path=path))  # begin new file state
            local = states[path]
        elif parser.peek().type == TokenType.INDENT:
            parser.eat()
            local.indent += 1
        else:  # parse statement
            if len(local.ancestors) < local.indent:  # too much indentation
                raise ParseError("unexpected indent level", parser.peek())

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
                error = ParseError(
                    "unexpected statement", parser.peek(), cause=cause, context={"errors": errors}
                )
                on_error(error)
                continue

            statement._source = parser.eaten(start_mark)
            local.add_statement(statement)

    return [state.file for state in states.values()]


def _parse_comment(tokens: TokenParser, **kwargs) -> Statement:
    token = tokens.eat_type(TokenType.COMMENT)
    tokens.eat_newline_or_eof()
    return Statement(type=StatementType.COMMENT, text=token.value, **kwargs)


def _parse_requirement(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a requirement statement."""
    tokens.eat_keyword(StatementType.REQUIREMENT)
    tokens.eat_space()
    dependency = tokens.eat_identifier()
    tokens.eat_separator("@")
    version = tokens.eat_identifier()
    tokens.eat_newline_or_eof()
    return Statement(
        type=StatementType.REQUIREMENT,
        name=dependency.value,
        requirement=Requirement(name=dependency.value, version=version.value),
        **kwargs,
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
    source = tokens.eat_identifier().value
    tokens.eat_newline_or_eof()
    return Statement(
        type=StatementType.IMPORT,
        name=alias,
        symbol_type=symbol_type.value,
        reference=StatementPath(source, reference),
        **kwargs,
    )


def _parse_definition(tokens: TokenParser, **kwargs) -> Statement:
    """Parses a symbol definition statement."""
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError("expected statement modifier", modifier)
        tokens.eat_space()
    else:
        modifier = None
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline()
    literal = tokens.eat_literal()
    tokens.eat_newline_or_eof()

    definition = Statement(
        type=StatementType.DEFINITION,
        modifier=modifier,
        name=name.value,
        symbol_type=symbol_type.value,
        **kwargs,
    )

    if symbol_type.value == SymbolType.SCHEMA:
        try:
            element = parse_bsl(literal.value)
        except ValueError as e:
            raise ParseError("failed to parse schema element", literal) from e
        content = Schema(
            type=SymbolType.SCHEMA, description="", element=element, definition=definition
        )
    elif symbol_type.value == SymbolType.TASK:
        content = Task(type=SymbolType.TASK, description=literal.value, definition=definition)
    elif symbol_type.value == SymbolType.EXPECTATION:
        content = Expectation(
            type=SymbolType.EXPECTATION, description=literal.value, definition=definition
        )
    elif symbol_type.value == SymbolType.CODE:
        lang = literal.value_extras.get("lang")
        if lang is None:
            raise ParseError("expected language", literal)
        if lang != "python":
            raise ParseError("unsupported code language", literal)
        code_text = _clean_literal_indent(literal.value, tokens.indent_level)
        content = Code(
            language=lang,
            type=SymbolType.CODE,
            code_function_name=None,
            builtin_id=None,
            code=code_text,
            definition=definition,
        )
    elif symbol_type.value == SymbolType.DATASET:
        lang = literal.value_extras.get("lang")
        try:
            if lang is None:
                raise ParseError("expected language", literal)
            elif lang == "jsonl":
                records = []
                for value_line in literal.value.strip().splitlines():
                    records.append(json.loads(value_line.strip()))
            elif lang == "json":
                records = json.loads(literal.value)
            else:
                raise ParseError("unsupported dataset language", literal)
            content = Dataset(
                type=SymbolType.DATASET, records=RecordList(records), definition=definition
            )
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse records: {e}", literal)
    elif symbol_type.value == SymbolType.VALUE:
        try:  # parse as json
            value = json.loads(literal.value)
            content = Value(type=SymbolType.VALUE, value=value, definition=definition)
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse json: {e}", literal)
    else:
        raise ParseError(f"unexpected symbol type {symbol_type.value}", symbol_type)
    definition.content = content
    return definition


def _parse_reference(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a reference statement."""
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError("expected statement modifier", modifier)
        tokens.eat_space()
    else:
        modifier = None
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_newline_or_eof()
    return Statement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=name.value,
        reference=StatementPath(".", name.value),
        symbol_type=symbol_type.value,
        **kwargs,
    )


def _parse_compile(tokens: TokenParser, **kwargs) -> Statement:
    tokens.eat_keyword(StatementType.COMPILATION)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_space()
    tokens.eat_separator("=")
    tokens.eat_space()
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    reference = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline_or_eof()
    return Statement(
        type=StatementType.COMPILATION,
        name=name.value,
        symbol_type=symbol_type.value,
        reference=StatementPath(".", reference.value),
        **kwargs,
    )


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
        _parse_requirement,
        _parse_compile,
        _parse_definition,
        _parse_import,
        _parse_reference,
        _parse_blank,
    ]:
        mark = parser.mark()
        try:
            statement = _parse(parser, **statement_kwargs)
            # check if there's a blank line after the end of a group
            is_ungrouped = statement.type in UNGROUPED_STATEMENT_TYPES
            could_be_group_end = (is_root and is_ungrouped) or not is_root
            # we're at the end if there is an unintended token next
            if could_be_group_end and parser.peek_indent_level == 0:
                parser.eat_newline_or_eof()
            return statement
        except ParseError as e:
            e.parser = _parse.__name__
            e.position = parser.current_pos
            on_error(e)
            parser.reset(mark)
    return None


class SemanticError(ValueError):
    def __init__(
        self,
        message: str,
        statement: Optional[Statement],
        related_statements: dict[str, Statement] | None = None,
    ):
        super().__init__(self._format_message(message, statement, related_statements))
        self.statement = statement
        self.related_statements = related_statements

    def _format_message(
        self, message: str, statement: Statement, related_statements: dict[str, Statement] | None
    ) -> str:
        related_statements = related_statements or {}
        related_context = "".join(
            f"\nrelated {k}: {SemanticError.statement_context(v)}"
            for k, v in related_statements.items()
        )
        return message + SemanticError.statement_context(statement) + related_context

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
            return f" at\n{statement}\n{source[0].source_file.path} {source[0].line_number}\n{source_context}"


def resolve(files: list[File], on_error: Callable[[SemanticError], None]):
    """Resolve references across files."""

    statements_by_path: dict[StatementPath, Statement] = {}
    statements_by_parent: dict[UUID, list[Statement]] = defaultdict(list)

    def _error(
        message: str, statement: Statement, related_statements: dict[str, Statement] | None = None
    ):
        on_error(SemanticError(message, statement, related_statements))

    # collect files and statements
    for file in files:
        for statement in file.statements:
            if statement.parent is not None:
                statements_by_parent[statement.parent.id].append(statement)
            if statement.referable:
                statement_path = StatementPath(f".{file.path_without_extension}", statement.name)
                if statement_path in statements_by_path:
                    error = SemanticError(f"multiple definitions for {statement_path}", statement)
                    on_error(error)
                    continue
                statements_by_path[statement_path] = statement

    # resolve references
    for file in files:
        for statement in file.statements:
            if isinstance(statement.reference, StatementPath):
                if statement.reference.path[0] == ".":  # current file
                    normalized_path = StatementPath(
                        f".{file.path_without_extension}", statement.name
                    )
                else:
                    normalized_path = statement.reference
                # resolve reference
                if normalized_path in statements_by_path:
                    statement.reference = statements_by_path[normalized_path]
                else:
                    _error(
                        f"reference is undefined: {statement_path_as_str(statement.reference)}",
                        statement,
                    )
                # check if the reference has the correct type
                if statement.reference.symbol_type != statement.symbol_type:
                    _error(
                        f"reference has other symbol type: {statement_path_as_str(statement.reference)}",
                        statement,
                    )
