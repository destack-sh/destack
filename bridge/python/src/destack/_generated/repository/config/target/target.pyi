# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.target
import destack._generated.artifact.emit
import destack._generated.repository.config.policy
import destack._generated.repository.config.runtime.runtime
import destack._generated.repository.config.target.compiler
import destack._generated.repository.config.target.condition
import destack._generated.repository.config.target.js
import destack._generated.repository.config.target.native
import destack._generated.repository.config.target.output

@dataclass(frozen=True, slots=True)
class Target:
    """A build target configuration."""

    # entry points for entry-rooted targets
    entry: Sequence[str]
    # user callable launched by executable products
    entrypoint: Entrypoint | None
    # global modules added to every target root set
    globals: Sequence[str]
    # glob patterns for files to include when no entry is declared
    include: Sequence[str]
    # glob patterns for files to exclude
    exclude: Sequence[str]
    # policy declarations and rules for this target
    policy: destack._generated.repository.config.policy.Policy
    # emitted artifact family (js, ts, wasm, native)
    emit: destack._generated.artifact.emit.EmitFormat
    # target operating system
    platform: destack._generated.artifact.core.target.Platform
    # target host environment
    host: destack._generated.artifact.core.target.Host
    # source graph conditions for this target
    conditions: destack._generated.repository.config.target.condition.TargetConditionSet
    # compiler behavior for this target
    compiler: destack._generated.repository.config.target.compiler.TargetCompilerOptions
    # output paths and metadata options
    output: destack._generated.repository.config.target.output.TargetOutputOptions
    # javaScript output configuration
    js: destack._generated.repository.config.target.js.TargetJsOptions
    # native codegen and linking configuration
    native: destack._generated.repository.config.target.native.TargetNativeOptions
    # runtime execution options
    execution: destack._generated.repository.config.runtime.runtime.RuntimeOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Target: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Target: ...

def encode_target(writer: BinaryWriter, value: Target) -> None: ...
def decode_target(reader: BinaryReader) -> Target: ...
def to_json_target(value: Target) -> Json: ...
def from_json_target(value: Json) -> Target: ...

@dataclass(frozen=True, slots=True)
class Entrypoint:
    """User callable launched by executable Destack products."""

    # module containing the exported entrypoint function
    module: str
    # exported function invoked after runtime bootstrap
    export: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Entrypoint: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Entrypoint: ...

def encode_entrypoint(writer: BinaryWriter, value: Entrypoint) -> None: ...
def decode_entrypoint(reader: BinaryReader) -> Entrypoint: ...
def to_json_entrypoint(value: Entrypoint) -> Json: ...
def from_json_entrypoint(value: Json) -> Entrypoint: ...

__all__ = [
    "Target",
    "encode_target",
    "decode_target",
    "to_json_target",
    "from_json_target",
    "Entrypoint",
    "encode_entrypoint",
    "decode_entrypoint",
    "to_json_entrypoint",
    "from_json_entrypoint",
]
