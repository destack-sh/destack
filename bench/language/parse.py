from __future__ import annotations

import dataclasses
import enum
import json
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench.language.lex import Token, TokenType, get_location_range_pointer
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
    Value,
)
from bench.utils.record import RecordList

logger = structlog.get_logger(__name__)


class ParseError(ValueError):
    def __init__(
        self,
        message: str,
        token: Optional[Token],
        cause: Optional[Exception] = None,
        context: Optional[dict] = None,
    ):
        super().__init__(self._to_message(message, token, cause))
        self.token = token
        self.cause = cause
        self.context = context

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


def parse(tokens: list[Token], strip_whitespace: bool) -> list[File]:
    """Parse a stream of tokens into Bench AST (with top-level files)."""
    if len(tokens) == 0:
        return []

    if strip_whitespace:
        tokens = [t for t in tokens if t.type != TokenType.WHITESPACE]

    # first pass: extract files and statements
    files = preparse(tokens)
    # second pass: resolve statements into AST by resolving references
    resolve(files)

    return files


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically skips whitespace and matching indentation.
    """

    def __init__(self, tokens: list[Token], start_pos: int, indent_level: int):
        self._tokens = tokens
        self._start_pos = start_pos
        self._mark_pos = start_pos
        self._peek_pos = start_pos
        self.indent_level = indent_level
        self.reset()

    def mark(self):
        self._mark_pos = self.current_pos

    def reset(self):
        self._peek_pos = self._mark_pos
        self._advance()

    @property
    def eaten(self) -> list[Token]:
        return self._tokens[self._mark_pos : max(0, self.current_pos)]

    @property
    def current_pos(self) -> int:
        return self._peek_pos - 1

    @property
    def previous(self) -> Optional[Token]:
        if self.current_pos <= self._start_pos:
            return None
        return self._tokens[self.current_pos - 1]

    def _advance(self):
        """Advances the peek position, skipping whitespace and matching indentation."""

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

        # skip whitespace
        while _get(TokenType.WHITESPACE) is not None:
            self._peek_pos += 1

        # skip indentation
        if self.indent_level != 0:
            prev_pos = self._peek_pos
            for _ in range(self.indent_level):
                if _get(TokenType.INDENT) is None:
                    self._peek_pos = prev_pos  # rewind
                    break
                self._peek_pos += 1

    def peek(self) -> Optional[Token]:
        """Peeks the next token without eating it."""
        if self.current_pos >= len(self._tokens):
            return None
        return self._tokens[self.current_pos]

    def eat(self) -> Token:
        """Eats the next token."""
        if self.peek() is None:
            raise ParseError("expected token", None)
        peek_token = self.peek()
        self._advance()
        return peek_token

    def eat_whitespace(self) -> Token:
        return self.eat_type(TokenType.WHITESPACE)

    def eat_indent(self) -> Token:
        return self.eat_type(TokenType.INDENT)

    def peek_type(self, type: TokenType) -> Optional[Token]:
        token = self.peek()
        return token if token is not None and token.type == type else None

    def eat_type(self, type: TokenType) -> Token:
        token = self.eat()
        if token.type != type:
            raise ParseError(f"expected {type.value}", token)
        return token

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


def preparse(tokens: list[Token]) -> list[File]:
    """Map tokens into files and statements with unresolved references."""

    files: dict[str, File] = {}
    parser = TokenParser(tokens, start_pos=0, indent_level=0)
    # first token has to be a new file
    root = DerivedRoot()
    file: File = File(path=parser.eat_type(TokenType.NEW_FILE).value)
    indent: int = 0
    ancestors: list[Statement] = []  # by indent

    while parser.peek() is not None:
        # reset indent if previous token was on a different line
        if parser.previous is not None and parser.previous.line_number != parser.peek().line_number:
            indent = 0

        if parser.peek().type == TokenType.NEW_FILE:
            path = parser.eat().value
            if path not in files:
                files[path] = File(path=path)
            # reset per-file state
            file = files[path]
            indent = 0
            ancestors = []
        elif parser.peek().type == TokenType.WHITESPACE:
            parser.eat()
        elif parser.peek().type == TokenType.INDENT:
            parser.eat()
            indent += 1
        else:  # parse statement
            # error if there is no ancestor one level up
            if len(ancestors) < indent:
                raise ParseError("unexpected indent level", parser.peek())

            parent = ancestors[indent - 1] if indent > 0 else None
            index = len(file.root_statements) if parent is None else len(parent.children)

            local_errors = []  # (parser name, pos, error)
            parser.indent_level = indent
            statement = _parse_statement(
                parser,
                on_error=lambda p, c, e: local_errors.append((p, c, e)),
                file=file,
                parent=parent,
                index=index,
            )
            parser.indent_level = 0  # skip only for statement parsing
            if statement is None:
                # sort by parsed pos descending (get the furthest error)
                local_errors.sort(key=lambda e: e[1], reverse=True)
                likely_error = (
                    local_errors[0][2] if local_errors[0][1] > local_errors[-1][1] else None
                )
                raise ParseError(
                    "unexpected statement",
                    parser.peek(),
                    cause=likely_error,
                    context={"local_errors": local_errors},
                )
            statement = dataclasses.replace(statement, _root=root, _source=parser.eaten)
            file.statements.append(statement)

            ancestors = ancestors[:indent]  # wipe ancestors with higher indent
            ancestors += [statement]  # replace ancestor at current indent

    return list(files.values())


def _parse_comment(tokens: TokenParser, **kwargs) -> Statement:
    token = tokens.eat_type(TokenType.COMMENT)
    return Statement(type=StatementType.COMMENT, text=token.value, **kwargs)


def _parse_requirement(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a requirement statement."""
    tokens.eat_keyword(StatementType.REQUIREMENT)
    dependency = tokens.eat_identifier()
    if tokens.peek_keyword("as"):
        tokens.eat_keyword("as")
        alias = tokens.eat_identifier()
    else:
        alias = dependency.value
    tokens.eat_separator("@")
    version = tokens.eat_identifier()
    return Statement(
        type=StatementType.REQUIREMENT,
        name=alias,
        requirement=Requirement(name=dependency.value, version=version.value),
        **kwargs,
    )


def _parse_import(tokens: TokenParser, **kwargs) -> Statement:
    """Parse an import statement."""
    tokens.eat_keyword(StatementType.IMPORT)
    symbol_type = tokens.eat_keyword_like(SymbolType)
    reference = tokens.eat_identifier()
    if tokens.peek_keyword("as"):
        tokens.eat_keyword("as")
        alias = tokens.eat_identifier()
    else:
        alias = reference.value
    tokens.eat_keyword("from")
    source = tokens.eat_identifier().value
    return Statement(
        type=StatementType.IMPORT,
        name=alias,
        symbol_type=symbol_type.value,
        reference=(reference.value, source),
        **kwargs,
    )


def _parse_definition(tokens: TokenParser, **kwargs) -> Statement:
    """Parses a symbol definition statement."""
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError("expected statement modifier", modifier)
    else:
        modifier = None
    symbol_type = tokens.eat_keyword_like(SymbolType)
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    literal = tokens.eat_literal()

    definition = Statement(
        type=StatementType.DEFINITION,
        modifier=modifier,
        name=name.value,
        symbol_type=symbol_type.value,
        **kwargs,
    )

    if symbol_type.value == SymbolType.SCHEMA:
        try:
            element = json.loads(literal.value)
        except json.JSONDecodeError as e:
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
        code_text = _clean_literal_indent(literal.value, tokens.indent_level)
        content = Code(
            type=SymbolType.CODE,
            code_function_name=None,
            builtin_id=None,
            code_text=code_text,
            definition=definition,
        )
    elif symbol_type.value == SymbolType.DATASET:
        try:  # parse as jsonl
            records = []
            for value_line in literal.value.strip().splitlines():
                records.append(json.loads(value_line.strip()))
            content = Dataset(
                type=SymbolType.DATASET, records=RecordList(records), definition=definition
            )
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse jsonl: {e}", literal)
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
    else:
        modifier = None
    symbol_type = tokens.eat_keyword_like(SymbolType)
    name = tokens.eat_identifier()
    return Statement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=name.value,
        symbol_type=symbol_type.value,
        **kwargs,
    )


def _parse_compile(tokens: TokenParser, **kwargs) -> Statement:
    tokens.eat_keyword(StatementType.COMPILATION)
    name = tokens.eat_identifier()
    tokens.eat_separator("=")
    symbol_type = tokens.eat_keyword_like(SymbolType)
    reference = tokens.eat_identifier()
    tokens.eat_separator(":")
    return Statement(
        type=StatementType.COMPILATION,
        name=name.value,
        symbol_type=symbol_type.value,
        reference=reference.value,
        **kwargs,
    )


def _parse_statement(
    parser: TokenParser, on_error: typing.Callable[[str, int, ParseError], None], **statement_kwargs
) -> Optional[Statement]:
    parser.mark()
    # parsing is greedy, so the order matters (e.g. definition before reference)
    for _parse in [
        _parse_comment,
        _parse_requirement,
        _parse_compile,
        _parse_definition,
        _parse_reference,
        _parse_import,
    ]:
        try:
            return _parse(parser, **statement_kwargs)
        except ParseError as e:
            on_error(_parse.__name__, parser.current_pos, e)
            parser.reset()
    return None


def resolve(files: list[File]):
    """Resolve references"""
    files: dict[UUID, File] = {}
    statements: dict[UUID, Statement] = {}

    return files
