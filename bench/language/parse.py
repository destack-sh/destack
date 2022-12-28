from __future__ import annotations

import enum
import json
import typing
from dataclasses import dataclass, field
from typing import Optional

import structlog

from bench.language.lex import Token, TokenType, get_location_range_pointer
from bench.language.types import Dataset, File, StatementType, SymbolContent, SymbolType

logger = structlog.get_logger(__name__)


class ParseError(ValueError):
    def __init__(self, message: str, token: Optional[Token], context: Optional[str] = None):
        super().__init__(
            message + ParseError.token_context(token) + (f"\n{context}" if context else "")
        )
        self.token = token

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


@dataclass
class ProtoFile:
    path: str
    statements: list[ProtoStatement] = field(default_factory=list)


@dataclass
class ProtoStatement:
    type: StatementType
    parent: Optional[ProtoStatement] = None
    index: Optional[int] = None
    name: Optional[str] = None
    symbol_type: Optional[SymbolType] = None
    text: typing.Optional[str] = None
    value: typing.Optional[dict] = None
    reference: str | tuple[str, str] | None = None
    content: typing.Optional[SymbolContent] = None
    compilation: Optional[ProtoCompilation] = None
    requirement: Optional[ProtoRequirement] = None
    runconfig: Optional[ProtoRunConfiguration] = None
    _source: Optional[list[Token]] = field(default_factory=list, repr=False)


@dataclass
class ProtoCompilation:
    pass


@dataclass
class ProtoRequirement:
    name: str
    version: str


@dataclass
class ProtoRunConfiguration:
    pass


@dataclass
class ProtoParse:
    files: dict[str, ProtoFile] = field(default_factory=dict)  # by path


def parse(tokens: list[Token]) -> list[File]:
    """Parse a stream of tokens into Bench AST (with top-level files)."""
    if len(tokens) == 0:
        return []

    # first pass: extract files and statements
    proto = _parse_proto(tokens)

    # second pass: resolve statements into AST by resolving references


class TokenEater:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically skips whitespace and matching indentation.
    """

    def __init__(self, tokens: list[Token], start_pos: int, indent_level: int):
        self._tokens = tokens
        self._start_pos = start_pos
        self._mark_pos = start_pos
        self._peek_pos = start_pos
        self._indent_level = indent_level
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
        if self._indent_level != 0:
            prev_pos = self._peek_pos
            for _ in range(self._indent_level):
                if _get(TokenType.INDENT) is None:
                    self._peek_pos = prev_pos  # rewind
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

    def peek_type(self, type: TokenType) -> Optional[Token]:
        token = self.peek()
        return token if token is not None and token.type == type else None

    def peek_keyword(self, keyword: str | enum.Enum) -> Optional[Token]:
        if isinstance(keyword, enum.Enum):
            keyword = keyword.value
        token = self.peek_type(TokenType.KEYWORD)
        if token is None or token.value != keyword:
            return None
        return token

    def eat_whitespace(self) -> Token:
        return self.eat_type(TokenType.WHITESPACE)

    def eat_indent(self) -> Token:
        return self.eat_type(TokenType.INDENT)

    def eat_type(self, type: TokenType) -> Token:
        token = self.eat()
        if token.type != type:
            raise ParseError(f"expected {type}", token)
        return token

    def eat_keyword(self, keyword: str | enum.Enum) -> Token:
        token = self.eat_type(TokenType.KEYWORD)
        if token.value != keyword:
            raise ParseError(f"expected {TokenType.KEYWORD} {keyword}", token)
        return token

    def eat_identifier(self) -> Token:
        return self.eat_type(TokenType.IDENTIFIER)

    def eat_separator(self, separator: str | enum.Enum) -> Token:
        if isinstance(separator, enum.Enum):
            separator = separator.value
        token = self.eat_type(TokenType.SEPARATOR)
        if token.value != separator:
            raise ParseError(f"expected {TokenType.SEPARATOR} {separator}", token)
        return token

    def eat_literal(self) -> Token:
        return self.eat_type(TokenType.LITERAL)


def _parse_proto(tokens: list[Token]) -> ProtoParse:
    proto = ProtoParse()

    eater = TokenEater(tokens, start_pos=0, indent_level=0)
    # first token has to be a new file
    file: ProtoFile = ProtoFile(path=eater.eat_type(TokenType.NEW_FILE).value)
    indent: int = 0
    ancestors: list[ProtoStatement] = []  # by indent

    while eater.peek() is not None:
        # reset indent if previous token was on a different line
        if eater.previous is not None and eater.previous.line_number != eater.peek().line_number:
            indent = 0

        if eater.peek().type == TokenType.NEW_FILE:
            path = eater.eat().value
            if path not in proto.files:
                proto.files[path] = ProtoFile(path=path)
            # reset per-file state
            file = proto.files[path]
            indent = 0
            ancestors = []
        elif eater.peek().type == TokenType.WHITESPACE:
            eater.eat()
        elif eater.peek().type == TokenType.INDENT:
            eater.eat()
            indent += 1
        else:  # parse statement
            # error if there is no ancestor one level up
            if len(ancestors) < indent:
                raise ParseError("unexpected indent level", eater.peek())

            local_errors = []
            eater.mark()
            statement = _parse_statement(eater, on_error=lambda p, e: local_errors.append((p, e)))
            if statement is None:
                context = "\n".join(f"{parser}:\n{error}" for parser, error in local_errors)
                raise ParseError(f"unexpected statement", eater.peek(), f"local errors:\n{context}")
            statement._source = eater.eaten
            file.statements.append(statement)

            ancestors = ancestors[:indent]  # wipe ancestors with higher indent
            ancestors += [statement]  # replace ancestor at current indent

    return proto


def _parse_comment(tokens: TokenEater) -> ProtoStatement:
    token = tokens.eat_type(TokenType.COMMENT)
    return ProtoStatement(type=StatementType.COMMENT, text=token.value)


def _parse_requirement(tokens: TokenEater) -> ProtoStatement:
    tokens.eat_keyword(StatementType.REQUIREMENT)
    dependency = tokens.eat_identifier()
    if tokens.peek_keyword("as"):
        tokens.eat_keyword("as")
        alias = tokens.eat_identifier()
    else:
        alias = dependency.value
    tokens.eat_separator("@")
    version = tokens.eat_identifier()
    return ProtoStatement(
        type=StatementType.REQUIREMENT,
        name=alias,
        requirement=ProtoRequirement(name=dependency.value, version=version.value),
    )


def _parse_import(tokens: TokenEater) -> ProtoStatement:
    tokens.eat_keyword(StatementType.IMPORT)
    reference = tokens.eat_identifier().value
    if tokens.peek_keyword("as"):
        tokens.eat_keyword("as")
        alias = tokens.eat_identifier()
    else:
        alias = reference.value
    tokens.eat_keyword("from")
    source = tokens.eat_identifier().value
    return ProtoStatement(type=StatementType.IMPORT, name=alias, reference=(reference, source))


def _parse_definition_dataset(tokens: TokenEater) -> ProtoStatement:
    tokens.eat_keyword("data")
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    value = tokens.eat_literal()
    # parse as jsonl
    records = []
    for value_line in value.value.strip().splitlines():
        try:
            records.append(json.loads(value_line.strip()))
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse jsonl: {e}", value)
    dataset = Dataset(records=records)
    return ProtoStatement(
        type=StatementType.DEFINITION,
        name=name.value,
        symbol_type=SymbolType.DATASET,
        content=dataset,
    )


# in parse order
STATEMENT_PARSERS = [_parse_comment, _parse_requirement, _parse_definition_dataset, _parse_import]


def _parse_statement(
    eater: TokenEater, on_error: typing.Callable[[str, ParseError], None]
) -> Optional[ProtoStatement]:
    for parser in STATEMENT_PARSERS:
        try:
            return parser(eater)
        except ParseError as e:
            on_error(parser.__name__, e)
            eater.reset()
    return None
