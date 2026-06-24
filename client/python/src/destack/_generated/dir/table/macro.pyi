# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class MacroTable:
    """Macro expansion state for one DIR module."""

    # the module id of the macro table
    module_id: destack._generated.source.file.model.module.ModuleId
    # expanded macro invocations
    invocations: Sequence[MacroInvocation]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MacroTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MacroTable: ...

def encode_macro_table(writer: BinaryWriter, value: MacroTable) -> None: ...
def decode_macro_table(reader: BinaryReader) -> MacroTable: ...
def to_json_macro_table(value: MacroTable) -> Json: ...
def from_json_macro_table(value: Json) -> MacroTable: ...

@dataclass(frozen=True, slots=True)
class MacroInvocation:
    """One macro invocation completed during fixed-point expansion."""

    # the decorated target node
    target_node: destack._generated.dir.tree.node.GlobalNodeIdAny
    # what caused this macro invocation
    trigger: MacroTrigger
    # the resolved `Macro` implementation
    implementation: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the static macro configuration
    config: destack._generated.dir.tree.static.StaticTerm
    # state carried from expansion to materialization
    state: destack._generated.dir.tree.static.StaticTerm | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MacroInvocation: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MacroInvocation: ...

def encode_macro_invocation(writer: BinaryWriter, value: MacroInvocation) -> None: ...
def decode_macro_invocation(reader: BinaryReader) -> MacroInvocation: ...
def to_json_macro_invocation(value: MacroInvocation) -> Json: ...
def from_json_macro_invocation(value: Json) -> MacroInvocation: ...

@dataclass(frozen=True, slots=True)
class MacroTriggerDecorator:
    """A decorator on the target node."""

    decorator: destack._generated.dir.tree.node.GlobalNodeId
    kind: typing.Literal["decorator"] = "decorator"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MacroTriggerAutoDerive:
    """A configured auto-derive provider."""

    kind: typing.Literal["autoDerive"] = "autoDerive"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""What caused one macro invocation."""
MacroTrigger: typing.TypeAlias = MacroTriggerDecorator | MacroTriggerAutoDerive

def encode_macro_trigger(writer: BinaryWriter, value: MacroTrigger) -> None: ...
def decode_macro_trigger(reader: BinaryReader) -> MacroTrigger: ...
def to_json_macro_trigger(value: MacroTrigger) -> Json: ...
def from_json_macro_trigger(value: Json) -> MacroTrigger: ...

__all__ = [
    "MacroTable",
    "encode_macro_table",
    "decode_macro_table",
    "to_json_macro_table",
    "from_json_macro_table",
    "MacroInvocation",
    "encode_macro_invocation",
    "decode_macro_invocation",
    "to_json_macro_invocation",
    "from_json_macro_invocation",
    "MacroTrigger",
    "encode_macro_trigger",
    "decode_macro_trigger",
    "to_json_macro_trigger",
    "from_json_macro_trigger",
    "MacroTriggerDecorator",
    "MacroTriggerAutoDerive",
]
