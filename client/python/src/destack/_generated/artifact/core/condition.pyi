# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.target

@dataclass(frozen=True, slots=True)
class ConditionSet:
    """Active source graph and runtime selection conditions."""

    # active source graph modes
    modes: Sequence[str]
    # active source graph roles
    roles: Sequence[str]
    # active optional features
    features: Sequence[str]
    # active source graph tags
    tags: Sequence[str]
    # active build target
    target: str | None
    # active product
    product: str | None
    # active package release stage
    stage: str | None
    # active target platform
    platform: destack._generated.artifact.core.target.Platform | None
    # active host environment
    host: destack._generated.artifact.core.target.Host | None
    # active runtime
    runtime: destack._generated.artifact.core.target.Runtime | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionSet: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionSet: ...

def encode_condition_set(writer: BinaryWriter, value: ConditionSet) -> None: ...
def decode_condition_set(reader: BinaryReader) -> ConditionSet: ...
def to_json_condition_set(value: ConditionSet) -> Json: ...
def from_json_condition_set(value: Json) -> ConditionSet: ...

__all__ = [
    "ConditionSet",
    "encode_condition_set",
    "decode_condition_set",
    "to_json_condition_set",
    "from_json_condition_set",
]
