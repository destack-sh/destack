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
        linecount = len(self.linebreaks)
        return f"{self.path} ({linecount} lines)"

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


class TokenType(enum.Enum):
    NEW_FILE = "new_file"
    INDENT = "indent"
    COMMENT = "comment"
    BLANK = "blank"
    KEYWORD = "keyword"
    SEPERATOR = "seperator"
    IDENTIFIER = "identifier"
    LITERAL = "literal"


class SyntaxError(ValueError):
    pass


KEYWORDS = {
    # StatementModifier
    **{modifier.value: modifier for modifier in StatementModifier},
    # SymbolType
    **{symbol_type.value: symbol_type for symbol_type in SymbolType},
    # Other
    "as": None,
    "from": None,
    "import": StatementType.IMPORT,
    "require": StatementType.REQUIREMENT,
    "run": StatementType.RUNCONFIG,
    "compile": StatementType.COMPILATION,
}
SEPARATORS = {":", "=", "\n"}


# new file like --- <path> ---
NEW_FILE_REGEX = re.compile(r"^---\s*(?P<value>.*)\s*---$", re.MULTILINE)
# indent with 4 spaces or 1 tab
INDENT_REGEX = re.compile(r"^(?P<value> {4}|\t)")
# comment like # <comment>
COMMENT_REGEX = re.compile(r"^#\s*(?P<value>.*)\s*$", re.MULTILINE)
# blank lines
BLANK_REGEX = re.compile(r"^\s*$", re.MULTILINE)
# keywords from set
KEYWORD_REGEX = re.compile(r"(?P<value>" + "|".join(KEYWORDS.keys()) + r")")
# seperator from set
SEPERATOR_REGEX = re.compile(r"(?P<value>" + "|".join(SEPARATORS) + r")")
# identifier like <12na_me-> or <name_.name> or '<name name name>'
# (allowed characters: a-z, A-Z, 0-9, _, -, . and whitespace in quotes)
IDENTIFIER_REGEX = re.compile(r"(?P<value>'([a-zA-Z_][ a-zA-Z0-9_]')|([a-zA-Z_][a-zA-Z0-9_.-]*))")
# literal as `<value>` or ^```<multiline\n value>```$
# (two separate regexes to avoid multiline matching with re.DOTALL)
MULTILINE_LITERAL_REGEX = re.compile(r"^```(?P<value>.*)```$", re.DOTALL | re.MULTILINE)
INLINE_LITERAL_REGEX = re.compile(r"`(?P<value>[^`]+)`")

# token type + corresponding pattern in lex order
PATTERNS = [
    (TokenType.NEW_FILE, NEW_FILE_REGEX),
    (TokenType.INDENT, INDENT_REGEX),
    (TokenType.COMMENT, COMMENT_REGEX),
    (TokenType.BLANK, BLANK_REGEX),
    (TokenType.KEYWORD, KEYWORD_REGEX),
    (TokenType.SEPERATOR, SEPERATOR_REGEX),
    (TokenType.IDENTIFIER, IDENTIFIER_REGEX),
    (TokenType.LITERAL, MULTILINE_LITERAL_REGEX),
    (TokenType.LITERAL, INLINE_LITERAL_REGEX),
]


@dataclass
class Token:
    source_file: SourceFile
    line_number: int
    line_span: int
    start_column: Optional[int]
    end_column: Optional[int]
    type: TokenType
    value: Optional[str]

    def __str__(self):
        return f"{self.type.name} {self.value!r} ({self.line_number}:{self.start_column}-{self.end_column})"


def lex(source_file: SourceFile) -> list[Token]:
    """Lex a source file into a list of tokens."""
    tokens = []

    prev_token = None
    while True:
        token = _lex_token(source_file, prev_token)
        if token is None:
            break
        tokens.append(token)
        prev_token = token

    return tokens


def _lex_token(source_file: SourceFile, prev_token: Optional[Token]) -> Optional[Token]:
    """Lex a single token from a source file."""
    if prev_token is None:
        line_number = 0
        start_column = 0
    else:
        line_number = prev_token.line_number
        start_column = prev_token.end_column

    current_pos = source_file.linebreaks[line_number] + start_column
    # try to match a pattern (once at current position)
    for token_type, pattern in PATTERNS:
        match = pattern.match(source_file.content, current_pos)
        if match is not None:
            break
    else:
        preview_length = 35
        next_few_chars = source_file.content[current_pos : current_pos + preview_length]
        if len(next_few_chars) == preview_length:
            next_few_chars += "..."
        raise SyntaxError(
            f"unknown token at {line_number}:{start_column} in {source_file.path}: {next_few_chars!r}"
        )

    # create token
    value = match.group("value")
    return Token(
        source_file=source_file,
        line_number=line_number,
        line_span=source_file.content.count("\n", current_pos, match.end()),
        start_column=start_column,
        end_column=start_column + len(value),
        type=token_type,
        value=value,
    )
