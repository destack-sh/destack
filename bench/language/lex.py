from __future__ import annotations

import enum
import re
from dataclasses import dataclass
from functools import cached_property
from typing import Optional

from bench.language.types import StatementModifier, StatementType, SymbolType


@dataclass(repr=False)
class SourceFile:
    path: Optional[str]
    content: str

    def __str__(self):
        return f"{self.path} ({len(self.linebreaks)} lines, {len(self.content)} characters)"

    def __repr__(self):
        # truncate content on both sides
        max_length = 250
        if len(self.content) > max_length:
            prefix_content = self.content[: max_length // 2]
            postfix_content = self.content[-max_length // 2 :]
            lines_omitted = (
                len(self.linebreaks)
                - len(prefix_content.splitlines())
                - len(postfix_content.splitlines())
            )
            content = (
                f"{prefix_content}\n... ({lines_omitted} lines omitted) ...\n{postfix_content}"
            )
        else:
            content = self.content
        return f"<{self.__class__.__name__}: {str(self)}\n{content}\n>"

    @cached_property
    def linebreaks(self) -> list[int]:
        # index start of each line
        linebreaks = [0]
        for match in re.finditer(r"\r?\n", self.content):
            linebreaks.append(match.end())
        return linebreaks

    def line(self, index: int) -> str:
        if index >= len(self.linebreaks):
            raise IndexError(f"line {index} does not exist")
        elif index == len(self.linebreaks) - 1:
            return self.content[self.linebreaks[index] :]
        else:
            return self.content[self.linebreaks[index] : self.linebreaks[index + 1]]


class TokenType(enum.Enum):
    NEW_FILE = "new_file"
    INDENT = "indent"
    COMMENT = "comment"
    WHITESPACE = "whitespace"
    KEYWORD = "keyword"
    SEPERATOR = "seperator"
    IDENTIFIER = "identifier"
    LITERAL = "literal"


@dataclass
class Token:
    source_file: SourceFile
    line_number: int
    line_span: int
    start_column: int
    end_column: int
    type: TokenType
    value: Optional[str | enum.Enum]

    def __str__(self):
        # truncate value if too long
        max_length = 100
        if self.value is None:
            value = "<none>"
        elif len(self.value) > max_length:
            value = f"{self.value[: max_length // 2]}...{self.value[-max_length // 2 :]}"
        else:
            value = self.value
        # replace newlines with literal \n
        value = value.replace("\n", "\\n")
        if self.line_span > 1:
            loc = f"{self.line_number}:{self.start_column}-{self.line_number + self.line_span}:{self.end_column}"
        else:
            loc = f"{self.line_number}:{self.start_column}-{self.end_column}"
        return f"{self.type.name} {value} ({self.source_file.path} {loc})"


class SyntaxError(ValueError):
    pass


KEYWORDS = {
    # StatementModifier
    "with": StatementModifier.WITH,
    "like": StatementModifier.LIKE,
    "unlike": StatementModifier.UNLIKE,
    "verify": StatementModifier.VERIFY,
    # SymbolType
    "schema": SymbolType.SCHEMA,
    "task": SymbolType.TASK,
    "expect": SymbolType.EXPECTATION,
    "code": SymbolType.CODE,
    "model": SymbolType.MODEL,
    "data": SymbolType.DATASET,
    "value": SymbolType.VALUE,
    # Other
    "as": None,
    "from": None,
    "import": StatementType.IMPORT,
    "require": StatementType.REQUIREMENT,
    "run": StatementType.RUNCONFIG,
    "compile": StatementType.COMPILATION,
}
SEPARATORS = {":", "=", "@"}


# any whitespace
WHITESPACE_REGEX = re.compile(r"\s+", re.MULTILINE)
# new file like --- <path> ---
NEW_FILE_REGEX = re.compile(r"^---\s*(?P<value>[\w\.-]*)\s*---$", re.MULTILINE)
# indent with 4 spaces or 1 tab
INDENT_REGEX = re.compile(r"^(?P<value> {4}|\t)")
# comment like # <comment>
COMMENT_REGEX = re.compile(r"^#\s*(?P<value>.*)\s*$", re.MULTILINE)
# keywords from set
KEYWORD_REGEX = re.compile(r"(?P<value>" + "|".join(KEYWORDS.keys()) + r")")
# seperator from set
SEPERATOR_REGEX = re.compile(r"(?P<value>" + "|".join(SEPARATORS) + r")")
# identifier like <12na_me-> or <name_.name> or '<name name name>'
# (allowed characters: a-z, A-Z, 0-9, _, -, . and whitespace in quotes)
IDENTIFIER_REGEX = re.compile(r"(?P<value>('\w[ \w.\-]*')|(\w[\w.-]*))")
# literal as `<value>` or ^```<multiline\n value>```$
# (two separate regexes to avoid multiline matching with re.DOTALL)
MULTILINE_LITERAL_REGEX = re.compile(r"^```(?P<value>.*?)```$", re.DOTALL | re.MULTILINE)
INLINE_LITERAL_REGEX = re.compile(r"`(?P<value>[^`]+)`")

# token type + corresponding pattern in lex order
PATTERNS = [
    (TokenType.WHITESPACE, WHITESPACE_REGEX),  # eat any whitespace
    (TokenType.NEW_FILE, NEW_FILE_REGEX),
    (TokenType.INDENT, INDENT_REGEX),
    (TokenType.COMMENT, COMMENT_REGEX),
    (TokenType.KEYWORD, KEYWORD_REGEX),
    (TokenType.SEPERATOR, SEPERATOR_REGEX),
    (TokenType.IDENTIFIER, IDENTIFIER_REGEX),
    (TokenType.LITERAL, MULTILINE_LITERAL_REGEX),
    (TokenType.LITERAL, INLINE_LITERAL_REGEX),
]


def lex(source: SourceFile) -> list[Token]:
    """Lex a source file into a list of tokens."""
    tokens = []

    prev_token = None
    while True:
        if prev_token is None:
            line_number = 1
            start_column = 0
        else:
            line_number = prev_token.line_number + prev_token.line_span
            start_column = prev_token.end_column

        current_pos = source.linebreaks[line_number - 1] + start_column
        token = _lex_token(source, current_pos)
        if token is None:
            # if we're at the end of the file, we're done
            if current_pos >= len(source.content):
                break
            else:  # otherwise, we have an error
                prev_lines = "".join(
                    source.line(line_number - i - 1) for i in reversed(range(0, 4))
                )
                if not prev_lines.endswith("\n"):
                    prev_lines = prev_lines + "\n"
                context = f"{prev_lines}{'-' * start_column}^"
                raise SyntaxError(
                    f"unknown token at {line_number}:{start_column} in {source.path}:\n{context}"
                )
        if token.type != TokenType.WHITESPACE:
            tokens.append(token)
            print(token)
        prev_token = token

    return tokens


def _lex_token(source: SourceFile, current_pos: int) -> Optional[Token]:
    """Lex a single token from a source file."""
    # try to match a pattern (once at current position)
    for token_type, pattern in PATTERNS:
        match = pattern.match(source.content, current_pos)
        if match is not None:
            break
    else:
        return None

    # create token (with value if group "value" exists)
    value = match.group("value") if "value" in match.groupdict() else None
    if token_type == TokenType.KEYWORD and KEYWORDS.get(value) is not None:
        value = KEYWORDS[value]
    line_number = source.content.count("\n", 0, match.start()) + 1
    line_span = source.content.count("\n", current_pos, match.end())
    start_column = match.start() - source.linebreaks[line_number - 1]
    end_column = match.end() - source.linebreaks[line_number + line_span - 1]
    return Token(
        source_file=source,
        line_number=line_number,
        line_span=line_span,
        start_column=start_column,
        end_column=end_column,
        type=token_type,
        value=value,
    )
