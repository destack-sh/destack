# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
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
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class MacroTable:
    """Macro expansion state for one DIR module."""

    # the module id of the macro table
    module_id: destack._generated.source.file.model.module.ModuleId
    # expanded macro invocations
    invocations: Sequence[MacroInvocation]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_macro_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MacroTable:
        """Decode one MacroTable."""
        return decode_macro_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_macro_table(self)

    @classmethod
    def from_json(cls, value: Json) -> MacroTable:
        """Return one MacroTable from one JSON value."""
        return from_json_macro_table(value)


def encode_macro_table(writer: BinaryWriter, value: MacroTable) -> None:
    """Encode one MacroTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(len(value.invocations))
    for item_value_invocations_0 in value.invocations:
        encode_macro_invocation(writer, item_value_invocations_0)


def decode_macro_table(reader: BinaryReader) -> MacroTable:
    """Decode one MacroTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    invocations = [decode_macro_invocation(reader) for _ in range(reader.read_number())]

    return MacroTable(
        module_id=module_id,
        invocations=invocations,
    )


def to_json_macro_table(value: MacroTable) -> Json:
    """Return one JSON value for one MacroTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "invocations": [
            to_json_macro_invocation(item_0) for item_0 in value.invocations
        ],
    }


def from_json_macro_table(value: Json) -> MacroTable:
    """Return one MacroTable from one JSON value."""
    object_ = json_object(value)

    return MacroTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        invocations=[
            from_json_macro_invocation(item_0)
            for item_0 in json_array(json_field(object_, "invocations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class MacroInvocation:
    """One macro invocation completed during fixed-point expansion."""

    # the decorated target node
    target_node: destack._generated.dir.tree.node.GlobalNodeIdAny
    # what caused this macro invocation
    trigger: MacroTrigger
    # the resolved `Macro` implementation
    implementation: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the static macro configuration
    config: destack._generated.dir.tree.static.StaticTerm
    # state carried from expansion to materialization
    state: destack._generated.dir.tree.static.StaticTerm | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_macro_invocation(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MacroInvocation:
        """Decode one MacroInvocation."""
        return decode_macro_invocation(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_macro_invocation(self)

    @classmethod
    def from_json(cls, value: Json) -> MacroInvocation:
        """Return one MacroInvocation from one JSON value."""
        return from_json_macro_invocation(value)


def encode_macro_invocation(writer: BinaryWriter, value: MacroInvocation) -> None:
    """Encode one MacroInvocation."""
    destack._generated.dir.tree.node.encode_global_node_id_any(
        writer, value.target_node
    )
    encode_macro_trigger(writer, value.trigger)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.implementation
    )
    destack._generated.dir.tree.static.encode_static_term(writer, value.config)
    if value.state is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.static.encode_static_term(writer, value.state)


def decode_macro_invocation(reader: BinaryReader) -> MacroInvocation:
    """Decode one MacroInvocation."""
    target_node = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    trigger = decode_macro_trigger(reader)
    implementation = destack._generated.dir.symbol.symbol.decode_global_symbol_id(
        reader
    )
    config = destack._generated.dir.tree.static.decode_static_term(reader)
    state = reader.read_option(
        lambda: destack._generated.dir.tree.static.decode_static_term(reader)
    )

    return MacroInvocation(
        target_node=target_node,
        trigger=trigger,
        implementation=implementation,
        config=config,
        state=state,
    )


def to_json_macro_invocation(value: MacroInvocation) -> Json:
    """Return one JSON value for one MacroInvocation."""
    return {
        "targetNode": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target_node
        ),
        "trigger": to_json_macro_trigger(value.trigger),
        "implementation": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.implementation
        ),
        "config": destack._generated.dir.tree.static.to_json_static_term(value.config),
        **(
            {}
            if value.state is None
            else {
                "state": destack._generated.dir.tree.static.to_json_static_term(
                    value.state
                )
            }
        ),
    }


def from_json_macro_invocation(value: Json) -> MacroInvocation:
    """Return one MacroInvocation from one JSON value."""
    object_ = json_object(value)

    return MacroInvocation(
        target_node=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "targetNode")
        ),
        trigger=from_json_macro_trigger(json_field(object_, "trigger")),
        implementation=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "implementation")
        ),
        config=destack._generated.dir.tree.static.from_json_static_term(
            json_field(object_, "config")
        ),
        state=json_optional(
            object_,
            "state",
            lambda value: destack._generated.dir.tree.static.from_json_static_term(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class MacroTriggerDecorator:
    """A decorator on the target node."""

    decorator: destack._generated.dir.tree.node.GlobalNodeId
    kind: typing.Literal["decorator"] = "decorator"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_macro_trigger(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_macro_trigger(self)


@dataclass(frozen=True, slots=True)
class MacroTriggerAutoDerive:
    """A configured auto-derive provider."""

    kind: typing.Literal["autoDerive"] = "autoDerive"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_macro_trigger(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_macro_trigger(self)


"""What caused one macro invocation."""
MacroTrigger: typing.TypeAlias = MacroTriggerDecorator | MacroTriggerAutoDerive


def encode_macro_trigger(writer: BinaryWriter, value: MacroTrigger) -> None:
    """Encode one MacroTrigger."""
    if value.kind == "decorator":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_global_node_id(writer, value.decorator)
    elif value.kind == "autoDerive":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_macro_trigger(reader: BinaryReader) -> MacroTrigger:
    """Decode one MacroTrigger."""
    variant = reader.read_number()

    if variant == 0:
        decorator = destack._generated.dir.tree.node.decode_global_node_id(reader)

        return MacroTriggerDecorator(decorator=decorator)
    elif variant == 1:
        return MacroTriggerAutoDerive()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_macro_trigger(value: MacroTrigger) -> Json:
    """Return one JSON value for one MacroTrigger."""
    if value.kind == "decorator":
        return {
            "kind": "decorator",
            "decorator": destack._generated.dir.tree.node.to_json_global_node_id(
                value.decorator
            ),
        }
    elif value.kind == "autoDerive":
        return {
            "kind": "autoDerive",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_macro_trigger(value: Json) -> MacroTrigger:
    """Return one MacroTrigger from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "decorator":
        return MacroTriggerDecorator(
            decorator=destack._generated.dir.tree.node.from_json_global_node_id(
                json_field(object_, "decorator")
            )
        )
    elif kind == "autoDerive":
        return MacroTriggerAutoDerive()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "MacroTable",
    "encode_macro_table",
    "decode_macro_table",
    "to_json_macro_table",
    "from_json_macro_table",
    "MacroInvocation",
    "encode_macro_invocation",
    "decode_macro_invocation",
    "to_json_macro_invocation",
    "from_json_macro_invocation",
    "MacroTrigger",
    "encode_macro_trigger",
    "decode_macro_trigger",
    "to_json_macro_trigger",
    "from_json_macro_trigger",
    "MacroTriggerDecorator",
    "MacroTriggerAutoDerive",
]
