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
import destack._generated.artifact.emit
import destack._generated.repository.config.policy
import destack._generated.repository.config.runtime.runtime
import destack._generated.repository.config.target.compiler
import destack._generated.repository.config.target.condition
import destack._generated.repository.config.target.js
import destack._generated.repository.config.target.native
import destack._generated.repository.config.target.output


@dataclass(frozen=True, slots=True)
class Target:
    """A build target configuration."""

    # entry points for entry-rooted targets
    entry: Sequence[str]
    # user callable launched by executable products
    entrypoint: Entrypoint | None
    # global modules added to every target root set
    globals: Sequence[str]
    # glob patterns for files to include when no entry is declared
    include: Sequence[str]
    # glob patterns for files to exclude
    exclude: Sequence[str]
    # policy declarations and rules for this target
    policy: destack._generated.repository.config.policy.Policy
    # emitted artifact family (js, ts, wasm, native)
    emit: destack._generated.artifact.emit.EmitFormat
    # target operating system
    platform: destack._generated.artifact.core.target.Platform
    # target host environment
    host: destack._generated.artifact.core.target.Host
    # source graph conditions for this target
    conditions: destack._generated.repository.config.target.condition.TargetConditionSet
    # compiler behavior for this target
    compiler: destack._generated.repository.config.target.compiler.TargetCompilerOptions
    # output paths and metadata options
    output: destack._generated.repository.config.target.output.TargetOutputOptions
    # javaScript output configuration
    js: destack._generated.repository.config.target.js.TargetJsOptions
    # native codegen and linking configuration
    native: destack._generated.repository.config.target.native.TargetNativeOptions
    # runtime execution options
    execution: destack._generated.repository.config.runtime.runtime.RuntimeOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Target:
        """Decode one Target."""
        return decode_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target(self)

    @classmethod
    def from_json(cls, value: Json) -> Target:
        """Return one Target from one JSON value."""
        return from_json_target(value)


def encode_target(writer: BinaryWriter, value: Target) -> None:
    """Encode one Target."""
    writer.write_unsigned(len(value.entry))
    for item_value_entry_0 in value.entry:
        writer.write_string(item_value_entry_0)
    if value.entrypoint is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_entrypoint(writer, value.entrypoint)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        writer.write_string(item_value_globals_0)
    writer.write_unsigned(len(value.include))
    for item_value_include_0 in value.include:
        writer.write_string(item_value_include_0)
    writer.write_unsigned(len(value.exclude))
    for item_value_exclude_0 in value.exclude:
        writer.write_string(item_value_exclude_0)
    destack._generated.repository.config.policy.encode_policy(writer, value.policy)
    destack._generated.artifact.emit.encode_emit_format(writer, value.emit)
    destack._generated.artifact.core.target.encode_platform(writer, value.platform)
    destack._generated.artifact.core.target.encode_host(writer, value.host)
    destack._generated.repository.config.target.condition.encode_target_condition_set(
        writer, value.conditions
    )
    destack._generated.repository.config.target.compiler.encode_target_compiler_options(
        writer, value.compiler
    )
    destack._generated.repository.config.target.output.encode_target_output_options(
        writer, value.output
    )
    destack._generated.repository.config.target.js.encode_target_js_options(
        writer, value.js
    )
    destack._generated.repository.config.target.native.encode_target_native_options(
        writer, value.native
    )
    destack._generated.repository.config.runtime.runtime.encode_runtime_options(
        writer, value.execution
    )


def decode_target(reader: BinaryReader) -> Target:
    """Decode one Target."""
    entry = [reader.read_string() for _ in range(reader.read_number())]
    entrypoint = reader.read_option(lambda: decode_entrypoint(reader))
    globals = [reader.read_string() for _ in range(reader.read_number())]
    include = [reader.read_string() for _ in range(reader.read_number())]
    exclude = [reader.read_string() for _ in range(reader.read_number())]
    policy = destack._generated.repository.config.policy.decode_policy(reader)
    emit = destack._generated.artifact.emit.decode_emit_format(reader)
    platform = destack._generated.artifact.core.target.decode_platform(reader)
    host = destack._generated.artifact.core.target.decode_host(reader)
    conditions = destack._generated.repository.config.target.condition.decode_target_condition_set(
        reader
    )
    compiler = destack._generated.repository.config.target.compiler.decode_target_compiler_options(
        reader
    )
    output = (
        destack._generated.repository.config.target.output.decode_target_output_options(
            reader
        )
    )
    js = destack._generated.repository.config.target.js.decode_target_js_options(reader)
    native = (
        destack._generated.repository.config.target.native.decode_target_native_options(
            reader
        )
    )
    execution = (
        destack._generated.repository.config.runtime.runtime.decode_runtime_options(
            reader
        )
    )

    return Target(
        entry=entry,
        entrypoint=entrypoint,
        globals=globals,
        include=include,
        exclude=exclude,
        policy=policy,
        emit=emit,
        platform=platform,
        host=host,
        conditions=conditions,
        compiler=compiler,
        output=output,
        js=js,
        native=native,
        execution=execution,
    )


def to_json_target(value: Target) -> Json:
    """Return one JSON value for one Target."""
    return {
        "entry": [item_0 for item_0 in value.entry],
        **(
            {}
            if value.entrypoint is None
            else {"entrypoint": to_json_entrypoint(value.entrypoint)}
        ),
        "globals": [item_0 for item_0 in value.globals],
        "include": [item_0 for item_0 in value.include],
        "exclude": [item_0 for item_0 in value.exclude],
        "policy": destack._generated.repository.config.policy.to_json_policy(
            value.policy
        ),
        "emit": destack._generated.artifact.emit.to_json_emit_format(value.emit),
        "platform": destack._generated.artifact.core.target.to_json_platform(
            value.platform
        ),
        "host": destack._generated.artifact.core.target.to_json_host(value.host),
        "conditions": destack._generated.repository.config.target.condition.to_json_target_condition_set(
            value.conditions
        ),
        "compiler": destack._generated.repository.config.target.compiler.to_json_target_compiler_options(
            value.compiler
        ),
        "output": destack._generated.repository.config.target.output.to_json_target_output_options(
            value.output
        ),
        "js": destack._generated.repository.config.target.js.to_json_target_js_options(
            value.js
        ),
        "native": destack._generated.repository.config.target.native.to_json_target_native_options(
            value.native
        ),
        "execution": destack._generated.repository.config.runtime.runtime.to_json_runtime_options(
            value.execution
        ),
    }


def from_json_target(value: Json) -> Target:
    """Return one Target from one JSON value."""
    object_ = json_object(value)

    return Target(
        entry=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "entry"))
        ],
        entrypoint=json_optional(
            object_, "entrypoint", lambda value: from_json_entrypoint(value)
        ),
        globals=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "globals"))
        ],
        include=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "include"))
        ],
        exclude=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "exclude"))
        ],
        policy=destack._generated.repository.config.policy.from_json_policy(
            json_field(object_, "policy")
        ),
        emit=destack._generated.artifact.emit.from_json_emit_format(
            json_field(object_, "emit")
        ),
        platform=destack._generated.artifact.core.target.from_json_platform(
            json_field(object_, "platform")
        ),
        host=destack._generated.artifact.core.target.from_json_host(
            json_field(object_, "host")
        ),
        conditions=destack._generated.repository.config.target.condition.from_json_target_condition_set(
            json_field(object_, "conditions")
        ),
        compiler=destack._generated.repository.config.target.compiler.from_json_target_compiler_options(
            json_field(object_, "compiler")
        ),
        output=destack._generated.repository.config.target.output.from_json_target_output_options(
            json_field(object_, "output")
        ),
        js=destack._generated.repository.config.target.js.from_json_target_js_options(
            json_field(object_, "js")
        ),
        native=destack._generated.repository.config.target.native.from_json_target_native_options(
            json_field(object_, "native")
        ),
        execution=destack._generated.repository.config.runtime.runtime.from_json_runtime_options(
            json_field(object_, "execution")
        ),
    )


@dataclass(frozen=True, slots=True)
class Entrypoint:
    """User callable launched by executable Destack products."""

    # module containing the exported entrypoint function
    module: str
    # exported function invoked after runtime bootstrap
    export: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entrypoint(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Entrypoint:
        """Decode one Entrypoint."""
        return decode_entrypoint(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entrypoint(self)

    @classmethod
    def from_json(cls, value: Json) -> Entrypoint:
        """Return one Entrypoint from one JSON value."""
        return from_json_entrypoint(value)


def encode_entrypoint(writer: BinaryWriter, value: Entrypoint) -> None:
    """Encode one Entrypoint."""
    writer.write_string(value.module)
    writer.write_string(value.export)


def decode_entrypoint(reader: BinaryReader) -> Entrypoint:
    """Decode one Entrypoint."""
    module = reader.read_string()
    export = reader.read_string()

    return Entrypoint(
        module=module,
        export=export,
    )


def to_json_entrypoint(value: Entrypoint) -> Json:
    """Return one JSON value for one Entrypoint."""
    return {
        "module": value.module,
        "export": value.export,
    }


def from_json_entrypoint(value: Json) -> Entrypoint:
    """Return one Entrypoint from one JSON value."""
    object_ = json_object(value)

    return Entrypoint(
        module=json_string(json_field(object_, "module")),
        export=json_string(json_field(object_, "export")),
    )


__all__ = [
    "Target",
    "encode_target",
    "decode_target",
    "to_json_target",
    "from_json_target",
    "Entrypoint",
    "encode_entrypoint",
    "decode_entrypoint",
    "to_json_entrypoint",
    "from_json_entrypoint",
]
