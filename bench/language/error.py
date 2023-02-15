import enum
import typing
from dataclasses import dataclass
from typing import Optional

from bench.language.type import (
    File,
    SourceFile,
    Statement,
    StatementPath,
    Token,
    statement_path_as_str,
)


def get_location_pointer(
    source: SourceFile, line_number: int, start_column: int, prev_lines: int = 4
) -> str:
    prev_lines = "> ".join(
        source.line(line_number - i) for i in reversed(range(0, min(line_number, prev_lines)))
    )
    if not prev_lines.endswith("\n"):
        prev_lines = prev_lines + "\n"
    context = f"> {prev_lines}> {'-' * start_column}^"
    return context


def get_location_range_pointer(
    source: SourceFile,
    start_line: int,
    start_column: int,
    end_line: int,
    end_column: int,
    prev_lines: int = 4,
) -> str:
    prev_lines = "> ".join(
        source.line(start_line - i) for i in reversed(range(0, min(start_line, prev_lines)))
    )
    if not prev_lines.endswith("\n"):
        prev_lines = prev_lines + "\n"
    # does not handle multiline ranges yet, so if it's multiline, we extend until end of first line
    if end_line > start_line:
        end_column = len(source.line(start_line))
    context = f"> {prev_lines}> {'-' * start_column}{'^' * (end_column - start_column)}"
    return context


class ErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    # syntax errors
    UNKNOWN_TOKEN = 1, "unknown token"
    # parser errors
    MISSING_TOKEN = 20, "expected a token"
    UNEXPECTED_TOKEN_TYPE = 21, "expected token type {type}"
    UNEXPECTED_TOKEN_VALUE = 22, "expected token value {value}"
    UNEXPECTED_INDENT = 23, "expected indent <= {indent}"
    EXPECTED_BLANK = 24, "expected blank line"
    MISSING_EXTRA = 25, "expected token value extra {extra}"
    UNEXPECTED_EXTRA = 26, "unexpected token value extra {extra}={value}"
    INVALID_TOKEN_VALUE = 27, "invalid token value {value} {error}"
    INVALID_STATEMENT = 28, "invalid statement"
    # resolve/interp errors
    UNKNOWN_IMPORT_SOURCE = 60, "unspecified import module source {source}"
    UNDEFINED_LOCAL_REFERENCE = 61, "undefined reference {path}"
    UNDEFINED_EXTERNAL_REFERENCE = 62, "undefined external reference {path} in module {module}"
    EXTERNAL_LOOKUP_FAILED = 63, "failed to lookup reference {path} in module {module}: {error}"
    REFERENCE_TYPE_MISMATCH = 64, "reference {resolved} is not of type {resolved}"
    AMBIGUOUS_DEFINITION = 65, "multiple definitions for {path}"
    AMBIGUOUS_REQUIREMENT = 66, "multiple requirements for {name}"
    UNEXPECTED_PARENT = 67, "unexpected parent {parent}"
    EXPECTED_PARENT = 68, "expected a parent"
    EXPECTED_PROPER_CHILDREN = 69, "expected proper children"
    UNEXPECTED_CHILDREN = 70, "unexpected children"
    EXPECTED_PARAMETERS = 71, "expected parameters of type {type}"
    UNEXPECTED_PARAMETERS = 72, "unexpected parameters"
    EXPECTED_ARGUMENTS = 73, "expected arguments of type {type}"
    UNEXPECTED_STATEMENT = 74, "unexpected statement"
    BUILD_MISSING_MODEL = 75, "missing model"
    BUILD_MISSING_TASK = 76, "missing task"
    CIRCULAR_ANCESTRY = 77, "Dark season 2 via {path}"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj

    @property
    def id(self) -> int:
        return self.value[0]


class SyntaxError(ValueError):
    def __int__(
        self,
        type: ErrorType,
        source_file: SourceFile,
        line_number: int,
        column: int,
    ):
        self.message = self._format_message(type, source_file, line_number, column)
        super().__init__(self.message)
        self.type = type
        self.source_file = source_file
        self.line_number = line_number
        self.column = column

    def _format_message(
        self,
        type: ErrorType,
        file: SourceFile,
        line_number: int,
        column: int,
    ) -> str:
        context = get_location_pointer(file, line_number, column)
        return f"{type.value} at {file.path}:{line_number}:{column}:\n{context}"

    def to_error(self) -> "Error":
        return Error(
            type=self.type,
            message=self.type.description,
            verbose_message=self.args[0],
            source_file=self.source_file,
            line_number=self.line_number,
            column=self.column,
        )


class ParseError(ValueError):
    def __init__(
        self,
        _t: ErrorType,
        token: Optional[Token],
        cause: Optional[Exception] = None,
        parser: Optional[str] = None,
        position: Optional[int] = -1,
        **error_args,
    ):
        self.short_message, message = self._format_message(_t, error_args, token, cause)
        super().__init__(message)
        self.type = _t
        self.token = token
        self.cause = cause
        self.position = position
        self.parser = parser
        self.error_args = error_args

    def _format_message(
        self, type: ErrorType, error_args: dict, token: Optional[Token], cause: Optional[dict]
    ) -> tuple[str, str]:
        # noinspection StrFormat
        short_message = type.description.format(**error_args)
        message = "{0}: {1}{2}{3}".format(
            type.name,
            short_message,
            ParseError.token_context(token),
            (f"\npossible cause: {cause}" if cause else ""),
        )
        return short_message, message

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

    def to_error(self) -> "Error":
        return Error(
            type=self.type,
            message=self.short_message,
            verbose_message=self.args[0],
            source_file=self.token.source_file,
            line_number=self.token.line_number,
            column=self.token.start_column,
        )


class SemanticError(ValueError):
    def __init__(
        self,
        _t: ErrorType,
        subject: File | Statement | None,
        cause: Optional[Exception] = None,
        **error_args,
    ):
        self.short_message, message = self._format_message(_t, error_args, subject, cause)
        super().__init__(message)
        self.type = _t
        self.subject = subject
        self.related_statements = {k: v for k, v in error_args.items() if isinstance(v, Statement)}
        self.cause = cause

    def _format_message(
        self,
        error_type: ErrorType,
        error_args: dict,
        subject: File | Statement | None,
        cause: Exception | None,
    ) -> tuple[str, str]:
        # convert error args as needed
        # StatementPath with statement_path_as_str
        error_args = {
            k: statement_path_as_str(v) if isinstance(v, StatementPath) else v
            for k, v in error_args.items()
        }
        # noinspection StrFormat
        message = error_type.description.format(**error_args)
        cause_context = f"\ncause: {cause.__class__.__name__} {cause}" if cause is not None else ""
        if isinstance(subject, Statement):
            thing_context = SemanticError.statement_context(subject)
        elif isinstance(subject, File):
            thing_context = f" in {subject.path}"
        else:
            thing_context = ""
        return message, message + thing_context + cause_context

    @staticmethod
    def statement_context(statement: Statement) -> str:
        if statement._source is None:
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
            return f" at\n> {statement}\n{source[0].source_file.path}:{source[0].line_number}\n{source_context}"

    def to_error(self) -> "Error":
        if isinstance(self.subject, Statement) and self.subject._source is not None:
            source = typing.cast(list[Token], self.subject._source)
            extras = dict(
                source_file=source[0].source_file,
                line_number=source[0].line_number,
                column=source[0].start_column,
            )
        else:
            extras = {}
        return Error(
            type=self.type,
            message=self.short_message,
            verbose_message=self.args[0],
            file=self.subject if isinstance(self.subject, File) else None,
            statement=self.subject if isinstance(self.subject, Statement) else None,
            **extras,
        )


@dataclass
class Error:
    type: ErrorType
    message: str
    verbose_message: Optional[str] = None
    source_file: Optional[SourceFile] = None
    line_number: Optional[int] = None
    column: Optional[int] = None
    token: Optional[Token] = None
    file: Optional[File] = None
    statement: Optional[Statement] = None
