# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class LinterOptions:
    """Linter configuration."""

    # whether linting is enabled
    enabled: bool
    # explicit levels keyed by rule id or diagnostic code
    rules: Mapping[str, LintLevel]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterOptions: ...

def encode_linter_options(writer: BinaryWriter, value: LinterOptions) -> None: ...
def decode_linter_options(reader: BinaryReader) -> LinterOptions: ...
def to_json_linter_options(value: LinterOptions) -> Json: ...
def from_json_linter_options(value: Json) -> LinterOptions: ...

"""One configured lint level."""
LintLevel: typing.TypeAlias = (
    typing.Literal["off"] | typing.Literal["warning"] | typing.Literal["error"]
)

def encode_lint_level(writer: BinaryWriter, value: LintLevel) -> None: ...
def decode_lint_level(reader: BinaryReader) -> LintLevel: ...
def to_json_lint_level(value: LintLevel) -> Json: ...
def from_json_lint_level(value: Json) -> LintLevel: ...

__all__ = [
    "LinterOptions",
    "encode_linter_options",
    "decode_linter_options",
    "to_json_linter_options",
    "from_json_linter_options",
    "LintLevel",
    "encode_lint_level",
    "decode_lint_level",
    "to_json_lint_level",
    "from_json_lint_level",
]
