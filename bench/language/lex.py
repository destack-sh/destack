from __future__ import annotations

import re
from typing import Optional

from bench.language.error import ErrorType, SyntaxError
from bench.language.type import (
    SourceFile,
    StatementModifier,
    StatementType,
    SymbolType,
    Token,
    TokenType,
    TypeTag,
)

KEYWORDS = {
    # StatementModifier
    "var": StatementModifier.VAR,
    "with": StatementModifier.WITH,
    "like": StatementModifier.LIKE,
    "unlike": StatementModifier.UNLIKE,
    "check": StatementModifier.CHECK,
    # SymbolType
    "type": SymbolType.TYPE,
    "capability": SymbolType.CAPABILITY,
    "task": SymbolType.TASK,
    "expect": SymbolType.EXPECTATION,
    "code": SymbolType.CODE,
    "model": SymbolType.MODEL,
    "data": SymbolType.DATA,
    "import": StatementType.IMPORT,
    "require": SymbolType.REQUIREMENT,
    "run": SymbolType.RUNCONFIG,
    "build": SymbolType.BUILD,
    # ValueType
    "string": TypeTag.STRING,
    "number": TypeTag.NUMBER,
    "boolean": TypeTag.BOOLEAN,
    "embedding": TypeTag.EMBEDDING,
    "file": TypeTag.FILE,
    "image": TypeTag.IMAGE,
    "video": TypeTag.VIDEO,
    "audio": TypeTag.AUDIO,
    "null": TypeTag.NULL,
    "enum": TypeTag.ENUM,
    "any": TypeTag.ANY,
    # Other
    "on": None,
    "as": None,
    "from": None,
}
SEPARATORS = [" ", r"\|", "&", ",", "::", ":", "=", "@", "->", "-"]  # order matters (LTR)!

# indent with 4 spaces or 1 tab
INDENT_REGEX = re.compile(r"(?P<value>( {4})|\t)", re.MULTILINE)
# any whitespace except indent
NEWLINE_REGEX = re.compile(r"(?P<value>[\n\r\f\v])")
# new file like --- <path> --- (eating previous newline)
# (eating the previous newline should be a parsing concern, but it's easier in lex for now)
NEWFILE_REGEX = re.compile(r"^\n?--- (?P<value>[\w.\- ]*) ---$\n", re.MULTILINE)
# comment like # <comment>
LINE_COMMENT_REGEX = re.compile(r"# (?P<value>.*)")
MULTILINE_COMMENT_REGEX = re.compile(r"###\n(?P<value>.+?)\n[ \t]*###", re.DOTALL | re.MULTILINE)
# keywords from set
# (must have non-word character after, but that is not considered part of the token)
KEYWORD_REGEX = re.compile(r"(?P<value>" + "|".join(KEYWORDS.keys()) + r")(?=\W|$)", re.MULTILINE)
# separator from set
SEPARATOR_REGEX = re.compile(r"(?P<value>" + "|".join(SEPARATORS) + r")")
# identifier like <12na_me-> or <name_.name> or '<name name name>'
# (allowed characters: a-z, A-Z, 0-9, _, -, . and whitespace in quotes)
IDENTIFIER_REGEX = re.compile(r"(?P<value>([\w.\-][\w.-]*))")
ESCAPED_IDENTIFIER_REGEX = re.compile(r"'(?P<value>[\w.\-][ \w.\-]*)'")
# literal as `<value>`{<lang>}? or ^```<lang>?\n<multi \n line \n value>\n```$
MULTILINE_LITERAL_REGEX = re.compile(
    r"```(?P<lang>\w+)?\n(?P<value>.*?)\n[ \t]*```", re.DOTALL | re.MULTILINE
)
INLINE_LITERAL_REGEX = re.compile(r"`(?P<value>[^`\n]+)`({\.(?P<lang>\w+)})?")
# descriptions as "<value>"
DESCRIPTION_REGEX = re.compile(r'"(?P<value>[^"\n]*)"')

# token type + corresponding pattern in lex order
TOKEN_PATTERNS = [
    (TokenType.NEWFILE, NEWFILE_REGEX),
    (TokenType.INDENT, INDENT_REGEX),
    # eat any other whitespace
    (TokenType.NEWLINE, NEWLINE_REGEX),
    (TokenType.COMMENT, MULTILINE_COMMENT_REGEX),
    (TokenType.COMMENT, LINE_COMMENT_REGEX),
    (TokenType.KEYWORD, KEYWORD_REGEX),
    (TokenType.SEPARATOR, SEPARATOR_REGEX),
    (TokenType.IDENTIFIER, IDENTIFIER_REGEX),
    (TokenType.IDENTIFIER, ESCAPED_IDENTIFIER_REGEX),
    (TokenType.LITERAL, MULTILINE_LITERAL_REGEX),
    (TokenType.LITERAL, INLINE_LITERAL_REGEX),
    (TokenType.DESCRIPTION, DESCRIPTION_REGEX),
    (TokenType.MARK_OPTIONAL, re.compile(r"\?")),
    (TokenType.BRACKET, re.compile(r"(?P<value>[(\[)\]])")),
    (TokenType.MARK_OPTIONAL, re.compile(r"(?P<value>\?)")),
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
        if current_pos >= len(source.content):
            break  # EOF, done

        token = _lex_token(source, current_pos)
        if token is None:
            raise SyntaxError(ErrorType.UNKNOWN_TOKEN, source, line_number, start_column)
        tokens.append(token)
        prev_token = token

    return tokens


def lex_string(source: str) -> list[Token]:
    """Lex a string into a list of tokens."""
    if source is None:
        raise ValueError("source must not be None")
    return lex(SourceFile("<string>", content=source))


def _lex_token(source: SourceFile, current_pos: int) -> Optional[Token]:
    """Lex a single token from a source file."""
    # try to match a pattern (once at current position)
    for token_type, pattern in TOKEN_PATTERNS:
        match = pattern.match(source.content, current_pos)
        if match is not None:
            break
    else:
        return None

    # create token (with value if group "value" exists)
    value = match.group("value") if "value" in match.groupdict() else None
    if token_type == TokenType.KEYWORD and KEYWORDS.get(value) is not None:
        value = KEYWORDS[value]  # map to enum
    # add additional groups as value_extras
    value_extras = {k: v for k, v in match.groupdict().items() if k != "value" and v is not None}

    line_number = source.content.count("\n", 0, match.start()) + 1
    line_span = source.content.count("\n", current_pos, match.end() - 1)
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
        value_extras=value_extras,
    )
