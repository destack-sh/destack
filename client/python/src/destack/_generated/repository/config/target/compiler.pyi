# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.compiler

@dataclass(frozen=True, slots=True)
class TargetCompilerOptions:
    """Target compiler behavior options."""

    # target tree tag builder override
    tree: str | None
    # target well-known derives
    derive: Sequence[destack._generated.repository.config.compiler.Derive]
    # static semantic restrictions for this target
    restrictions: destack._generated.repository.config.compiler.CompilerRestrictions
    # optimization level
    optimize: OptimizeLevel
    # generated safety check policies
    checks: CheckPolicySet

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetCompilerOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetCompilerOptions: ...

def encode_target_compiler_options(
    writer: BinaryWriter, value: TargetCompilerOptions
) -> None: ...
def decode_target_compiler_options(reader: BinaryReader) -> TargetCompilerOptions: ...
def to_json_target_compiler_options(value: TargetCompilerOptions) -> Json: ...
def from_json_target_compiler_options(value: Json) -> TargetCompilerOptions: ...

"""Optimization level for builds."""
OptimizeLevel: typing.TypeAlias = (
    typing.Literal["o0"]
    | typing.Literal["o1"]
    | typing.Literal["o2"]
    | typing.Literal["o3"]
    | typing.Literal["o4"]
)

def encode_optimize_level(writer: BinaryWriter, value: OptimizeLevel) -> None: ...
def decode_optimize_level(reader: BinaryReader) -> OptimizeLevel: ...
def to_json_optimize_level(value: OptimizeLevel) -> Json: ...
def from_json_optimize_level(value: Json) -> OptimizeLevel: ...

@dataclass(frozen=True, slots=True)
class CheckPolicySet:
    """Generated check policy set."""

    # integer overflow check policy
    overflow: CheckPolicy
    # bounds check policy for array and slice accesses
    bounds: CheckPolicy
    # null check policy for reference operations
    null: CheckPolicy
    # division check policy for divide and remainder operations
    division: CheckPolicy
    # shift range check policy
    shift: CheckPolicy
    # check failure behavior
    failure: CheckFailurePolicy

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CheckPolicySet: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CheckPolicySet: ...

def encode_check_policy_set(writer: BinaryWriter, value: CheckPolicySet) -> None: ...
def decode_check_policy_set(reader: BinaryReader) -> CheckPolicySet: ...
def to_json_check_policy_set(value: CheckPolicySet) -> Json: ...
def from_json_check_policy_set(value: Json) -> CheckPolicySet: ...

"""Generated safety check policy."""
CheckPolicy: typing.TypeAlias = (
    typing.Literal["always"] | typing.Literal["debug"] | typing.Literal["never"]
)

def encode_check_policy(writer: BinaryWriter, value: CheckPolicy) -> None: ...
def decode_check_policy(reader: BinaryReader) -> CheckPolicy: ...
def to_json_check_policy(value: CheckPolicy) -> Json: ...
def from_json_check_policy(value: Json) -> CheckPolicy: ...

"""Check failure behavior."""
CheckFailurePolicy: typing.TypeAlias = typing.Literal["panic"] | typing.Literal["abort"]

def encode_check_failure_policy(
    writer: BinaryWriter, value: CheckFailurePolicy
) -> None: ...
def decode_check_failure_policy(reader: BinaryReader) -> CheckFailurePolicy: ...
def to_json_check_failure_policy(value: CheckFailurePolicy) -> Json: ...
def from_json_check_failure_policy(value: Json) -> CheckFailurePolicy: ...

__all__ = [
    "TargetCompilerOptions",
    "encode_target_compiler_options",
    "decode_target_compiler_options",
    "to_json_target_compiler_options",
    "from_json_target_compiler_options",
    "OptimizeLevel",
    "encode_optimize_level",
    "decode_optimize_level",
    "to_json_optimize_level",
    "from_json_optimize_level",
    "CheckPolicySet",
    "encode_check_policy_set",
    "decode_check_policy_set",
    "to_json_check_policy_set",
    "from_json_check_policy_set",
    "CheckPolicy",
    "encode_check_policy",
    "decode_check_policy",
    "to_json_check_policy",
    "from_json_check_policy",
    "CheckFailurePolicy",
    "encode_check_failure_policy",
    "decode_check_failure_policy",
    "to_json_check_failure_policy",
    "from_json_check_failure_policy",
]
