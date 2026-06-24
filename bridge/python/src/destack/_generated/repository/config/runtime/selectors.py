# generated bridge target, do not edit

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


@dataclass(frozen=True, slots=True)
class RuntimeIdentitySelector:
    """Runtime identity selector for worker and runtime scopes."""

    # name selector for one runtime or one worker
    name: str | None
    # label selector for one runtime or one worker
    labels: RuntimeLabelSelector | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_identity_selector(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeIdentitySelector:
        """Decode one RuntimeIdentitySelector."""
        return decode_runtime_identity_selector(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_identity_selector(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeIdentitySelector:
        """Return one RuntimeIdentitySelector from one JSON value."""
        return from_json_runtime_identity_selector(value)


def encode_runtime_identity_selector(
    writer: BinaryWriter, value: RuntimeIdentitySelector
) -> None:
    """Encode one RuntimeIdentitySelector."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    if value.labels is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_runtime_label_selector(writer, value.labels)


def decode_runtime_identity_selector(reader: BinaryReader) -> RuntimeIdentitySelector:
    """Decode one RuntimeIdentitySelector."""
    name = reader.read_option(lambda: reader.read_string())
    labels = reader.read_option(lambda: decode_runtime_label_selector(reader))

    return RuntimeIdentitySelector(
        name=name,
        labels=labels,
    )


def to_json_runtime_identity_selector(value: RuntimeIdentitySelector) -> Json:
    """Return one JSON value for one RuntimeIdentitySelector."""
    return {
        **({} if value.name is None else {"name": value.name}),
        **(
            {}
            if value.labels is None
            else {"labels": to_json_runtime_label_selector(value.labels)}
        ),
    }


def from_json_runtime_identity_selector(value: Json) -> RuntimeIdentitySelector:
    """Return one RuntimeIdentitySelector from one JSON value."""
    object_ = json_object(value)

    return RuntimeIdentitySelector(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        labels=json_optional(
            object_, "labels", lambda value: from_json_runtime_label_selector(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class RuntimeLabelSelector:
    """Kubernetes-style label selector for runtime identity."""

    # exact-match labels that must all be present
    match_labels: Mapping[str, str]
    # additional set-based label requirements
    match_expressions: Sequence[RuntimeLabelRequirement]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_label_selector(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeLabelSelector:
        """Decode one RuntimeLabelSelector."""
        return decode_runtime_label_selector(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_label_selector(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeLabelSelector:
        """Return one RuntimeLabelSelector from one JSON value."""
        return from_json_runtime_label_selector(value)


def encode_runtime_label_selector(
    writer: BinaryWriter, value: RuntimeLabelSelector
) -> None:
    """Encode one RuntimeLabelSelector."""
    entries_value_match_labels_0 = []
    for (
        key_value_match_labels_0,
        item_value_match_labels_0,
    ) in value.match_labels.items():

        def write_key_value_match_labels_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_match_labels_0)

        key_bytes = nested_bytes(write_key_value_match_labels_0)
        entries_value_match_labels_0.append(
            (key_value_match_labels_0, item_value_match_labels_0, key_bytes)
        )
    entries_value_match_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_match_labels_0))
    for entry_value_match_labels_0 in entries_value_match_labels_0:
        writer.write_string(entry_value_match_labels_0[0])
        writer.write_string(entry_value_match_labels_0[1])
    writer.write_unsigned(len(value.match_expressions))
    for item_value_match_expressions_0 in value.match_expressions:
        encode_runtime_label_requirement(writer, item_value_match_expressions_0)


def decode_runtime_label_selector(reader: BinaryReader) -> RuntimeLabelSelector:
    """Decode one RuntimeLabelSelector."""
    match_labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }
    match_expressions = [
        decode_runtime_label_requirement(reader) for _ in range(reader.read_number())
    ]

    return RuntimeLabelSelector(
        match_labels=match_labels,
        match_expressions=match_expressions,
    )


def to_json_runtime_label_selector(value: RuntimeLabelSelector) -> Json:
    """Return one JSON value for one RuntimeLabelSelector."""
    return {
        "matchLabels": {key_0: item_0 for key_0, item_0 in value.match_labels.items()},
        "matchExpressions": [
            to_json_runtime_label_requirement(item_0)
            for item_0 in value.match_expressions
        ],
    }


def from_json_runtime_label_selector(value: Json) -> RuntimeLabelSelector:
    """Return one RuntimeLabelSelector from one JSON value."""
    object_ = json_object(value)

    return RuntimeLabelSelector(
        match_labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "matchLabels")).items()
        },
        match_expressions=[
            from_json_runtime_label_requirement(item_0)
            for item_0 in json_array(json_field(object_, "matchExpressions"))
        ],
    )


@dataclass(frozen=True, slots=True)
class RuntimeLabelRequirement:
    """One label requirement clause for runtime identity selectors."""

    # label key to evaluate
    key: str
    # label requirement operator
    operator: RuntimeLabelOperator
    # label values for set-based operators
    values: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_label_requirement(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeLabelRequirement:
        """Decode one RuntimeLabelRequirement."""
        return decode_runtime_label_requirement(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_label_requirement(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeLabelRequirement:
        """Return one RuntimeLabelRequirement from one JSON value."""
        return from_json_runtime_label_requirement(value)


def encode_runtime_label_requirement(
    writer: BinaryWriter, value: RuntimeLabelRequirement
) -> None:
    """Encode one RuntimeLabelRequirement."""
    writer.write_string(value.key)
    encode_runtime_label_operator(writer, value.operator)
    writer.write_unsigned(len(value.values))
    for item_value_values_0 in value.values:
        writer.write_string(item_value_values_0)


def decode_runtime_label_requirement(reader: BinaryReader) -> RuntimeLabelRequirement:
    """Decode one RuntimeLabelRequirement."""
    key = reader.read_string()
    operator = decode_runtime_label_operator(reader)
    values = [reader.read_string() for _ in range(reader.read_number())]

    return RuntimeLabelRequirement(
        key=key,
        operator=operator,
        values=values,
    )


def to_json_runtime_label_requirement(value: RuntimeLabelRequirement) -> Json:
    """Return one JSON value for one RuntimeLabelRequirement."""
    return {
        "key": value.key,
        "operator": to_json_runtime_label_operator(value.operator),
        "values": [item_0 for item_0 in value.values],
    }


def from_json_runtime_label_requirement(value: Json) -> RuntimeLabelRequirement:
    """Return one RuntimeLabelRequirement from one JSON value."""
    object_ = json_object(value)

    return RuntimeLabelRequirement(
        key=json_string(json_field(object_, "key")),
        operator=from_json_runtime_label_operator(json_field(object_, "operator")),
        values=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "values"))
        ],
    )


"""Label selection operator for runtime identity selectors."""
RuntimeLabelOperator: typing.TypeAlias = (
    typing.Literal["in"]
    | typing.Literal["notIn"]
    | typing.Literal["exists"]
    | typing.Literal["doesNotExist"]
)


def encode_runtime_label_operator(
    writer: BinaryWriter, value: RuntimeLabelOperator
) -> None:
    """Encode one RuntimeLabelOperator."""
    if value == "in":
        writer.write_unsigned(0)
    elif value == "notIn":
        writer.write_unsigned(1)
    elif value == "exists":
        writer.write_unsigned(2)
    elif value == "doesNotExist":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_runtime_label_operator(reader: BinaryReader) -> RuntimeLabelOperator:
    """Decode one RuntimeLabelOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "in"
    elif variant == 1:
        return "notIn"
    elif variant == 2:
        return "exists"
    elif variant == 3:
        return "doesNotExist"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_runtime_label_operator(value: RuntimeLabelOperator) -> Json:
    """Return one JSON value for one RuntimeLabelOperator."""
    return value


def from_json_runtime_label_operator(value: Json) -> RuntimeLabelOperator:
    """Return one RuntimeLabelOperator from one JSON value."""
    variant = json_string(value)

    if variant == "in":
        return "in"
    elif variant == "notIn":
        return "notIn"
    elif variant == "exists":
        return "exists"
    elif variant == "doesNotExist":
        return "doesNotExist"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "RuntimeIdentitySelector",
    "encode_runtime_identity_selector",
    "decode_runtime_identity_selector",
    "to_json_runtime_identity_selector",
    "from_json_runtime_identity_selector",
    "RuntimeLabelSelector",
    "encode_runtime_label_selector",
    "decode_runtime_label_selector",
    "to_json_runtime_label_selector",
    "from_json_runtime_label_selector",
    "RuntimeLabelRequirement",
    "encode_runtime_label_requirement",
    "decode_runtime_label_requirement",
    "to_json_runtime_label_requirement",
    "from_json_runtime_label_requirement",
    "RuntimeLabelOperator",
    "encode_runtime_label_operator",
    "decode_runtime_label_operator",
    "to_json_runtime_label_operator",
    "from_json_runtime_label_operator",
]
