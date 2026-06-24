# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.stage

@dataclass(frozen=True, slots=True)
class TargetConditionSet:
    """Target contribution to the active source graph condition set."""

    # explicit profile name for this target
    profile: str | None
    # release stage for this target
    stage: destack._generated.repository.config.stage.Stage | None
    # active source graph modes for this target
    modes: Sequence[str]
    # active source graph roles for this target
    roles: Sequence[str]
    # active optional features for this target
    features: Sequence[str]
    # active source graph tags for this target
    tags: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetConditionSet: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetConditionSet: ...

def encode_target_condition_set(
    writer: BinaryWriter, value: TargetConditionSet
) -> None: ...
def decode_target_condition_set(reader: BinaryReader) -> TargetConditionSet: ...
def to_json_target_condition_set(value: TargetConditionSet) -> Json: ...
def from_json_target_condition_set(value: Json) -> TargetConditionSet: ...

__all__ = [
    "TargetConditionSet",
    "encode_target_condition_set",
    "decode_target_condition_set",
    "to_json_target_condition_set",
    "from_json_target_condition_set",
]
