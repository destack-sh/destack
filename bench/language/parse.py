from __future__ import annotations

import typing
from dataclasses import dataclass, field
from typing import Optional
from uuid import uuid4

from bench.language.lex import Token, TokenType
from bench.language.types import File, Statement, StatementType, SymbolContent, SymbolType


class ParseError(ValueError):
    def __init__(self, message: str, token: Token):
        super().__init__(message)
        self.token = token


@dataclass
class UnformedFile:
    path: str
    statements: list[UnformedStatement] = field(default_factory=list)


@dataclass
class UnformedStatement:
    name: Optional[str]
    type: StatementType
    symbol_type: Optional[SymbolType]
    name: typing.Optional[str]
    text: typing.Optional[str]
    value: typing.Optional[dict]
    symbol_type: SymbolType
    content: typing.Optional[SymbolContent]
    reference: Optional[str]


def parse(tokens: list[Token]) -> list[File]:
    """Parse a stream of tokens into Bench AST (with top-level files)."""
    if len(tokens) == 0:
        return []

    # first pass: extract files and statements
    files: dict[str, UnformedFile] = {}  # by path
    statements: dict[tuple[str, str], list[UnformedStatement]] = {}  # by (path, name)

    # first token has to be a new file
    current_file: UnformedFile = _match_file(tokens[0])
    current_pos: int = 1
    current_indent: int = 0
    current_statement: Optional[UnformedStatement] = None
    current_parent: Optional[UnformedStatement] = None

    if current_file is None:
        raise ParseError(f"first token must be a new file: {tokens[0]}")

    while current_pos < len(tokens):
        token = tokens[current_pos]

        # handle special tokens
        if token.type == TokenType.NEW_FILE:
            current_file = _match_file(token)
            current_indent = 0
            current_parent = None
            current_statement = None
        elif token.type == TokenType.INDENT:
            current_indent += 1
            current_pos += 1
            current_parent = current_statement
            current_statement = None
        elif token.type == TokenType.WHITESPACE:
            current_pos += 1
            continue
        else:  # match statements
            # prepare peek window
            # strip whitespace at the current indent level from the token stream
            # (we don't know how many tokens we'll need, so just prepare some constant)
            peek_window_size = 10
            peek_window = []
            peek_pos = current_pos
            while peek_pos < len(tokens) and len(peek_window) < peek_window_size:
                peek_token = tokens[peek_pos]
                if peek_token.type == TokenType.WHITESPACE:
                    peek_pos += 1
                    continue
                if peek_token.type == TokenType.INDENT:
                    peek_pos += 1
                    continue
                peek_window.append(peek_token)
                peek_pos += 1

            # match against patterns
            # comment
            # COMMENT
            # requirement
            # KEYWORD=StatementType.Requirement IDENTIFIER SEPERATOR=@ IDENTIFIER
            # import
            # KEYWORD=StatementType.Import IDENTIFIER (KEYWORD=as IDENTIFIER)? KEYWORD=from IDENTIFIER
            # definitions: data
            # [KEYWORD=instanceof StatementModifier]* KEYWORD=StatementType.Data IDENTIFIER SEPERATOR=: LITERAL

    # second pass: form statements into AST by resolving references


def _match_file(token: Token) -> UnformedFile:
    if token.type != TokenType.NEW_FILE:
        raise ParseError(f"expected new file token", token)
    return UnformedFile(path=token.value)
