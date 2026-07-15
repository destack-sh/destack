# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_string,
    nested_bytes,
)


@dataclass(frozen=True, slots=True)
class LinterOptions:
    """Linter configuration."""

    # whether linting is enabled
    enabled: bool
    # explicit levels keyed by rule id or diagnostic code
    rules: Mapping[str, LintLevel]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterOptions:
        """Decode one LinterOptions."""
        return decode_linter_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterOptions:
        """Return one LinterOptions from one JSON value."""
        return from_json_linter_options(value)


def encode_linter_options(writer: BinaryWriter, value: LinterOptions) -> None:
    """Encode one LinterOptions."""
    writer.write_bool(value.enabled)
    entries_value_rules_0 = []
    for key_value_rules_0, item_value_rules_0 in value.rules.items():

        def write_key_value_rules_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_rules_0)

        key_bytes = nested_bytes(write_key_value_rules_0)
        entries_value_rules_0.append((key_value_rules_0, item_value_rules_0, key_bytes))
    entries_value_rules_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_rules_0))
    for entry_value_rules_0 in entries_value_rules_0:
        writer.write_string(entry_value_rules_0[0])
        encode_lint_level(writer, entry_value_rules_0[1])


def decode_linter_options(reader: BinaryReader) -> LinterOptions:
    """Decode one LinterOptions."""
    enabled = reader.read_bool()
    rules = {
        reader.read_string(): decode_lint_level(reader)
        for _ in range(reader.read_number())
    }

    return LinterOptions(
        enabled=enabled,
        rules=rules,
    )


def to_json_linter_options(value: LinterOptions) -> Json:
    """Return one JSON value for one LinterOptions."""
    return {
        "enabled": value.enabled,
        "rules": {
            key_0: to_json_lint_level(item_0) for key_0, item_0 in value.rules.items()
        },
    }


def from_json_linter_options(value: Json) -> LinterOptions:
    """Return one LinterOptions from one JSON value."""
    object_ = json_object(value)

    return LinterOptions(
        enabled=json_bool(json_field(object_, "enabled")),
        rules={
            key_0: from_json_lint_level(item_0)
            for key_0, item_0 in json_object(json_field(object_, "rules")).items()
        },
    )


"""One configured lint level."""
LintLevel: typing.TypeAlias = (
    typing.Literal["off"] | typing.Literal["warning"] | typing.Literal["error"]
)


def encode_lint_level(writer: BinaryWriter, value: LintLevel) -> None:
    """Encode one LintLevel."""
    if value == "off":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_lint_level(reader: BinaryReader) -> LintLevel:
    """Decode one LintLevel."""
    variant = reader.read_number()

    if variant == 0:
        return "off"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lint_level(value: LintLevel) -> Json:
    """Return one JSON value for one LintLevel."""
    return value


def from_json_lint_level(value: Json) -> LintLevel:
    """Return one LintLevel from one JSON value."""
    variant = json_string(value)

    if variant == "off":
        return "off"
    elif variant == "warning":
        return "warning"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LinterOptions",
    "encode_linter_options",
    "decode_linter_options",
    "to_json_linter_options",
    "from_json_linter_options",
    "LintLevel",
    "encode_lint_level",
    "decode_lint_level",
    "to_json_lint_level",
    "from_json_lint_level",
]
