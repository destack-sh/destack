# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.repository.config.dependency


@dataclass(frozen=True, slots=True)
class ConditionRefName:
    """Named condition alias or `axis:name` reference."""

    name: str
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_ref(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_ref(self)


@dataclass(frozen=True, slots=True)
class ConditionRefPredicate:
    """Inline condition gate."""

    predicate: ConditionPredicate
    kind: typing.Literal["predicate"] = "predicate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_ref(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_ref(self)


"""Source-level reference to one active condition predicate."""
ConditionRef: typing.TypeAlias = ConditionRefName | ConditionRefPredicate


def encode_condition_ref(writer: BinaryWriter, value: ConditionRef) -> None:
    """Encode one ConditionRef."""
    if value.kind == "name":
        writer.write_unsigned(0)
        writer.write_string(value.name)
    elif value.kind == "predicate":
        writer.write_unsigned(1)
        encode_condition_predicate(writer, value.predicate)
    else:
        raise SerdeError("unknown enum variant")


def decode_condition_ref(reader: BinaryReader) -> ConditionRef:
    """Decode one ConditionRef."""
    variant = reader.read_number()

    if variant == 0:
        name = reader.read_string()

        return ConditionRefName(name=name)
    elif variant == 1:
        predicate = decode_condition_predicate(reader)

        return ConditionRefPredicate(predicate=predicate)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_condition_ref(value: ConditionRef) -> Json:
    """Return one JSON value for one ConditionRef."""
    if value.kind == "name":
        return {
            "kind": "name",
            "name": value.name,
        }
    elif value.kind == "predicate":
        return {
            "kind": "predicate",
            "predicate": to_json_condition_predicate(value.predicate),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_condition_ref(value: Json) -> ConditionRef:
    """Return one ConditionRef from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "name":
        return ConditionRefName(name=json_string(json_field(object_, "name")))
    elif kind == "predicate":
        return ConditionRefPredicate(
            predicate=from_json_condition_predicate(json_field(object_, "predicate"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ConditionPredicate:
    """Declared condition predicate before named references are resolved."""

    # active mode selector
    mode: ConditionSelector | None
    # active role selector
    role: ConditionSelector | None
    # active feature selector
    feature: ConditionSelector | None
    # active tag selector
    tag: ConditionSelector | None
    # active build target selector
    target: ConditionSelector | None
    # active product selector
    product: ConditionSelector | None
    # active package release stage selector
    stage: ConditionSelector | None
    # active target platform selector
    platform: ConditionSelector | None
    # active host environment selector
    host: ConditionSelector | None
    # active runtime selector
    runtime: ConditionSelector | None
    # predicates that must all match
    all: Sequence[ConditionRef] | None
    # predicates where at least one must match
    any: Sequence[ConditionRef] | None
    # predicate that must not match
    not_: ConditionRef | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_predicate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionPredicate:
        """Decode one ConditionPredicate."""
        return decode_condition_predicate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_predicate(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionPredicate:
        """Return one ConditionPredicate from one JSON value."""
        return from_json_condition_predicate(value)


def encode_condition_predicate(writer: BinaryWriter, value: ConditionPredicate) -> None:
    """Encode one ConditionPredicate."""
    if value.mode is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.mode)
    if value.role is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.role)
    if value.feature is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.feature)
    if value.tag is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.tag)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.target)
    if value.product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.product)
    if value.stage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.stage)
    if value.platform is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.platform)
    if value.host is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.host)
    if value.runtime is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_selector(writer, value.runtime)
    if value.all is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.all))
        for item_value_all_1 in value.all:
            encode_condition_ref(writer, item_value_all_1)
    if value.any is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.any))
        for item_value_any_1 in value.any:
            encode_condition_ref(writer, item_value_any_1)
    if value.not_ is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_condition_ref(writer, value.not_)


def decode_condition_predicate(reader: BinaryReader) -> ConditionPredicate:
    """Decode one ConditionPredicate."""
    mode = reader.read_option(lambda: decode_condition_selector(reader))
    role = reader.read_option(lambda: decode_condition_selector(reader))
    feature = reader.read_option(lambda: decode_condition_selector(reader))
    tag = reader.read_option(lambda: decode_condition_selector(reader))
    target = reader.read_option(lambda: decode_condition_selector(reader))
    product = reader.read_option(lambda: decode_condition_selector(reader))
    stage = reader.read_option(lambda: decode_condition_selector(reader))
    platform = reader.read_option(lambda: decode_condition_selector(reader))
    host = reader.read_option(lambda: decode_condition_selector(reader))
    runtime = reader.read_option(lambda: decode_condition_selector(reader))
    all = reader.read_option(
        lambda: [decode_condition_ref(reader) for _ in range(reader.read_number())]
    )
    any = reader.read_option(
        lambda: [decode_condition_ref(reader) for _ in range(reader.read_number())]
    )
    not_ = reader.read_option(lambda: decode_condition_ref(reader))

    return ConditionPredicate(
        mode=mode,
        role=role,
        feature=feature,
        tag=tag,
        target=target,
        product=product,
        stage=stage,
        platform=platform,
        host=host,
        runtime=runtime,
        all=all,
        any=any,
        not_=not_,
    )


def to_json_condition_predicate(value: ConditionPredicate) -> Json:
    """Return one JSON value for one ConditionPredicate."""
    return {
        **(
            {}
            if value.mode is None
            else {"mode": to_json_condition_selector(value.mode)}
        ),
        **(
            {}
            if value.role is None
            else {"role": to_json_condition_selector(value.role)}
        ),
        **(
            {}
            if value.feature is None
            else {"feature": to_json_condition_selector(value.feature)}
        ),
        **({} if value.tag is None else {"tag": to_json_condition_selector(value.tag)}),
        **(
            {}
            if value.target is None
            else {"target": to_json_condition_selector(value.target)}
        ),
        **(
            {}
            if value.product is None
            else {"product": to_json_condition_selector(value.product)}
        ),
        **(
            {}
            if value.stage is None
            else {"stage": to_json_condition_selector(value.stage)}
        ),
        **(
            {}
            if value.platform is None
            else {"platform": to_json_condition_selector(value.platform)}
        ),
        **(
            {}
            if value.host is None
            else {"host": to_json_condition_selector(value.host)}
        ),
        **(
            {}
            if value.runtime is None
            else {"runtime": to_json_condition_selector(value.runtime)}
        ),
        **(
            {}
            if value.all is None
            else {"all": [to_json_condition_ref(item_0) for item_0 in value.all]}
        ),
        **(
            {}
            if value.any is None
            else {"any": [to_json_condition_ref(item_0) for item_0 in value.any]}
        ),
        **({} if value.not_ is None else {"not": to_json_condition_ref(value.not_)}),
    }


def from_json_condition_predicate(value: Json) -> ConditionPredicate:
    """Return one ConditionPredicate from one JSON value."""
    object_ = json_object(value)

    return ConditionPredicate(
        mode=json_optional(
            object_, "mode", lambda value: from_json_condition_selector(value)
        ),
        role=json_optional(
            object_, "role", lambda value: from_json_condition_selector(value)
        ),
        feature=json_optional(
            object_, "feature", lambda value: from_json_condition_selector(value)
        ),
        tag=json_optional(
            object_, "tag", lambda value: from_json_condition_selector(value)
        ),
        target=json_optional(
            object_, "target", lambda value: from_json_condition_selector(value)
        ),
        product=json_optional(
            object_, "product", lambda value: from_json_condition_selector(value)
        ),
        stage=json_optional(
            object_, "stage", lambda value: from_json_condition_selector(value)
        ),
        platform=json_optional(
            object_, "platform", lambda value: from_json_condition_selector(value)
        ),
        host=json_optional(
            object_, "host", lambda value: from_json_condition_selector(value)
        ),
        runtime=json_optional(
            object_, "runtime", lambda value: from_json_condition_selector(value)
        ),
        all=json_optional(
            object_,
            "all",
            lambda value: [
                from_json_condition_ref(item_0) for item_0 in json_array(value)
            ],
        ),
        any=json_optional(
            object_,
            "any",
            lambda value: [
                from_json_condition_ref(item_0) for item_0 in json_array(value)
            ],
        ),
        not_=json_optional(
            object_, "not", lambda value: from_json_condition_ref(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class ConditionSelector:
    """Selector over one active condition axis."""

    # condition names or glob patterns
    patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_selector(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionSelector:
        """Decode one ConditionSelector."""
        return decode_condition_selector(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_selector(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionSelector:
        """Return one ConditionSelector from one JSON value."""
        return from_json_condition_selector(value)


def encode_condition_selector(writer: BinaryWriter, value: ConditionSelector) -> None:
    """Encode one ConditionSelector."""
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        writer.write_string(item_value_patterns_0)


def decode_condition_selector(reader: BinaryReader) -> ConditionSelector:
    """Decode one ConditionSelector."""
    patterns = [reader.read_string() for _ in range(reader.read_number())]

    return ConditionSelector(
        patterns=patterns,
    )


def to_json_condition_selector(value: ConditionSelector) -> Json:
    """Return one JSON value for one ConditionSelector."""
    return {
        "patterns": [item_0 for item_0 in value.patterns],
    }


def from_json_condition_selector(value: Json) -> ConditionSelector:
    """Return one ConditionSelector from one JSON value."""
    object_ = json_object(value)

    return ConditionSelector(
        patterns=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ConditionCatalog:
    """Named condition declarations from `destack.json`."""

    # named source graph modes
    modes: Mapping[str, Condition]
    # named source graph roles
    roles: Mapping[str, Condition]
    # named optional source graph features
    features: Mapping[str, Condition]
    # named source graph tags
    tags: Mapping[str, Condition]
    # named condition aliases
    aliases: Mapping[str, ConditionRef]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_catalog(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionCatalog:
        """Decode one ConditionCatalog."""
        return decode_condition_catalog(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_catalog(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionCatalog:
        """Return one ConditionCatalog from one JSON value."""
        return from_json_condition_catalog(value)


def encode_condition_catalog(writer: BinaryWriter, value: ConditionCatalog) -> None:
    """Encode one ConditionCatalog."""
    entries_value_modes_0 = []
    for key_value_modes_0, item_value_modes_0 in value.modes.items():

        def write_key_value_modes_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_modes_0)

        key_bytes = nested_bytes(write_key_value_modes_0)
        entries_value_modes_0.append((key_value_modes_0, item_value_modes_0, key_bytes))
    entries_value_modes_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_modes_0))
    for entry_value_modes_0 in entries_value_modes_0:
        writer.write_string(entry_value_modes_0[0])
        encode_condition(writer, entry_value_modes_0[1])
    entries_value_roles_0 = []
    for key_value_roles_0, item_value_roles_0 in value.roles.items():

        def write_key_value_roles_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_roles_0)

        key_bytes = nested_bytes(write_key_value_roles_0)
        entries_value_roles_0.append((key_value_roles_0, item_value_roles_0, key_bytes))
    entries_value_roles_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_roles_0))
    for entry_value_roles_0 in entries_value_roles_0:
        writer.write_string(entry_value_roles_0[0])
        encode_condition(writer, entry_value_roles_0[1])
    entries_value_features_0 = []
    for key_value_features_0, item_value_features_0 in value.features.items():

        def write_key_value_features_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_features_0)

        key_bytes = nested_bytes(write_key_value_features_0)
        entries_value_features_0.append(
            (key_value_features_0, item_value_features_0, key_bytes)
        )
    entries_value_features_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_features_0))
    for entry_value_features_0 in entries_value_features_0:
        writer.write_string(entry_value_features_0[0])
        encode_condition(writer, entry_value_features_0[1])
    entries_value_tags_0 = []
    for key_value_tags_0, item_value_tags_0 in value.tags.items():

        def write_key_value_tags_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_tags_0)

        key_bytes = nested_bytes(write_key_value_tags_0)
        entries_value_tags_0.append((key_value_tags_0, item_value_tags_0, key_bytes))
    entries_value_tags_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_tags_0))
    for entry_value_tags_0 in entries_value_tags_0:
        writer.write_string(entry_value_tags_0[0])
        encode_condition(writer, entry_value_tags_0[1])
    entries_value_aliases_0 = []
    for key_value_aliases_0, item_value_aliases_0 in value.aliases.items():

        def write_key_value_aliases_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_aliases_0)

        key_bytes = nested_bytes(write_key_value_aliases_0)
        entries_value_aliases_0.append(
            (key_value_aliases_0, item_value_aliases_0, key_bytes)
        )
    entries_value_aliases_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_aliases_0))
    for entry_value_aliases_0 in entries_value_aliases_0:
        writer.write_string(entry_value_aliases_0[0])
        encode_condition_ref(writer, entry_value_aliases_0[1])


def decode_condition_catalog(reader: BinaryReader) -> ConditionCatalog:
    """Decode one ConditionCatalog."""
    modes = {
        reader.read_string(): decode_condition(reader)
        for _ in range(reader.read_number())
    }
    roles = {
        reader.read_string(): decode_condition(reader)
        for _ in range(reader.read_number())
    }
    features = {
        reader.read_string(): decode_condition(reader)
        for _ in range(reader.read_number())
    }
    tags = {
        reader.read_string(): decode_condition(reader)
        for _ in range(reader.read_number())
    }
    aliases = {
        reader.read_string(): decode_condition_ref(reader)
        for _ in range(reader.read_number())
    }

    return ConditionCatalog(
        modes=modes,
        roles=roles,
        features=features,
        tags=tags,
        aliases=aliases,
    )


def to_json_condition_catalog(value: ConditionCatalog) -> Json:
    """Return one JSON value for one ConditionCatalog."""
    return {
        "modes": {
            key_0: to_json_condition(item_0) for key_0, item_0 in value.modes.items()
        },
        "roles": {
            key_0: to_json_condition(item_0) for key_0, item_0 in value.roles.items()
        },
        "features": {
            key_0: to_json_condition(item_0) for key_0, item_0 in value.features.items()
        },
        "tags": {
            key_0: to_json_condition(item_0) for key_0, item_0 in value.tags.items()
        },
        "aliases": {
            key_0: to_json_condition_ref(item_0)
            for key_0, item_0 in value.aliases.items()
        },
    }


def from_json_condition_catalog(value: Json) -> ConditionCatalog:
    """Return one ConditionCatalog from one JSON value."""
    object_ = json_object(value)

    return ConditionCatalog(
        modes={
            key_0: from_json_condition(item_0)
            for key_0, item_0 in json_object(json_field(object_, "modes")).items()
        },
        roles={
            key_0: from_json_condition(item_0)
            for key_0, item_0 in json_object(json_field(object_, "roles")).items()
        },
        features={
            key_0: from_json_condition(item_0)
            for key_0, item_0 in json_object(json_field(object_, "features")).items()
        },
        tags={
            key_0: from_json_condition(item_0)
            for key_0, item_0 in json_object(json_field(object_, "tags")).items()
        },
        aliases={
            key_0: from_json_condition_ref(item_0)
            for key_0, item_0 in json_object(json_field(object_, "aliases")).items()
        },
    )


@dataclass(frozen=True, slots=True)
class Condition:
    """Named source graph condition."""

    # human-readable condition description
    description: str | None
    # condition labels
    labels: Mapping[str, str]
    # condition names included before this condition
    extends: Sequence[str]
    # dependencies enabled by this condition
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Condition:
        """Decode one Condition."""
        return decode_condition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition(self)

    @classmethod
    def from_json(cls, value: Json) -> Condition:
        """Return one Condition from one JSON value."""
        return from_json_condition(value)


def encode_condition(writer: BinaryWriter, value: Condition) -> None:
    """Encode one Condition."""
    if value.description is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.description)
    entries_value_labels_0 = []
    for key_value_labels_0, item_value_labels_0 in value.labels.items():

        def write_key_value_labels_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_labels_0)

        key_bytes = nested_bytes(write_key_value_labels_0)
        entries_value_labels_0.append(
            (key_value_labels_0, item_value_labels_0, key_bytes)
        )
    entries_value_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_labels_0))
    for entry_value_labels_0 in entries_value_labels_0:
        writer.write_string(entry_value_labels_0[0])
        writer.write_string(entry_value_labels_0[1])
    writer.write_unsigned(len(value.extends))
    for item_value_extends_0 in value.extends:
        writer.write_string(item_value_extends_0)
    entries_value_dependencies_0 = []
    for (
        key_value_dependencies_0,
        item_value_dependencies_0,
    ) in value.dependencies.items():

        def write_key_value_dependencies_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_dependencies_0)

        key_bytes = nested_bytes(write_key_value_dependencies_0)
        entries_value_dependencies_0.append(
            (key_value_dependencies_0, item_value_dependencies_0, key_bytes)
        )
    entries_value_dependencies_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_dependencies_0))
    for entry_value_dependencies_0 in entries_value_dependencies_0:
        writer.write_string(entry_value_dependencies_0[0])
        destack._generated.repository.config.dependency.encode_dependency(
            writer, entry_value_dependencies_0[1]
        )


def decode_condition(reader: BinaryReader) -> Condition:
    """Decode one Condition."""
    description = reader.read_option(lambda: reader.read_string())
    labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }
    extends = [reader.read_string() for _ in range(reader.read_number())]
    dependencies = {
        reader.read_string(): destack._generated.repository.config.dependency.decode_dependency(
            reader
        )
        for _ in range(reader.read_number())
    }

    return Condition(
        description=description,
        labels=labels,
        extends=extends,
        dependencies=dependencies,
    )


def to_json_condition(value: Condition) -> Json:
    """Return one JSON value for one Condition."""
    return {
        **({} if value.description is None else {"description": value.description}),
        "labels": {key_0: item_0 for key_0, item_0 in value.labels.items()},
        "extends": [item_0 for item_0 in value.extends],
        "dependencies": {
            key_0: destack._generated.repository.config.dependency.to_json_dependency(
                item_0
            )
            for key_0, item_0 in value.dependencies.items()
        },
    }


def from_json_condition(value: Json) -> Condition:
    """Return one Condition from one JSON value."""
    object_ = json_object(value)

    return Condition(
        description=json_optional(
            object_, "description", lambda value: json_string(value)
        ),
        labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "labels")).items()
        },
        extends=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "extends"))
        ],
        dependencies={
            key_0: destack._generated.repository.config.dependency.from_json_dependency(
                item_0
            )
            for key_0, item_0 in json_object(
                json_field(object_, "dependencies")
            ).items()
        },
    )


__all__ = [
    "ConditionRef",
    "encode_condition_ref",
    "decode_condition_ref",
    "to_json_condition_ref",
    "from_json_condition_ref",
    "ConditionRefName",
    "ConditionRefPredicate",
    "ConditionPredicate",
    "encode_condition_predicate",
    "decode_condition_predicate",
    "to_json_condition_predicate",
    "from_json_condition_predicate",
    "ConditionSelector",
    "encode_condition_selector",
    "decode_condition_selector",
    "to_json_condition_selector",
    "from_json_condition_selector",
    "ConditionCatalog",
    "encode_condition_catalog",
    "decode_condition_catalog",
    "to_json_condition_catalog",
    "from_json_condition_catalog",
    "Condition",
    "encode_condition",
    "decode_condition",
    "to_json_condition",
    "from_json_condition",
]
