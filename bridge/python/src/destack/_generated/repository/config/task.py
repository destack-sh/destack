# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

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


@dataclass(frozen=True, slots=True)
class Task:
    """Named toolchain or shell invocation."""

    # destack command name to run
    run: str | None
    # explicit shell command to execute
    exec: str | None
    # build target selected for this task
    target: str | None
    # product selected for this task
    product: str | None
    # profile selected for this task
    profile: str | None
    # active source graph modes added by this task
    modes: Sequence[str]
    # active source graph roles added by this task
    roles: Sequence[str]
    # active source graph features added by this task
    features: Sequence[str]
    # active source graph tags added by this task
    tags: Sequence[str]
    # environment variables passed to this task
    env: Mapping[str, str]
    # structured command arguments
    arguments: Mapping[str, typing.Any]
    # tasks that must complete before this task
    depends_on: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Task:
        """Decode one Task."""
        return decode_task(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task(self)

    @classmethod
    def from_json(cls, value: Json) -> Task:
        """Return one Task from one JSON value."""
        return from_json_task(value)


def encode_task(writer: BinaryWriter, value: Task) -> None:
    """Encode one Task."""
    if value.run is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.run)
    if value.exec is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.exec)
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
    if value.profile is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.profile)
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
    entries_value_env_0 = []
    for key_value_env_0, item_value_env_0 in value.env.items():

        def write_key_value_env_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_env_0)

        key_bytes = nested_bytes(write_key_value_env_0)
        entries_value_env_0.append((key_value_env_0, item_value_env_0, key_bytes))
    entries_value_env_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_env_0))
    for entry_value_env_0 in entries_value_env_0:
        writer.write_string(entry_value_env_0[0])
        writer.write_string(entry_value_env_0[1])
    entries_value_arguments_0 = []
    for key_value_arguments_0, item_value_arguments_0 in value.arguments.items():

        def write_key_value_arguments_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_arguments_0)

        key_bytes = nested_bytes(write_key_value_arguments_0)
        entries_value_arguments_0.append(
            (key_value_arguments_0, item_value_arguments_0, key_bytes)
        )
    entries_value_arguments_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_arguments_0))
    for entry_value_arguments_0 in entries_value_arguments_0:
        writer.write_string(entry_value_arguments_0[0])
        writer.write_json(entry_value_arguments_0[1])
    writer.write_unsigned(len(value.depends_on))
    for item_value_depends_on_0 in value.depends_on:
        writer.write_string(item_value_depends_on_0)


def decode_task(reader: BinaryReader) -> Task:
    """Decode one Task."""
    run = reader.read_option(lambda: reader.read_string())
    exec = reader.read_option(lambda: reader.read_string())
    target = reader.read_option(lambda: reader.read_string())
    product = reader.read_option(lambda: reader.read_string())
    profile = reader.read_option(lambda: reader.read_string())
    modes = [reader.read_string() for _ in range(reader.read_number())]
    roles = [reader.read_string() for _ in range(reader.read_number())]
    features = [reader.read_string() for _ in range(reader.read_number())]
    tags = [reader.read_string() for _ in range(reader.read_number())]
    env = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }
    arguments = {
        reader.read_string(): reader.read_json() for _ in range(reader.read_number())
    }
    depends_on = [reader.read_string() for _ in range(reader.read_number())]

    return Task(
        run=run,
        exec=exec,
        target=target,
        product=product,
        profile=profile,
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        env=env,
        arguments=arguments,
        depends_on=depends_on,
    )


def to_json_task(value: Task) -> Json:
    """Return one JSON value for one Task."""
    return {
        **({} if value.run is None else {"run": value.run}),
        **({} if value.exec is None else {"exec": value.exec}),
        **({} if value.target is None else {"target": value.target}),
        **({} if value.product is None else {"product": value.product}),
        **({} if value.profile is None else {"profile": value.profile}),
        "modes": [item_0 for item_0 in value.modes],
        "roles": [item_0 for item_0 in value.roles],
        "features": [item_0 for item_0 in value.features],
        "tags": [item_0 for item_0 in value.tags],
        "env": {key_0: item_0 for key_0, item_0 in value.env.items()},
        "arguments": {key_0: item_0 for key_0, item_0 in value.arguments.items()},
        "dependsOn": [item_0 for item_0 in value.depends_on],
    }


def from_json_task(value: Json) -> Task:
    """Return one Task from one JSON value."""
    object_ = json_object(value)

    return Task(
        run=json_optional(object_, "run", lambda value: json_string(value)),
        exec=json_optional(object_, "exec", lambda value: json_string(value)),
        target=json_optional(object_, "target", lambda value: json_string(value)),
        product=json_optional(object_, "product", lambda value: json_string(value)),
        profile=json_optional(object_, "profile", lambda value: json_string(value)),
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
        env={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "env")).items()
        },
        arguments={
            key_0: item_0
            for key_0, item_0 in json_object(json_field(object_, "arguments")).items()
        },
        depends_on=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "dependsOn"))
        ],
    )


__all__ = [
    "Task",
    "encode_task",
    "decode_task",
    "to_json_task",
    "from_json_task",
]
