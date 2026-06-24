# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_condition_set(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetConditionSet:
        """Decode one TargetConditionSet."""
        return decode_target_condition_set(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_condition_set(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetConditionSet:
        """Return one TargetConditionSet from one JSON value."""
        return from_json_target_condition_set(value)


def encode_target_condition_set(
    writer: BinaryWriter, value: TargetConditionSet
) -> None:
    """Encode one TargetConditionSet."""
    if value.profile is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.profile)
    if value.stage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.stage.encode_stage(writer, value.stage)
    writer.write_unsigned(len(value.modes))
    for item_value_modes_0 in value.modes:
        writer.write_string(item_value_modes_0)
    writer.write_unsigned(len(value.roles))
    for item_value_roles_0 in value.roles:
        writer.write_string(item_value_roles_0)
    writer.write_unsigned(len(value.features))
    for item_value_features_0 in value.features:
        writer.write_string(item_value_features_0)
    writer.write_unsigned(len(value.tags))
    for item_value_tags_0 in value.tags:
        writer.write_string(item_value_tags_0)


def decode_target_condition_set(reader: BinaryReader) -> TargetConditionSet:
    """Decode one TargetConditionSet."""
    profile = reader.read_option(lambda: reader.read_string())
    stage = reader.read_option(
        lambda: destack._generated.repository.config.stage.decode_stage(reader)
    )
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]

    return TargetConditionSet(
        profile=profile,
        stage=stage,
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
    )


def to_json_target_condition_set(value: TargetConditionSet) -> Json:
    """Return one JSON value for one TargetConditionSet."""
    return {
        **({} if value.profile is None else {"profile": value.profile}),
        **(
            {}
            if value.stage is None
            else {
                "stage": destack._generated.repository.config.stage.to_json_stage(
                    value.stage
                )
            }
        ),
        "modes": [item_0 for item_0 in value.modes],
        "roles": [item_0 for item_0 in value.roles],
        "features": [item_0 for item_0 in value.features],
        "tags": [item_0 for item_0 in value.tags],
    }


def from_json_target_condition_set(value: Json) -> TargetConditionSet:
    """Return one TargetConditionSet from one JSON value."""
    object_ = json_object(value)

    return TargetConditionSet(
        profile=json_optional(object_, "profile", lambda value: json_string(value)),
        stage=json_optional(
            object_,
            "stage",
            lambda value: destack._generated.repository.config.stage.from_json_stage(
                value
            ),
        ),
        modes=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "modes"))
        ],
        roles=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "roles"))
        ],
        features=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "features"))
        ],
        tags=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "tags"))
        ],
    )


__all__ = [
    "TargetConditionSet",
    "encode_target_condition_set",
    "decode_target_condition_set",
    "to_json_target_condition_set",
    "from_json_target_condition_set",
]
