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

from bench.language.error import IssueType, ParseError
from bench.language.lex import lex_string
from bench.language.type import (
    BuildContent,
    CapabilityContent,
    CodeContent,
    DataContent,
    ExpectationContent,
    Field,
    File,
    Module,
    Record,
    RequirementContent,
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
    TypeFlag,
    TypeNode,
    TypeTag,
)
from bench.utils.fractional import (
    INTEGER_ZERO,
    generate_key_between,
    generate_n_keys_between,
    increment_integer,
)


def raise_error(error: ValueError):
    raise error


def ignore_error(*args, **kwargs):
    pass


ErrorT = typing.TypeVar("ErrorT", bound=ValueError)


ET = IssueType
TT = TokenType


class TokenParser:
    """
    Iterates over the source stream with a peek window of 1 token.
    Automatically matches (and ignores) indentation at the set level (mutable).
    """

    def __init__(self, tokens: list[Token], start_pos: int = 0, indent_level: int = 0):
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


def _strip_literal_indent(text: str, indent_level: int) -> str:
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
    def next_order_key(self) -> str:
        if self.parent is None:
            siblings = self.root_statements
        else:
            siblings = self.children(self.parent)
        if len(siblings) == 0:
            return INTEGER_ZERO
        else:
            return increment_integer(siblings[-1].order_key)

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

    tokens = TokenParser(tokens, start_pos=0, indent_level=0)
    files: dict[str, FileParseState] = OrderedDict()  # remember original file order
    # tokens[0] must be newfile
    initial_file: File = File(module=module, path=tokens.eat_newfile().value)
    local = FileParseState(file=initial_file)
    files[initial_file.path] = local

    while tokens.peek() is not None:
        # reset indent if previous token was on a different line
        if tokens.previous is not None and tokens.previous.line_number != tokens.peek().line_number:
            local.indent = 0

        if tokens.peek().type == TT.NEWFILE:
            path = tokens.eat_newfile().value
            if path not in files:  # begin new file state
                new_file = File(module=module, path=path)
                files[path] = FileParseState(file=new_file)
            local = files[path]
        elif tokens.peek().type == TT.INDENT:
            tokens.eat()
            local.indent += 1
        else:  # parse statement
            if len(local.ancestors) < local.indent:  # too much indentation
                raise ParseError(ET.UNEXPECTED_INDENT, tokens.peek(), indent=local.indent)

            errors: list[ParseError] = []
            tokens.indent_level = local.indent  # skip indent tokens at current level
            start_mark = tokens.mark()
            statement = _parse_statement(
                tokens,
                is_root=local.root,
                on_error=errors.append,
                file=local.file,
                parent=local.parent,
                order_key=local.next_order_key,
            )
            tokens.indent_level = 0  # skip only for statement parsing

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
                error = ParseError(ET.INVALID_STATEMENT, tokens.peek(), cause=cause)
                on_error(error)
                continue

            statement._source = tokens.eaten(start_mark)

            # only comments/blanks can have comments/blanks as parents
            if (
                local.parent
                and local.parent.type in (StatementType.BLANK, StatementType.COMMENT)
                and statement.type not in (StatementType.BLANK, StatementType.COMMENT)
            ):
                # can't have children
                raise ParseError(ET.UNEXPECTED_INDENT, tokens.peek(), indent=local.indent)
            local.add_statement(statement)

    return [state.file for state in files.values()]


def _parse_comment(tokens: TokenParser, **kwargs) -> Statement:
    token = tokens.eat_type(TT.COMMENT)
    tokens.eat_newline_or_eos()
    return Statement(type=StatementType.COMMENT, text=token.value, **kwargs)


# identifier names can be escaped as '<name with space>'
# references can look like
# 1. <name>
# 2. .<path>.<name>
# 3. <module_owner>.<module_name>.<path>.<name>
REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?(\.(?P<path>[\w.\- ]+)\.)?(?P<name>[\w\- ]+)$"
)
# import source must either be in current module (.*) or absolute (<owner>.<name>.*)
RELATIVE_REFERENCE_REGEX = re.compile(r"^\.(?P<path>[\w.\- ]+)$")
ABSOLUTE_IMPORT_SOURCE_REGEX = re.compile(
    r"^(?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+)\.(?P<path>[\w.\- ]+)$"
)


def _parse_typed_reference_slot(tokens: TokenParser) -> tuple[StatementPath, Token]:
    symbol_type = tokens.eat_keyword_like(SymbolType)
    tokens.eat_space()
    path = _parse_reference_slot(tokens)
    return path, symbol_type


def _parse_reference_slot(tokens: TokenParser) -> StatementPath:
    # references can look like
    # 1. <name>
    # 2. .<path>.<name>
    # 3. <module>.<path>.<name>
    reference = tokens.eat_identifier()
    path_match = REFERENCE_REGEX.match(reference.value)
    if path_match is None:
        raise ParseError(ET.INVALID_TOKEN_VALUE, reference, value=reference.value, error="invalid")
    if path_match.group("path") is not None:
        if path_match.group("module_owner") is not None:
            path = f"{path_match.group('module_owner')}.{path_match.group('module_name')}.{path_match.group('path')}"
        else:
            path = f".{path_match.group('path')}"
    else:
        path = "."
    name = path_match.group("name")
    return StatementPath(path, name)


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
    if (
        RELATIVE_REFERENCE_REGEX.match(source.value) is None
        and ABSOLUTE_IMPORT_SOURCE_REGEX.match(source.value) is None
    ):
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
        type = parse_type_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return TaskContent(
            name=name.value,
            description=description.value,
            tag=TypeTag.FUNCTION,
            fields=type.fields,
        )
    elif symbol_type.value == SymbolType.EXPECTATION:
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = tokens.eat_description()
        return ExpectationContent(description=description.value)
    elif symbol_type.value == SymbolType.CODE:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_func(tokens, name=name.value)
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        if lang not in ("python", "py", "x"):
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
        if lang == "py":
            lang = "python"
        code_text = _strip_literal_indent(literal.value, tokens.indent_level)
        return CodeContent(
            description=description,
            language=lang,  # noqa
            code=code_text,
            tag=TypeTag.FUNCTION,
            fields=type.fields,
            xblocks=[],  # not parsed yet
        )
    elif symbol_type.value == SymbolType.DATA:
        tokens.eat_space()
        tokens.eat_separator("::")
        tokens.eat_space()
        type = parse_type_struct(tokens, name="element")
        tokens.eat_separator(":")
        tokens.eat_newline()
        description = _parse_description_line_optional(tokens)
        literal = tokens.eat_literal()
        lang = literal.value_extras.get("lang")
        records = _parse_dataset_records(tokens, type, lang, literal)
        return DataContent(
            description=description,
            language=lang,
            records=records,
            tag=TypeTag.STRUCT,
            fields=type.fields,
        )
    elif symbol_type.value == SymbolType.BUILD:
        tokens.eat_separator(":")
        return BuildContent()

    raise ParseError(ET.UNEXPECTED_TOKEN_VALUE, symbol_type, type=TT.KEYWORD, value=SymbolType)


def _parse_dataset_records(
    tokens: TokenParser, type: TypeNode, lang: str | None, literal: Token
) -> list[Record]:
    """Parses the language and records from a dataset literal."""
    value_str = _strip_literal_indent(literal.value, tokens.indent_level)
    try:
        if lang is None:
            raise ParseError(ET.MISSING_EXTRA, literal, extra="lang")
        elif lang == "jsonl":
            records_data = [json.loads(line) for line in value_str.splitlines()]
        elif lang == "json":
            records_data = json.loads(value_str)
        elif lang == "csv":
            field_names = [element.name for element in type.fields]
            csv_reader = csv.DictReader(
                value_str.splitlines(), quoting=csv.QUOTE_NONNUMERIC, fieldnames=field_names
            )
            records_data = list(csv_reader)
        else:
            raise ParseError(ET.UNEXPECTED_EXTRA, literal, extra="lang", value=lang)
    except ParseError:
        raise  # re-raise since we don't want to catch our own errors
    except ValueError as e:
        raise ParseError(ET.INVALID_TOKEN_VALUE, literal, value=value_str, error=e)

    order_keys = generate_n_keys_between(None, None, len(records_data))
    records = [
        Record(data=data, order_key=order_key) for data, order_key in zip(records_data, order_keys)
    ]
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
    definition.content = RequirementContent(
        module_name=dependency.value, version=version.value, module_id=None
    )
    return definition


def _parse_definition_type(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a type statement (special path because of special syntax)."""
    tokens.eat_keyword(SymbolType.TYPE)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline()
    description = _parse_description_line_optional(tokens)
    struct = parse_type_struct_def(tokens, name=name.value)
    struct.description = description
    tokens.eat_newline_or_eos()
    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        content=struct,
        **kwargs,
    )
    return definition


def _parse_definition_enum(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a value enum statement (special path because of special syntax)"""
    tokens.eat_keyword(TypeTag.ENUM)
    tokens.eat_space()
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_newline()
    description = _parse_description_line_optional(tokens)

    # parse members (assumes literal members only)
    members: list[Field] = []
    while tokens.peek_separator("-"):
        tokens.eat_separator("-")
        tokens.eat_space()
        member_name = tokens.eat_identifier().value
        member_description = None
        if tokens.peek_separator(" "):
            tokens.eat_space()
            member_description = tokens.eat_description().value
        last_order_key = members[-1].order_key if members else INTEGER_ZERO
        member = Field(
            order_key=generate_key_between(None, last_order_key),
            name=member_name,
            tag=TypeTag.LITERAL,
            value=member_name,
            description=member_description,
        )
        members.append(member)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()

    enum_field = TypeContent(
        name=name.value,
        description=description,
        tag=TypeTag.ENUM,
        fields=[*members],
    )
    definition = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        content=enum_field,
        **kwargs,
    )
    return definition


def parse_field(tokens: TokenParser) -> Field:
    """Parse single tuple like <name>: <type>[ "<description>"]"""
    name = tokens.eat_identifier()
    tokens.eat_separator(":")
    tokens.eat_space()
    return parse_field_inline(tokens, name.value)


def parse_field_inline(tokens: TokenParser, name: str | None) -> Field:
    """Parse a type node type including description, handling simple nesting.."""

    flags = TypeFlag.Zero
    # parse array like [<type>] (one layer only)
    if tokens.peek_bracket("["):
        flags |= TypeFlag.IsArray
        tokens.eat_bracket("[")

    # parse actual type as either primitive or type reference
    reference = None
    value = None
    if tokens.peek_keyword_like(TypeTag):
        # not all value types are keywords, but only the valid ones are in KEYWORDS
        type = tokens.eat_keyword_like(TypeTag).value
    elif tokens.peek_type(TokenType.LITERAL):
        type = TypeTag.LITERAL
        value_token = tokens.eat_literal()
        try:
            value = json.loads(value_token.value)
        except ValueError as e:
            raise ParseError(ET.INVALID_TOKEN_VALUE, value_token, error=e, value=value)
    else:
        type = TypeTag.TYPE_REFERENCE
        reference = _parse_reference_slot(tokens)

    if flags & TypeFlag.IsArray:
        tokens.eat_bracket("]")

    if tokens.peek_type(TokenType.MARK_OPTIONAL):
        tokens.eat()
        flags |= TypeFlag.IsNullable

    if tokens.peek_separator(" "):
        tokens.eat_space()
        description = tokens.eat_description().value
    else:
        description = None

    return Field(
        name=name,
        tag=type,
        value=value,
        reference=reference,
        description=description,
        flags=flags,
    )


def assign_field_oks(nodes: list[Field]) -> list[Field]:
    """Assign order keys to type nodes and return the list of nodes."""
    oks = generate_n_keys_between(None, None, len(nodes))
    for node, ok in zip(nodes, oks):
        node.order_key = ok
    return nodes


def parse_type_struct_def(tokens: TokenParser, name: str | None) -> TypeContent:
    # parse tuples like <tuple1>\n<tuple2>\n...
    struct = TypeContent(name=name, tag=TypeTag.STRUCT)
    while tokens.peek_separator("-") or tokens.peek_separator("&"):
        sep = tokens.eat()
        tokens.eat_space()
        if sep.value == "&":
            node = Field(
                name=None,
                tag=TypeTag.TYPE_REFERENCE,
                reference=(_parse_reference_slot(tokens)),
                flags=TypeFlag.IsUnionWith,
            )
        else:
            node = parse_field(tokens)
        struct.fields.append(node)
        if not tokens.peek_type(TokenType.NEWLINE):
            break
        tokens.eat_newline_or_eos()
        if not (tokens.peek_separator("-") or tokens.peek_separator("&")):
            tokens.advance(-1)  # go back one token to leave newline separator
            break
    assign_field_oks(struct.fields)
    return struct


def parse_type_struct(
    tokens: TokenParser, name: str | None, is_output: bool = False
) -> TypeContent:
    """Parse an entire struct with possible unions like (...) & ... & ..."""

    def _parse_union_join():
        if tokens.peek_separator(" "):
            tokens.eat_separator(" ")
            if tokens.peek_separator("&"):
                tokens.eat_separator("&")
                tokens.eat_separator(" ")
                return True
            else:
                tokens.advance(-1)  # go back one token
        return False

    expect_union = True
    if tokens.peek_bracket("("):
        tokens.eat_bracket("(")
        struct = parse_type_struct_inline(tokens, name, is_output=is_output)
        tokens.eat_bracket(")")

        # parse start of union if there is one
        expect_union = _parse_union_join()
    else:
        struct = TypeContent(name=name, tag=TypeTag.STRUCT)

    while expect_union:
        node = Field(
            name=None,
            tag=TypeTag.TYPE_REFERENCE,
            flags=TypeFlag.IsUnionWith | (TypeFlag.IsOutput if is_output else 0),
            reference=_parse_reference_slot(tokens),
        )
        struct.fields.append(node)
        expect_union = _parse_union_join()

    assign_field_oks(struct.fields)

    return struct


def parse_type_struct_inline(
    tokens: TokenParser, name: str | None, is_output: bool = False
) -> TypeContent:
    """Parse the inner part of an inline struct like name1: type1, name2: type2, ..."""
    struct = TypeContent(name=name, tag=TypeTag.STRUCT)
    while not tokens.peek_bracket(")"):
        tuple = parse_field(tokens)
        if is_output:
            tuple.flags = tuple.flags | TypeFlag.IsOutput
        struct.fields.append(tuple)
        if not tokens.peek_separator(","):
            break
        tokens.eat_separator(",")
        tokens.eat_space()
    assign_field_oks(struct.fields)
    return struct


def parse_type_func(tokens: TokenParser, name: str | None) -> TypeContent:
    # parse signature like (<tuple1>, <tuple2>, ...) -> (<tuple1>, <tuple2>, ...)
    nodes = parse_type_struct(tokens, name=None).fields
    if tokens.peek_separator(" "):
        tokens.eat_space()
        tokens.eat_separator("->")
        tokens.eat_space()
        nodes.extend(parse_type_struct(tokens, name=None, is_output=True).fields)
    assign_field_oks(nodes)
    return TypeContent(name=name, tag=TypeTag.FUNCTION, fields=nodes)


def _parse_redefinition(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a redefinition statement."""
    modifier = _parse_modifier_slot(tokens)
    symbol_type = tokens.eat_keyword_like(SymbolType)
    name = tokens.eat_identifier()
    tokens.eat_space()
    tokens.eat_separator("=")
    tokens.eat_space()
    reference = _parse_reference_slot(tokens)
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
        reference=reference,
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
    node = parse_field_inline(tokens, name=None)
    tokens.eat_newline_or_eos()

    # like other "redefinitions", type aliases are just syntactic sugar for definitions
    statement = Statement(
        type=StatementType.DEFINITION,
        symbol_type=SymbolType.TYPE,
        name=name.value,
        modifier=modifier,
        content=TypeContent(name=None, description=node.description, tag=node.tag),
        **kwargs,
    )
    return statement


def _parse_reference(tokens: TokenParser, **kwargs) -> Statement:
    """Parse a reference statement."""
    modifier = _parse_modifier_slot(tokens)
    reference, symbol_type = _parse_typed_reference_slot(tokens)
    tokens.eat_newline_or_eos()
    return Statement(
        type=StatementType.REFERENCE,
        modifier=modifier,
        name=reference.name,
        reference=reference,
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
