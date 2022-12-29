from __future__ import annotations

import dataclasses
import enum
import json
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench.language.lex import Token, TokenType, get_location_range_pointer
from bench.language.schema import parse_bql
from bench.language.types import (
    Code,
    Dataset,
    DerivedRoot,
    Expectation,
    File,
    Requirement,
    Schema,
    Statement,
    StatementModifier,
    StatementType,
    SymbolType,
    Task,
    UnresolvedStatement,
    Value,
)
from bench.utils.record import RecordList

logger = structlog.get_logger(__name__)

UNGROUPED_STATEMENT_TYPES = (
    StatementType.DEFINITION,
    StatementType.REDEFINITION,
    StatementType.COMPILATION,
    StatementType.RUNCONFIG,
)


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


def parse(tokens: list[Token]) -> list[File]:
    """
    Parse a stream of tokens into Bench AST (with top-level files).
    """
    if len(tokens) == 0:
        return []

    # first pass: extract files and statements
    files = preparse(tokens)
    # second pass: resolve statements into AST by resolving references
    resolve(files)

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
class ParseState:
    indent: int = 0
    ancestors: list[Statement] = dataclasses.field(default_factory=list)  # by indent
    previous_errors: list[ParseError | None] = dataclasses.field(default_factory=list)


def preparse(tokens: list[Token]) -> list[File]:
    """Map tokens into files and statements with unresolved references."""

    files: dict[str, File] = {}
    parser = TokenParser(tokens, start_pos=0, indent_level=0)
    root = DerivedRoot()

    # init per-file state (first token has to be a new file)
    file: File = File(path=parser.eat_type(TokenType.NEWFILE).value)
    files[file.path] = file
    local = ParseState()

    while parser.peek() is not None:
        # reset indent if previous token was on a different line
        if parser.previous is not None and parser.previous.line_number != parser.peek().line_number:
            local.indent = 0

        if parser.peek().type == TokenType.NEWFILE:
            path = parser.eat().value
            if path not in files:
                files[path] = File(path=path)
            # reset per-file state
            file = files[path]
            local = ParseState()
        elif parser.peek().type == TokenType.INDENT:
            parser.eat()
            local.indent += 1
        else:  # parse statement
            if len(local.ancestors) < local.indent:  # too much indentation
                raise ParseError("unexpected indent level", parser.peek())
            parent = local.ancestors[local.indent - 1] if local.indent > 0 else None
            index = len(file.root_statements) if parent is None else len(parent.children)

            errors: list[ParseError] = []
            parser.indent_level = local.indent  # skip indent tokens at current level
            start_mark = parser.mark()
            statement = _parse_statement(
                parser,
                is_root=parent is None,
                on_error=errors.append,
                file=file,
                parent=parent,
                index=index,
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
                raise ParseError(
                    "unexpected statement", parser.peek(), cause=cause, context={"errors": errors}
                )
            statement = dataclasses.replace(statement, _root=root, _source=parser.eaten(start_mark))
            file.statements.append(statement)

            local.ancestors = local.ancestors[: local.indent]  # wipe ancestors with higher indent
            local.ancestors += [statement]  # replace ancestor at current indent

    return list(files.values())


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
        reference=UnresolvedStatement(source, reference),
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
            element = parse_bql(literal.value)
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
    definition = dataclasses.replace(definition, content=content)
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
        reference=UnresolvedStatement(".", name.value),
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
        reference=UnresolvedStatement(".", reference.value),
        **kwargs,
    )


def _parse_blank(tokens: TokenParser, **kwargs) -> Statement:
    tokens.eat_newline()
    return Statement(type=StatementType.BLANK, **kwargs)


def _parse_statement(
    parser: TokenParser,
    is_root: bool,
    on_error: typing.Callable[[ParseError], None],
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


def resolve(files: list[File]):
    """Resolve references across files."""
    files: dict[UUID, File] = {}
    statements: dict[UUID, Statement] = {}

    # TODO @Broken: resolve references within and across files

    return files
