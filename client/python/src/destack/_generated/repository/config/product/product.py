# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

import destack._generated.repository.config.policy
import destack._generated.repository.config.product.app
import destack._generated.repository.config.stage


@dataclass(frozen=True, slots=True)
class Product:
    """Product assembled from one or more build targets."""

    # release stage for this product
    stage: destack._generated.repository.config.stage.Stage | None
    # active source graph modes for this product
    modes: Sequence[str]
    # active source graph roles for this product
    roles: Sequence[str]
    # active source graph features for this product
    features: Sequence[str]
    # active source graph tags for this product
    tags: Sequence[str]
    # target names keyed by product role
    targets: Mapping[str, str]
    # app declaration used for host integration
    app: destack._generated.repository.config.product.app.App
    # product policy declarations and rules
    policy: destack._generated.repository.config.policy.Policy

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_product(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Product:
        """Decode one Product."""
        return decode_product(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_product(self)

    @classmethod
    def from_json(cls, value: Json) -> Product:
        """Return one Product from one JSON value."""
        return from_json_product(value)


def encode_product(writer: BinaryWriter, value: Product) -> None:
    """Encode one Product."""
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
    entries_value_targets_0 = []
    for key_value_targets_0, item_value_targets_0 in value.targets.items():

        def write_key_value_targets_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_targets_0)

        key_bytes = nested_bytes(write_key_value_targets_0)
        entries_value_targets_0.append(
            (key_value_targets_0, item_value_targets_0, key_bytes)
        )
    entries_value_targets_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_targets_0))
    for entry_value_targets_0 in entries_value_targets_0:
        writer.write_string(entry_value_targets_0[0])
        writer.write_string(entry_value_targets_0[1])
    destack._generated.repository.config.product.app.encode_app(writer, value.app)
    destack._generated.repository.config.policy.encode_policy(writer, value.policy)


def decode_product(reader: BinaryReader) -> Product:
    """Decode one Product."""
    stage = reader.read_option(
        lambda: destack._generated.repository.config.stage.decode_stage(reader)
    )
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]
    targets = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }
    app = destack._generated.repository.config.product.app.decode_app(reader)
    policy = destack._generated.repository.config.policy.decode_policy(reader)

    return Product(
        stage=stage,
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        targets=targets,
        app=app,
        policy=policy,
    )


def to_json_product(value: Product) -> Json:
    """Return one JSON value for one Product."""
    return {
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
        "targets": {key_0: item_0 for key_0, item_0 in value.targets.items()},
        "app": destack._generated.repository.config.product.app.to_json_app(value.app),
        "policy": destack._generated.repository.config.policy.to_json_policy(
            value.policy
        ),
    }


def from_json_product(value: Json) -> Product:
    """Return one Product from one JSON value."""
    object_ = json_object(value)

    return Product(
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
        targets={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "targets")).items()
        },
        app=destack._generated.repository.config.product.app.from_json_app(
            json_field(object_, "app")
        ),
        policy=destack._generated.repository.config.policy.from_json_policy(
            json_field(object_, "policy")
        ),
    )


__all__ = [
    "Product",
    "encode_product",
    "decode_product",
    "to_json_product",
    "from_json_product",
]
