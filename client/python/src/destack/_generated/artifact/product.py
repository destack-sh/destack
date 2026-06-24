# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.artifact.core.target
import destack._generated.source.file.model.target


@dataclass(frozen=True, slots=True)
class ProductTarget:
    """One linked target assembled into a product."""

    # the configured product target name
    name: str
    # the repository target assembled into this product
    target: destack._generated.source.file.model.target.TargetId
    # the runtime contract this target expects
    runtime: destack._generated.artifact.core.target.Runtime
    # the host environment this target expects
    host: destack._generated.artifact.core.target.Host
    # the platform this target expects
    platform: destack._generated.artifact.core.target.Platform
    # whether this product target includes its toolchain build payload
    includes_build: bool
    # whether this product target includes its linked bundle
    includes_bundle: bool
    # whether this product target includes its executable program
    includes_program: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_product_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProductTarget:
        """Decode one ProductTarget."""
        return decode_product_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_product_target(self)

    @classmethod
    def from_json(cls, value: Json) -> ProductTarget:
        """Return one ProductTarget from one JSON value."""
        return from_json_product_target(value)


def encode_product_target(writer: BinaryWriter, value: ProductTarget) -> None:
    """Encode one ProductTarget."""
    writer.write_string(value.name)
    destack._generated.source.file.model.target.encode_target_id(writer, value.target)
    destack._generated.artifact.core.target.encode_runtime(writer, value.runtime)
    destack._generated.artifact.core.target.encode_host(writer, value.host)
    destack._generated.artifact.core.target.encode_platform(writer, value.platform)
    writer.write_bool(value.includes_build)
    writer.write_bool(value.includes_bundle)
    writer.write_bool(value.includes_program)


def decode_product_target(reader: BinaryReader) -> ProductTarget:
    """Decode one ProductTarget."""
    name = reader.read_string()
    target = destack._generated.source.file.model.target.decode_target_id(reader)
    runtime = destack._generated.artifact.core.target.decode_runtime(reader)
    host = destack._generated.artifact.core.target.decode_host(reader)
    platform = destack._generated.artifact.core.target.decode_platform(reader)
    includes_build = reader.read_bool()
    includes_bundle = reader.read_bool()
    includes_program = reader.read_bool()

    return ProductTarget(
        name=name,
        target=target,
        runtime=runtime,
        host=host,
        platform=platform,
        includes_build=includes_build,
        includes_bundle=includes_bundle,
        includes_program=includes_program,
    )


def to_json_product_target(value: ProductTarget) -> Json:
    """Return one JSON value for one ProductTarget."""
    return {
        "name": value.name,
        "target": destack._generated.source.file.model.target.to_json_target_id(
            value.target
        ),
        "runtime": destack._generated.artifact.core.target.to_json_runtime(
            value.runtime
        ),
        "host": destack._generated.artifact.core.target.to_json_host(value.host),
        "platform": destack._generated.artifact.core.target.to_json_platform(
            value.platform
        ),
        "includesBuild": value.includes_build,
        "includesBundle": value.includes_bundle,
        "includesProgram": value.includes_program,
    }


def from_json_product_target(value: Json) -> ProductTarget:
    """Return one ProductTarget from one JSON value."""
    object_ = json_object(value)

    return ProductTarget(
        name=json_string(json_field(object_, "name")),
        target=destack._generated.source.file.model.target.from_json_target_id(
            json_field(object_, "target")
        ),
        runtime=destack._generated.artifact.core.target.from_json_runtime(
            json_field(object_, "runtime")
        ),
        host=destack._generated.artifact.core.target.from_json_host(
            json_field(object_, "host")
        ),
        platform=destack._generated.artifact.core.target.from_json_platform(
            json_field(object_, "platform")
        ),
        includes_build=json_bool(json_field(object_, "includesBuild")),
        includes_bundle=json_bool(json_field(object_, "includesBundle")),
        includes_program=json_bool(json_field(object_, "includesProgram")),
    )


@dataclass(frozen=True, slots=True)
class Product:
    """One linked product assembled from one or more target artifacts."""

    # the configured product name
    name: str
    # the linked targets keyed by configured product target name
    targets: Mapping[str, ProductTarget]

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
    writer.write_string(value.name)
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
        encode_product_target(writer, entry_value_targets_0[1])


def decode_product(reader: BinaryReader) -> Product:
    """Decode one Product."""
    name = reader.read_string()
    targets = {
        reader.read_string(): decode_product_target(reader)
        for _ in range(reader.read_number())
    }

    return Product(
        name=name,
        targets=targets,
    )


def to_json_product(value: Product) -> Json:
    """Return one JSON value for one Product."""
    return {
        "name": value.name,
        "targets": {
            key_0: to_json_product_target(item_0)
            for key_0, item_0 in value.targets.items()
        },
    }


def from_json_product(value: Json) -> Product:
    """Return one Product from one JSON value."""
    object_ = json_object(value)

    return Product(
        name=json_string(json_field(object_, "name")),
        targets={
            key_0: from_json_product_target(item_0)
            for key_0, item_0 in json_object(json_field(object_, "targets")).items()
        },
    )


__all__ = [
    "ProductTarget",
    "encode_product_target",
    "decode_product_target",
    "to_json_product_target",
    "from_json_product_target",
    "Product",
    "encode_product",
    "decode_product",
    "to_json_product",
    "from_json_product",
]
