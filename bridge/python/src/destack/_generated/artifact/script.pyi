# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.map
import destack._generated.js.tree.script

"""One structured target language."""
ScriptLanguage: typing.TypeAlias = (
    typing.Literal["javaScript"] | typing.Literal["typeScript"]
)

def encode_script_language(writer: BinaryWriter, value: ScriptLanguage) -> None: ...
def decode_script_language(reader: BinaryReader) -> ScriptLanguage: ...
def to_json_script_language(value: ScriptLanguage) -> Json: ...
def from_json_script_language(value: Json) -> ScriptLanguage: ...

@dataclass(frozen=True, slots=True)
class Declaration:
    """One emitted declaration payload."""

    # the emitted declaration text
    text: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Declaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Declaration: ...

def encode_declaration(writer: BinaryWriter, value: Declaration) -> None: ...
def decode_declaration(reader: BinaryReader) -> Declaration: ...
def to_json_declaration(value: Declaration) -> Json: ...
def from_json_declaration(value: Json) -> Declaration: ...

@dataclass(frozen=True, slots=True)
class ScriptBodyEcmaScript:
    """ECMAScript-family module IR."""

    ecma_script: destack._generated.js.tree.script.Module
    kind: typing.Literal["ecmaScript"] = "ecmaScript"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One structured script body."""
ScriptBody: typing.TypeAlias = ScriptBodyEcmaScript

def encode_script_body(writer: BinaryWriter, value: ScriptBody) -> None: ...
def decode_script_body(reader: BinaryReader) -> ScriptBody: ...
def to_json_script_body(value: ScriptBody) -> Json: ...
def from_json_script_body(value: Json) -> ScriptBody: ...

@dataclass(frozen=True, slots=True)
class Script:
    """One structured script linker input for a target."""

    # the target language of this script
    language: ScriptLanguage
    # the structured script body
    body: ScriptBody
    # the emitted declaration when one exists
    declaration: Declaration | None
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None
    # whether this script has top level side effects
    has_top_level_side_effects: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Script: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Script: ...

def encode_script(writer: BinaryWriter, value: Script) -> None: ...
def decode_script(reader: BinaryReader) -> Script: ...
def to_json_script(value: Script) -> Json: ...
def from_json_script(value: Json) -> Script: ...

__all__ = [
    "ScriptLanguage",
    "encode_script_language",
    "decode_script_language",
    "to_json_script_language",
    "from_json_script_language",
    "Declaration",
    "encode_declaration",
    "decode_declaration",
    "to_json_declaration",
    "from_json_declaration",
    "ScriptBody",
    "encode_script_body",
    "decode_script_body",
    "to_json_script_body",
    "from_json_script_body",
    "ScriptBodyEcmaScript",
    "Script",
    "encode_script",
    "decode_script",
    "to_json_script",
    "from_json_script",
]
