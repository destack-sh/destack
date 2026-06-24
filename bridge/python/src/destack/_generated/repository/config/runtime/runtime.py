# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
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

import destack._generated.artifact.core.condition
import destack._generated.artifact.core.target
import destack._generated.repository.config.runtime.clock
import destack._generated.repository.config.runtime.diagnostic
import destack._generated.repository.config.runtime.execution
import destack._generated.repository.config.runtime.heap
import destack._generated.repository.config.runtime.host
import destack._generated.repository.config.runtime.random
import destack._generated.repository.config.runtime.trace
import destack._generated.repository.config.runtime.worker


@dataclass(frozen=True, slots=True)
class RuntimeOptions:
    """Runtime configuration."""

    # runtime topology and policy identity
    identity: RuntimeIdentityOptions
    # build distribution profile
    profile: destack._generated.artifact.core.target.BuildProfile
    # build payload linkage
    linkage: destack._generated.artifact.core.target.BuildLinkage
    # active source graph conditions for runtime policy selection
    conditions: destack._generated.artifact.core.condition.ConditionSet
    # execution mode for scheduling and effect handling
    mode: destack._generated.repository.config.runtime.execution.ExecutionMode
    # runtime worker configuration
    worker: destack._generated.repository.config.runtime.worker.WorkerOptions
    # runtime clock seed configuration
    clock: destack._generated.repository.config.runtime.clock.ClockOptions
    # runtime randomness source configuration
    random: destack._generated.repository.config.runtime.random.RandomOptions
    # runtime trace configuration
    trace: destack._generated.repository.config.runtime.trace.TraceOptions
    # runtime heap configuration
    heap: destack._generated.repository.config.runtime.heap.HeapOptions
    # runtime diagnostics configuration
    diagnostic: (
        destack._generated.repository.config.runtime.diagnostic.RuntimeDiagnosticOptions
    )
    # runtime host module defaults
    host: destack._generated.repository.config.runtime.host.HostOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeOptions:
        """Decode one RuntimeOptions."""
        return decode_runtime_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeOptions:
        """Return one RuntimeOptions from one JSON value."""
        return from_json_runtime_options(value)


def encode_runtime_options(writer: BinaryWriter, value: RuntimeOptions) -> None:
    """Encode one RuntimeOptions."""
    encode_runtime_identity_options(writer, value.identity)
    destack._generated.artifact.core.target.encode_build_profile(writer, value.profile)
    destack._generated.artifact.core.target.encode_build_linkage(writer, value.linkage)
    destack._generated.artifact.core.condition.encode_condition_set(
        writer, value.conditions
    )
    destack._generated.repository.config.runtime.execution.encode_execution_mode(
        writer, value.mode
    )
    destack._generated.repository.config.runtime.worker.encode_worker_options(
        writer, value.worker
    )
    destack._generated.repository.config.runtime.clock.encode_clock_options(
        writer, value.clock
    )
    destack._generated.repository.config.runtime.random.encode_random_options(
        writer, value.random
    )
    destack._generated.repository.config.runtime.trace.encode_trace_options(
        writer, value.trace
    )
    destack._generated.repository.config.runtime.heap.encode_heap_options(
        writer, value.heap
    )
    destack._generated.repository.config.runtime.diagnostic.encode_runtime_diagnostic_options(
        writer, value.diagnostic
    )
    destack._generated.repository.config.runtime.host.encode_host_options(
        writer, value.host
    )


def decode_runtime_options(reader: BinaryReader) -> RuntimeOptions:
    """Decode one RuntimeOptions."""
    identity = decode_runtime_identity_options(reader)
    profile = destack._generated.artifact.core.target.decode_build_profile(reader)
    linkage = destack._generated.artifact.core.target.decode_build_linkage(reader)
    conditions = destack._generated.artifact.core.condition.decode_condition_set(reader)
    mode = destack._generated.repository.config.runtime.execution.decode_execution_mode(
        reader
    )
    worker = destack._generated.repository.config.runtime.worker.decode_worker_options(
        reader
    )
    clock = destack._generated.repository.config.runtime.clock.decode_clock_options(
        reader
    )
    random = destack._generated.repository.config.runtime.random.decode_random_options(
        reader
    )
    trace = destack._generated.repository.config.runtime.trace.decode_trace_options(
        reader
    )
    heap = destack._generated.repository.config.runtime.heap.decode_heap_options(reader)
    diagnostic = destack._generated.repository.config.runtime.diagnostic.decode_runtime_diagnostic_options(
        reader
    )
    host = destack._generated.repository.config.runtime.host.decode_host_options(reader)

    return RuntimeOptions(
        identity=identity,
        profile=profile,
        linkage=linkage,
        conditions=conditions,
        mode=mode,
        worker=worker,
        clock=clock,
        random=random,
        trace=trace,
        heap=heap,
        diagnostic=diagnostic,
        host=host,
    )


def to_json_runtime_options(value: RuntimeOptions) -> Json:
    """Return one JSON value for one RuntimeOptions."""
    return {
        "identity": to_json_runtime_identity_options(value.identity),
        "profile": destack._generated.artifact.core.target.to_json_build_profile(
            value.profile
        ),
        "linkage": destack._generated.artifact.core.target.to_json_build_linkage(
            value.linkage
        ),
        "conditions": destack._generated.artifact.core.condition.to_json_condition_set(
            value.conditions
        ),
        "mode": destack._generated.repository.config.runtime.execution.to_json_execution_mode(
            value.mode
        ),
        "worker": destack._generated.repository.config.runtime.worker.to_json_worker_options(
            value.worker
        ),
        "clock": destack._generated.repository.config.runtime.clock.to_json_clock_options(
            value.clock
        ),
        "random": destack._generated.repository.config.runtime.random.to_json_random_options(
            value.random
        ),
        "trace": destack._generated.repository.config.runtime.trace.to_json_trace_options(
            value.trace
        ),
        "heap": destack._generated.repository.config.runtime.heap.to_json_heap_options(
            value.heap
        ),
        "diagnostic": destack._generated.repository.config.runtime.diagnostic.to_json_runtime_diagnostic_options(
            value.diagnostic
        ),
        "host": destack._generated.repository.config.runtime.host.to_json_host_options(
            value.host
        ),
    }


def from_json_runtime_options(value: Json) -> RuntimeOptions:
    """Return one RuntimeOptions from one JSON value."""
    object_ = json_object(value)

    return RuntimeOptions(
        identity=from_json_runtime_identity_options(json_field(object_, "identity")),
        profile=destack._generated.artifact.core.target.from_json_build_profile(
            json_field(object_, "profile")
        ),
        linkage=destack._generated.artifact.core.target.from_json_build_linkage(
            json_field(object_, "linkage")
        ),
        conditions=destack._generated.artifact.core.condition.from_json_condition_set(
            json_field(object_, "conditions")
        ),
        mode=destack._generated.repository.config.runtime.execution.from_json_execution_mode(
            json_field(object_, "mode")
        ),
        worker=destack._generated.repository.config.runtime.worker.from_json_worker_options(
            json_field(object_, "worker")
        ),
        clock=destack._generated.repository.config.runtime.clock.from_json_clock_options(
            json_field(object_, "clock")
        ),
        random=destack._generated.repository.config.runtime.random.from_json_random_options(
            json_field(object_, "random")
        ),
        trace=destack._generated.repository.config.runtime.trace.from_json_trace_options(
            json_field(object_, "trace")
        ),
        heap=destack._generated.repository.config.runtime.heap.from_json_heap_options(
            json_field(object_, "heap")
        ),
        diagnostic=destack._generated.repository.config.runtime.diagnostic.from_json_runtime_diagnostic_options(
            json_field(object_, "diagnostic")
        ),
        host=destack._generated.repository.config.runtime.host.from_json_host_options(
            json_field(object_, "host")
        ),
    )


@dataclass(frozen=True, slots=True)
class RuntimeIdentityOptions:
    """Runtime identity used for topology and policy selection."""

    # stable runtime name for policy selection
    name: str | None
    # stable runtime labels for topology and policy selection
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_identity_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeIdentityOptions:
        """Decode one RuntimeIdentityOptions."""
        return decode_runtime_identity_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_identity_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeIdentityOptions:
        """Return one RuntimeIdentityOptions from one JSON value."""
        return from_json_runtime_identity_options(value)


def encode_runtime_identity_options(
    writer: BinaryWriter, value: RuntimeIdentityOptions
) -> None:
    """Encode one RuntimeIdentityOptions."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
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


def decode_runtime_identity_options(reader: BinaryReader) -> RuntimeIdentityOptions:
    """Decode one RuntimeIdentityOptions."""
    name = reader.read_option(lambda: reader.read_string())
    labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }

    return RuntimeIdentityOptions(
        name=name,
        labels=labels,
    )


def to_json_runtime_identity_options(value: RuntimeIdentityOptions) -> Json:
    """Return one JSON value for one RuntimeIdentityOptions."""
    return {
        **({} if value.name is None else {"name": value.name}),
        "labels": {key_0: item_0 for key_0, item_0 in value.labels.items()},
    }


def from_json_runtime_identity_options(value: Json) -> RuntimeIdentityOptions:
    """Return one RuntimeIdentityOptions from one JSON value."""
    object_ = json_object(value)

    return RuntimeIdentityOptions(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "labels")).items()
        },
    )


__all__ = [
    "RuntimeOptions",
    "encode_runtime_options",
    "decode_runtime_options",
    "to_json_runtime_options",
    "from_json_runtime_options",
    "RuntimeIdentityOptions",
    "encode_runtime_identity_options",
    "decode_runtime_identity_options",
    "to_json_runtime_identity_options",
    "from_json_runtime_identity_options",
]
