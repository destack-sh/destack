# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.artifact.map
import destack._generated.js.tree.script

"""One structured target language."""
ScriptLanguage: typing.TypeAlias = (
    typing.Literal["javaScript"] | typing.Literal["typeScript"]
)


def encode_script_language(writer: BinaryWriter, value: ScriptLanguage) -> None:
    """Encode one ScriptLanguage."""
    if value == "javaScript":
        writer.write_unsigned(0)
    elif value == "typeScript":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_script_language(reader: BinaryReader) -> ScriptLanguage:
    """Decode one ScriptLanguage."""
    variant = reader.read_number()

    if variant == 0:
        return "javaScript"
    elif variant == 1:
        return "typeScript"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_script_language(value: ScriptLanguage) -> Json:
    """Return one JSON value for one ScriptLanguage."""
    return value


def from_json_script_language(value: Json) -> ScriptLanguage:
    """Return one ScriptLanguage from one JSON value."""
    variant = json_string(value)

    if variant == "javaScript":
        return "javaScript"
    elif variant == "typeScript":
        return "typeScript"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ScriptBodyEcmaScript:
    """ECMAScript-family module IR."""

    ecma_script: destack._generated.js.tree.script.Module
    kind: typing.Literal["ecmaScript"] = "ecmaScript"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_script_body(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_script_body(self)


"""One structured script body."""
ScriptBody: typing.TypeAlias = ScriptBodyEcmaScript


def encode_script_body(writer: BinaryWriter, value: ScriptBody) -> None:
    """Encode one ScriptBody."""
    if value.kind == "ecmaScript":
        writer.write_unsigned(0)
        destack._generated.js.tree.script.encode_module(writer, value.ecma_script)
    else:
        raise SerdeError("unknown enum variant")


def decode_script_body(reader: BinaryReader) -> ScriptBody:
    """Decode one ScriptBody."""
    variant = reader.read_number()

    if variant == 0:
        ecma_script = destack._generated.js.tree.script.decode_module(reader)

        return ScriptBodyEcmaScript(ecma_script=ecma_script)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_script_body(value: ScriptBody) -> Json:
    """Return one JSON value for one ScriptBody."""
    if value.kind == "ecmaScript":
        return {
            "kind": "ecmaScript",
            "ecma_script": destack._generated.js.tree.script.to_json_module(
                value.ecma_script
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_script_body(value: Json) -> ScriptBody:
    """Return one ScriptBody from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "ecmaScript":
        return ScriptBodyEcmaScript(
            ecma_script=destack._generated.js.tree.script.from_json_module(
                json_field(object_, "ecma_script")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class Script:
    """One structured script linker input for a target."""

    # the target language of this script
    language: ScriptLanguage
    # the structured script body
    body: ScriptBody
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None
    # whether this script has top level side effects
    has_top_level_side_effects: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_script(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Script:
        """Decode one Script."""
        return decode_script(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_script(self)

    @classmethod
    def from_json(cls, value: Json) -> Script:
        """Return one Script from one JSON value."""
        return from_json_script(value)


def encode_script(writer: BinaryWriter, value: Script) -> None:
    """Encode one Script."""
    encode_script_language(writer, value.language)
    encode_script_body(writer, value.body)
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.map.encode_source_map(writer, value.map)
    writer.write_bool(value.has_top_level_side_effects)


def decode_script(reader: BinaryReader) -> Script:
    """Decode one Script."""
    language = decode_script_language(reader)
    body = decode_script_body(reader)
    map = reader.read_option(
        lambda: destack._generated.artifact.map.decode_source_map(reader)
    )
    has_top_level_side_effects = reader.read_bool()

    return Script(
        language=language,
        body=body,
        map=map,
        has_top_level_side_effects=has_top_level_side_effects,
    )


def to_json_script(value: Script) -> Json:
    """Return one JSON value for one Script."""
    return {
        "language": to_json_script_language(value.language),
        "body": to_json_script_body(value.body),
        **(
            {}
            if value.map is None
            else {"map": destack._generated.artifact.map.to_json_source_map(value.map)}
        ),
        "hasTopLevelSideEffects": value.has_top_level_side_effects,
    }


def from_json_script(value: Json) -> Script:
    """Return one Script from one JSON value."""
    object_ = json_object(value)

    return Script(
        language=from_json_script_language(json_field(object_, "language")),
        body=from_json_script_body(json_field(object_, "body")),
        map=json_optional(
            object_,
            "map",
            lambda value: destack._generated.artifact.map.from_json_source_map(value),
        ),
        has_top_level_side_effects=json_bool(
            json_field(object_, "hasTopLevelSideEffects")
        ),
    )


__all__ = [
    "ScriptLanguage",
    "encode_script_language",
    "decode_script_language",
    "to_json_script_language",
    "from_json_script_language",
    "ScriptBody",
    "encode_script_body",
    "decode_script_body",
    "to_json_script_body",
    "from_json_script_body",
    "ScriptBodyEcmaScript",
    "Script",
    "encode_script",
    "decode_script",
    "to_json_script",
    "from_json_script",
]
