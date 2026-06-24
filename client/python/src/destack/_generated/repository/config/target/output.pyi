# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TargetOutputOptions:
    """Target output paths and metadata options."""

    # output directory for this target
    directory: str
    # output file for single-file targets
    file: str | None
    # whether to emit declaration files
    declaration: bool
    # separate directory for declaration files
    declaration_directory: str | None
    # source map emission mode
    source_map: SourceMapMode | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetOutputOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetOutputOptions: ...

def encode_target_output_options(
    writer: BinaryWriter, value: TargetOutputOptions
) -> None: ...
def decode_target_output_options(reader: BinaryReader) -> TargetOutputOptions: ...
def to_json_target_output_options(value: TargetOutputOptions) -> Json: ...
def from_json_target_output_options(value: Json) -> TargetOutputOptions: ...

"""Source map emission mode for one target."""
SourceMapMode: typing.TypeAlias = (
    typing.Literal["external"] | typing.Literal["inline"] | typing.Literal["hidden"]
)

def encode_source_map_mode(writer: BinaryWriter, value: SourceMapMode) -> None: ...
def decode_source_map_mode(reader: BinaryReader) -> SourceMapMode: ...
def to_json_source_map_mode(value: SourceMapMode) -> Json: ...
def from_json_source_map_mode(value: Json) -> SourceMapMode: ...

__all__ = [
    "TargetOutputOptions",
    "encode_target_output_options",
    "decode_target_output_options",
    "to_json_target_output_options",
    "from_json_target_output_options",
    "SourceMapMode",
    "encode_source_map_mode",
    "decode_source_map_mode",
    "to_json_source_map_mode",
    "from_json_source_map_mode",
]
