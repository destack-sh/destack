from __future__ import annotations

import enum
import json
import typing
from dataclasses import dataclass, field
from typing import Optional

import structlog

from bench.language.lex import Token, TokenType, get_location_range_pointer
from bench.language.types import (
    Code,
    Dataset,
    Expectation,
    File,
    Schema,
    StatementModifier,
    StatementType,
    SymbolContent,
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


@dataclass
class ProtoFile:
    path: str
    statements: list[ProtoStatement] = field(default_factory=list, repr=False)

    def __str__(self):
        return self.path


@dataclass
class ProtoStatement:
    type: StatementType
    modifier: Optional[StatementModifier] = None
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


def parse(tokens: list[Token], strip_whitespace: bool) -> list[File]:
    """Parse a stream of tokens into Bench AST (with top-level files)."""
    if len(tokens) == 0:
        return []

    if strip_whitespace:
        tokens = [t for t in tokens if t.type != TokenType.WHITESPACE]

    # first pass: extract files and statements
    proto = preparse(tokens)

    # second pass: resolve statements into AST by resolving references
    files = resolve(proto)
    return files


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


def preparse(tokens: list[Token]) -> ProtoParse:
    """Map tokens into proto files and statements with unresolved references."""
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

            local_errors = []  # (parser name, pos, error)
            eater.indent_level = indent
            statement = _parse_statement(
                eater, on_error=lambda p, c, e: local_errors.append((p, c, e))
            )
            eater.indent_level = 0  # skip only for statement parsing
            if statement is None:
                # sort by parsed pos descending (get the furthest error)
                local_errors.sort(key=lambda e: e[1], reverse=True)
                likely_error = (
                    local_errors[0][2] if local_errors[0][1] > local_errors[-1][1] else None
                )
                raise ParseError(
                    "unexpected statement",
                    eater.peek(),
                    cause=likely_error,
                    context={"local_errors": local_errors},
                )
            statement._source = eater.eaten
            file.statements.append(statement)

            ancestors = ancestors[:indent]  # wipe ancestors with higher indent
            ancestors += [statement]  # replace ancestor at current indent

    return proto


def _parse_comment(tokens: TokenEater) -> ProtoStatement:
    token = tokens.eat_type(TokenType.COMMENT)
    return ProtoStatement(type=StatementType.COMMENT, text=token.value)


def _parse_requirement(tokens: TokenEater) -> ProtoStatement:
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
    return ProtoStatement(
        type=StatementType.REQUIREMENT,
        name=alias,
        requirement=ProtoRequirement(name=dependency.value, version=version.value),
    )


def _parse_import(tokens: TokenEater) -> ProtoStatement:
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
    return ProtoStatement(
        type=StatementType.IMPORT,
        name=alias,
        symbol_type=symbol_type.value,
        reference=(reference.value, source),
    )


def _parse_definition(tokens: TokenEater) -> ProtoStatement:
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
    if symbol_type.value == SymbolType.SCHEMA:
        try:
            element = json.loads(literal.value)
        except json.JSONDecodeError as e:
            raise ParseError("failed to parse schema element", literal) from e
        content = Schema(type=SymbolType.SCHEMA, description="", element=element)
    elif symbol_type.value == SymbolType.TASK:
        content = Task(type=SymbolType.TASK, description=literal.value)
    elif symbol_type.value == SymbolType.EXPECTATION:
        content = Expectation(type=SymbolType.EXPECTATION, description=literal.value)
    elif symbol_type.value == SymbolType.CODE:
        code_text = _clean_literal_indent(literal.value, tokens.indent_level)
        content = Code(
            type=SymbolType.CODE, code_function_name=None, builtin_id=None, code_text=code_text
        )
    elif symbol_type.value == SymbolType.DATASET:
        try:  # parse as jsonl
            records = []
            for value_line in literal.value.strip().splitlines():
                records.append(json.loads(value_line.strip()))
            content = Dataset(type=SymbolType.DATASET, records=RecordList(records))
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse jsonl: {e}", literal)
    elif symbol_type.value == SymbolType.VALUE:
        try:  # parse as json
            value = json.loads(literal.value)
            content = Value(type=SymbolType.VALUE, value=value)
        except json.JSONDecodeError as e:
            raise ParseError(f"failed to parse json: {e}", literal)
    else:
        raise ParseError(f"unexpected symbol type {symbol_type.value}", symbol_type)

    return ProtoStatement(
        type=StatementType.DEFINITION,
        modifier=modifier,
        name=name.value,
        symbol_type=SymbolType.DATASET,
        content=content,
    )


def _parse_reference(tokens: TokenEater) -> ProtoStatement:
    """Parse a reference statement."""
    if tokens.peek_keyword_like(StatementModifier):
        modifier = tokens.eat_keyword_like(StatementModifier).value
        if not isinstance(modifier, StatementModifier):
            raise ParseError("expected statement modifier", modifier)
    else:
        modifier = None
    symbol_type = tokens.eat_keyword_like(SymbolType)
    name = tokens.eat_identifier()
    return ProtoStatement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=name.value,
        symbol_type=symbol_type.value,
    )


def _parse_compile(tokens: TokenEater) -> ProtoStatement:
    tokens.eat_keyword(StatementType.COMPILATION)
    name = tokens.eat_identifier()
    tokens.eat_separator("=")
    symbol_type = tokens.eat_keyword_like(SymbolType)
    reference = tokens.eat_identifier()
    tokens.eat_separator(":")
    return ProtoStatement(
        type=StatementType.COMPILATION,
        name=name.value,
        symbol_type=symbol_type.value,
        reference=reference.value,
    )


def _parse_statement(
    eater: TokenEater, on_error: typing.Callable[[str, int, ParseError], None]
) -> Optional[ProtoStatement]:
    eater.mark()
    # parsing is greedy, so the order matters (e.g. definition before reference)
    for parser in [
        _parse_comment,
        _parse_requirement,
        _parse_compile,
        _parse_definition,
        _parse_reference,
        _parse_import,
    ]:
        try:
            return parser(eater)
        except ParseError as e:
            on_error(parser.__name__, eater.current_pos, e)
            eater.reset()
    return None


def resolve(proto: ProtoParse) -> list[File]:
    """Resolve references and map to language objects."""
    return []
