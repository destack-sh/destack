# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.target.js

@dataclass(frozen=True, slots=True)
class CompilerOptions:
    """Normalized Destack compiler options."""

    # javaScript module format for output
    module: destack._generated.repository.config.target.js.JsModuleFormat
    # ECMAScript target version
    es_target: destack._generated.repository.config.target.js.EsTarget
    # default profile
    profile: str | None
    # default active source graph modes
    modes: Sequence[str]
    # default active source graph roles
    roles: Sequence[str]
    # default active source graph features
    features: Sequence[str]
    # default active source graph tags
    tags: Sequence[str]
    # comptime environment whitelist (if omitted, all env keys are visible)
    comptime_env: Sequence[str] | None
    # default tree tag builder provider
    tree: str | None
    # global provider modules added to every target profile
    globals: Sequence[str]
    # well-known derives automatically considered for nominal declarations
    derive: Sequence[Derive]
    # static semantic restrictions
    restrictions: CompilerRestrictions
    # root directory of source files (controls output directory structure, not module resolution)
    root_dir: str | None
    # output directory for compiled files
    out_dir: str | None
    # do not emit output files
    no_emit: bool
    # emit phase stats sidecars
    emit_stats: bool
    # emit phase event sidecars
    emit_events: bool
    # emit checked type annotation sidecars
    emit_checked_types: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CompilerOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CompilerOptions: ...

def encode_compiler_options(writer: BinaryWriter, value: CompilerOptions) -> None: ...
def decode_compiler_options(reader: BinaryReader) -> CompilerOptions: ...
def to_json_compiler_options(value: CompilerOptions) -> Json: ...
def from_json_compiler_options(value: Json) -> CompilerOptions: ...

"""Well-known compiler-owned derive provider."""
Derive: typing.TypeAlias = (
    typing.Literal["compare"]
    | typing.Literal["copy"]
    | typing.Literal["clone"]
    | typing.Literal["debug"]
    | typing.Literal["default"]
    | typing.Literal["deserialize"]
    | typing.Literal["equal"]
    | typing.Literal["hash"]
    | typing.Literal["partialCompare"]
    | typing.Literal["partialEqual"]
    | typing.Literal["serialize"]
    | typing.Literal["tagged"]
)

def encode_derive(writer: BinaryWriter, value: Derive) -> None: ...
def decode_derive(reader: BinaryReader) -> Derive: ...
def to_json_derive(value: Derive) -> Json: ...
def from_json_derive(value: Json) -> Derive: ...

@dataclass(frozen=True, slots=True)
class CompilerRestrictions:
    """Static semantic restrictions enforced by the compiler."""

    # policy for managed values and managed allocation
    no_managed: DiagnosticPolicy
    # policy for all heap allocation
    no_heap: DiagnosticPolicy
    # policy for runtime-dependent language features
    no_runtime: DiagnosticPolicy
    # policy for unsafe operations
    no_unsafe: DiagnosticPolicy
    # policy for calls that cannot be statically resolved
    no_dynamic_dispatch: DiagnosticPolicy
    # policy for runtime reflection and RTTI usage
    no_reflection: DiagnosticPolicy
    # policy for unwinding
    no_unwind: DiagnosticPolicy
    # policy banning aliasing mutable borrows
    no_aliasing_mutable_borrows: DiagnosticPolicy
    # policy banning implicit method receivers
    no_implicit_receivers: DiagnosticPolicy

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CompilerRestrictions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CompilerRestrictions: ...

def encode_compiler_restrictions(
    writer: BinaryWriter, value: CompilerRestrictions
) -> None: ...
def decode_compiler_restrictions(reader: BinaryReader) -> CompilerRestrictions: ...
def to_json_compiler_restrictions(value: CompilerRestrictions) -> Json: ...
def from_json_compiler_restrictions(value: Json) -> CompilerRestrictions: ...

"""Diagnostic policy for allow/warn/deny enforcement."""
DiagnosticPolicy: typing.TypeAlias = (
    typing.Literal["allow"] | typing.Literal["warn"] | typing.Literal["deny"]
)

def encode_diagnostic_policy(writer: BinaryWriter, value: DiagnosticPolicy) -> None: ...
def decode_diagnostic_policy(reader: BinaryReader) -> DiagnosticPolicy: ...
def to_json_diagnostic_policy(value: DiagnosticPolicy) -> Json: ...
def from_json_diagnostic_policy(value: Json) -> DiagnosticPolicy: ...

__all__ = [
    "CompilerOptions",
    "encode_compiler_options",
    "decode_compiler_options",
    "to_json_compiler_options",
    "from_json_compiler_options",
    "Derive",
    "encode_derive",
    "decode_derive",
    "to_json_derive",
    "from_json_derive",
    "CompilerRestrictions",
    "encode_compiler_restrictions",
    "decode_compiler_restrictions",
    "to_json_compiler_restrictions",
    "from_json_compiler_restrictions",
    "DiagnosticPolicy",
    "encode_diagnostic_policy",
    "decode_diagnostic_policy",
    "to_json_diagnostic_policy",
    "from_json_diagnostic_policy",
]
