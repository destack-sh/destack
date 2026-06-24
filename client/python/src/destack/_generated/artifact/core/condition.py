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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_set(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionSet:
        """Decode one ConditionSet."""
        return decode_condition_set(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_set(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionSet:
        """Return one ConditionSet from one JSON value."""
        return from_json_condition_set(value)


def encode_condition_set(writer: BinaryWriter, value: ConditionSet) -> None:
    """Encode one ConditionSet."""
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
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)
    if value.product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.product)
    if value.stage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.stage)
    if value.platform is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_platform(writer, value.platform)
    if value.host is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_host(writer, value.host)
    if value.runtime is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_runtime(writer, value.runtime)


def decode_condition_set(reader: BinaryReader) -> ConditionSet:
    """Decode one ConditionSet."""
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]
    target = reader.read_option(lambda: reader.read_string())
    product = reader.read_option(lambda: reader.read_string())
    stage = reader.read_option(lambda: reader.read_string())
    platform = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_platform(reader)
    )
    host = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_host(reader)
    )
    runtime = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_runtime(reader)
    )

    return ConditionSet(
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        target=target,
        product=product,
        stage=stage,
        platform=platform,
        host=host,
        runtime=runtime,
    )


def to_json_condition_set(value: ConditionSet) -> Json:
    """Return one JSON value for one ConditionSet."""
    return {
        "modes": [item_0 for item_0 in value.modes],
        "roles": [item_0 for item_0 in value.roles],
        "features": [item_0 for item_0 in value.features],
        "tags": [item_0 for item_0 in value.tags],
        **({} if value.target is None else {"target": value.target}),
        **({} if value.product is None else {"product": value.product}),
        **({} if value.stage is None else {"stage": value.stage}),
        **(
            {}
            if value.platform is None
            else {
                "platform": destack._generated.artifact.core.target.to_json_platform(
                    value.platform
                )
            }
        ),
        **(
            {}
            if value.host is None
            else {
                "host": destack._generated.artifact.core.target.to_json_host(value.host)
            }
        ),
        **(
            {}
            if value.runtime is None
            else {
                "runtime": destack._generated.artifact.core.target.to_json_runtime(
                    value.runtime
                )
            }
        ),
    }


def from_json_condition_set(value: Json) -> ConditionSet:
    """Return one ConditionSet from one JSON value."""
    object_ = json_object(value)

    return ConditionSet(
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
        target=json_optional(object_, "target", lambda value: json_string(value)),
        product=json_optional(object_, "product", lambda value: json_string(value)),
        stage=json_optional(object_, "stage", lambda value: json_string(value)),
        platform=json_optional(
            object_,
            "platform",
            lambda value: destack._generated.artifact.core.target.from_json_platform(
                value
            ),
        ),
        host=json_optional(
            object_,
            "host",
            lambda value: destack._generated.artifact.core.target.from_json_host(value),
        ),
        runtime=json_optional(
            object_,
            "runtime",
            lambda value: destack._generated.artifact.core.target.from_json_runtime(
                value
            ),
        ),
    )


__all__ = [
    "ConditionSet",
    "encode_condition_set",
    "decode_condition_set",
    "to_json_condition_set",
    "from_json_condition_set",
]
