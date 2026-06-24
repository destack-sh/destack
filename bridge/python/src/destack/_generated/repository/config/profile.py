# generated bridge target, do not edit

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
import destack._generated.repository.config.compiler
import destack._generated.repository.config.stage


@dataclass(frozen=True, slots=True)
class ProfileOptions:
    """Normalized profile options."""

    # release stage for this profile
    stage: destack._generated.repository.config.stage.Stage | None
    # target platform / operating system for this profile
    platform: destack._generated.artifact.core.target.Platform | None
    # target host environment for this profile
    host: destack._generated.artifact.core.target.Host | None
    # comptime environment whitelist
    comptime_env: Sequence[str] | None
    # active source graph modes for this profile
    modes: Sequence[str]
    # active source graph roles for this profile
    roles: Sequence[str]
    # active source graph features for this profile
    features: Sequence[str]
    # active source graph tags for this profile
    tags: Sequence[str]
    # default tree tag builder provider
    tree: str | None
    # global provider modules for this profile
    globals: Sequence[str]
    # well-known derives automatically considered in this profile
    derive: Sequence[destack._generated.repository.config.compiler.Derive]
    # static semantic restrictions for this profile
    restrictions: destack._generated.repository.config.compiler.CompilerRestrictions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProfileOptions:
        """Decode one ProfileOptions."""
        return decode_profile_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_options(self)

    @classmethod
    def from_json(cls, value: Json) -> ProfileOptions:
        """Return one ProfileOptions from one JSON value."""
        return from_json_profile_options(value)


def encode_profile_options(writer: BinaryWriter, value: ProfileOptions) -> None:
    """Encode one ProfileOptions."""
    if value.stage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.stage.encode_stage(writer, value.stage)
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
    if value.comptime_env is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.comptime_env))
        for item_value_comptime_env_1 in value.comptime_env:
            writer.write_string(item_value_comptime_env_1)
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
    if value.tree is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tree)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        writer.write_string(item_value_globals_0)
    writer.write_unsigned(len(value.derive))
    for item_value_derive_0 in value.derive:
        destack._generated.repository.config.compiler.encode_derive(
            writer, item_value_derive_0
        )
    destack._generated.repository.config.compiler.encode_compiler_restrictions(
        writer, value.restrictions
    )


def decode_profile_options(reader: BinaryReader) -> ProfileOptions:
    """Decode one ProfileOptions."""
    stage = reader.read_option(
        lambda: destack._generated.repository.config.stage.decode_stage(reader)
    )
    platform = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_platform(reader)
    )
    host = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_host(reader)
    )
    comptime_env = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]
    tree = reader.read_option(lambda: reader.read_string())
    globals = [reader.read_string() for _ in range(reader.read_number())]
    derive = [
        destack._generated.repository.config.compiler.decode_derive(reader)
        for _ in range(reader.read_number())
    ]
    restrictions = (
        destack._generated.repository.config.compiler.decode_compiler_restrictions(
            reader
        )
    )

    return ProfileOptions(
        stage=stage,
        platform=platform,
        host=host,
        comptime_env=comptime_env,
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        tree=tree,
        globals=globals,
        derive=derive,
        restrictions=restrictions,
    )


def to_json_profile_options(value: ProfileOptions) -> Json:
    """Return one JSON value for one ProfileOptions."""
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
            if value.comptime_env is None
            else {"comptimeEnv": [item_0 for item_0 in value.comptime_env]}
        ),
        "modes": [item_0 for item_0 in value.modes],
        "roles": [item_0 for item_0 in value.roles],
        "features": [item_0 for item_0 in value.features],
        "tags": [item_0 for item_0 in value.tags],
        **({} if value.tree is None else {"tree": value.tree}),
        "globals": [item_0 for item_0 in value.globals],
        "derive": [
            destack._generated.repository.config.compiler.to_json_derive(item_0)
            for item_0 in value.derive
        ],
        "restrictions": destack._generated.repository.config.compiler.to_json_compiler_restrictions(
            value.restrictions
        ),
    }


def from_json_profile_options(value: Json) -> ProfileOptions:
    """Return one ProfileOptions from one JSON value."""
    object_ = json_object(value)

    return ProfileOptions(
        stage=json_optional(
            object_,
            "stage",
            lambda value: destack._generated.repository.config.stage.from_json_stage(
                value
            ),
        ),
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
        comptime_env=json_optional(
            object_,
            "comptimeEnv",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
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
        tree=json_optional(object_, "tree", lambda value: json_string(value)),
        globals=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "globals"))
        ],
        derive=[
            destack._generated.repository.config.compiler.from_json_derive(item_0)
            for item_0 in json_array(json_field(object_, "derive"))
        ],
        restrictions=destack._generated.repository.config.compiler.from_json_compiler_restrictions(
            json_field(object_, "restrictions")
        ),
    )


__all__ = [
    "ProfileOptions",
    "encode_profile_options",
    "decode_profile_options",
    "to_json_profile_options",
    "from_json_profile_options",
]
