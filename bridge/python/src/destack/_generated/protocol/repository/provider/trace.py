# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""Trace detail returned to a caller."""
TraceView: TypeAlias = Literal["summary"] | Literal["detailed"]


def encode_trace_view(writer: Writer, value: TraceView) -> None:
    if value == "summary":
        writer.write_unsigned(0)
    elif value == "detailed":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_trace_view(reader: Reader) -> TraceView:
    variant = reader.read_number()

    if variant == 0:
        return "summary"
    elif variant == 1:
        return "detailed"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class TraceSnapshot:
    """Serializable snapshot of one trace."""

    """The wall time of the traced operation in microseconds."""
    total_micros: int
    """The number of workers that recorded attempts."""
    workers: int
    """Artifact stats."""
    stats: TraceStats
    """Operation-level spans around artifact execution."""
    spans: Sequence[TraceSpanSnapshot]
    """Operation-level counters."""
    counters: Sequence[TraceCounterSnapshot]
    """Busy time per toolchain stage, ordered by stage."""
    stages: Sequence[TraceStageSnapshot]
    """Summed time per named trace span."""
    times: Sequence[TraceTimeSnapshot]
    """The recorded artifact attempts, present only in detailed snapshots."""
    artifacts: Sequence[ArtifactAttemptSnapshot]


def encode_trace_snapshot(writer: Writer, value: TraceSnapshot) -> None:
    writer.write_unsigned(value.total_micros)
    writer.write_unsigned(value.workers)
    encode_trace_stats(writer, value.stats)
    writer.write_unsigned(len(value.spans))
    for item_0 in value.spans:
        encode_trace_span_snapshot(writer, item_0)
    writer.write_unsigned(len(value.counters))
    for item_0 in value.counters:
        encode_trace_counter_snapshot(writer, item_0)
    writer.write_unsigned(len(value.stages))
    for item_0 in value.stages:
        encode_trace_stage_snapshot(writer, item_0)
    writer.write_unsigned(len(value.times))
    for item_0 in value.times:
        encode_trace_time_snapshot(writer, item_0)
    writer.write_unsigned(len(value.artifacts))
    for item_0 in value.artifacts:
        encode_artifact_attempt_snapshot(writer, item_0)


def decode_trace_snapshot(reader: Reader) -> TraceSnapshot:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = decode_trace_stats(reader)
    field_3 = [decode_trace_span_snapshot(reader) for _ in range(reader.read_number())]
    field_4 = [
        decode_trace_counter_snapshot(reader) for _ in range(reader.read_number())
    ]
    field_5 = [decode_trace_stage_snapshot(reader) for _ in range(reader.read_number())]
    field_6 = [decode_trace_time_snapshot(reader) for _ in range(reader.read_number())]
    field_7 = [
        decode_artifact_attempt_snapshot(reader) for _ in range(reader.read_number())
    ]

    return TraceSnapshot(
        total_micros=field_0,
        workers=field_1,
        stats=field_2,
        spans=field_3,
        counters=field_4,
        stages=field_5,
        times=field_6,
        artifacts=field_7,
    )


@dataclass(frozen=True, slots=True)
class TraceStats:
    """Artifact attempt outcome counts in one trace."""

    """Attempts that produced an artifact."""
    built: int
    """Attempts served from memory."""
    memory_cached: int
    """Attempts restored from the persistent store."""
    store_cached: int
    """Attempts parked on missing requirements."""
    parked: int
    """Attempts that failed."""
    failed: int


def encode_trace_stats(writer: Writer, value: TraceStats) -> None:
    writer.write_unsigned(value.built)
    writer.write_unsigned(value.memory_cached)
    writer.write_unsigned(value.store_cached)
    writer.write_unsigned(value.parked)
    writer.write_unsigned(value.failed)


def decode_trace_stats(reader: Reader) -> TraceStats:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = reader.read_number()
    field_3 = reader.read_number()
    field_4 = reader.read_number()

    return TraceStats(
        built=field_0,
        memory_cached=field_1,
        store_cached=field_2,
        parked=field_3,
        failed=field_4,
    )


@dataclass(frozen=True, slots=True)
class TraceSpanSnapshot:
    """One span in a trace snapshot."""

    """The span name."""
    name: str
    """The offset from the trace start in microseconds."""
    start_micros: int
    """The span duration in microseconds."""
    micros: int


def encode_trace_span_snapshot(writer: Writer, value: TraceSpanSnapshot) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.start_micros)
    writer.write_unsigned(value.micros)


def decode_trace_span_snapshot(reader: Reader) -> TraceSpanSnapshot:
    field_0 = reader.read_string()
    field_1 = reader.read_number()
    field_2 = reader.read_number()

    return TraceSpanSnapshot(
        name=field_0,
        start_micros=field_1,
        micros=field_2,
    )


@dataclass(frozen=True, slots=True)
class TraceCounterSnapshot:
    """One counter in a trace snapshot."""

    """The counter name."""
    name: str
    """The counter value."""
    value: int


def encode_trace_counter_snapshot(writer: Writer, value: TraceCounterSnapshot) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.value)


def decode_trace_counter_snapshot(reader: Reader) -> TraceCounterSnapshot:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceCounterSnapshot(
        name=field_0,
        value=field_1,
    )


@dataclass(frozen=True, slots=True)
class TraceStageSnapshot:
    """Busy time of one toolchain stage."""

    """The stage display name."""
    name: str
    """The summed attempt time in microseconds."""
    micros: int


def encode_trace_stage_snapshot(writer: Writer, value: TraceStageSnapshot) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_stage_snapshot(reader: Reader) -> TraceStageSnapshot:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceStageSnapshot(
        name=field_0,
        micros=field_1,
    )


@dataclass(frozen=True, slots=True)
class TraceTimeSnapshot:
    """Summed time of one named trace span."""

    """The span name."""
    name: str
    """The summed span time in microseconds."""
    micros: int


def encode_trace_time_snapshot(writer: Writer, value: TraceTimeSnapshot) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_time_snapshot(reader: Reader) -> TraceTimeSnapshot:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceTimeSnapshot(
        name=field_0,
        micros=field_1,
    )


@dataclass(frozen=True, slots=True)
class ArtifactAttemptSnapshot:
    """One artifact attempt in a detailed trace snapshot."""

    """The artifact kind name."""
    name: str
    """The toolchain stage display name."""
    stage: str
    """The resolved artifact label, usually a module display name."""
    label: str | None
    """The resolved target name for emitted and linked artifacts."""
    target: str | None
    """The worker that executed the attempt."""
    worker: int
    """The offset from the trace start in microseconds."""
    start_micros: int
    """The attempt duration in microseconds."""
    micros: int
    """The attempt outcome name."""
    outcome: str
    """Interior spans recorded by the executor or provider."""
    spans: Sequence[TraceSpanSnapshot]
    """Counters recorded by the executor or provider."""
    counters: Sequence[TraceCounterSnapshot]


def encode_artifact_attempt_snapshot(
    writer: Writer, value: ArtifactAttemptSnapshot
) -> None:
    writer.write_string(value.name)
    writer.write_string(value.stage)
    if value.label is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.label)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)
    writer.write_unsigned(value.worker)
    writer.write_unsigned(value.start_micros)
    writer.write_unsigned(value.micros)
    writer.write_string(value.outcome)
    writer.write_unsigned(len(value.spans))
    for item_0 in value.spans:
        encode_trace_span_snapshot(writer, item_0)
    writer.write_unsigned(len(value.counters))
    for item_0 in value.counters:
        encode_trace_counter_snapshot(writer, item_0)


def decode_artifact_attempt_snapshot(reader: Reader) -> ArtifactAttemptSnapshot:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_number()
    field_5 = reader.read_number()
    field_6 = reader.read_number()
    field_7 = reader.read_string()
    field_8 = [decode_trace_span_snapshot(reader) for _ in range(reader.read_number())]
    field_9 = [
        decode_trace_counter_snapshot(reader) for _ in range(reader.read_number())
    ]

    return ArtifactAttemptSnapshot(
        name=field_0,
        stage=field_1,
        label=field_2,
        target=field_3,
        worker=field_4,
        start_micros=field_5,
        micros=field_6,
        outcome=field_7,
        spans=field_8,
        counters=field_9,
    )


__all__ = [
    "TraceView",
    "encode_trace_view",
    "decode_trace_view",
    "TraceSnapshot",
    "encode_trace_snapshot",
    "decode_trace_snapshot",
    "TraceStats",
    "encode_trace_stats",
    "decode_trace_stats",
    "TraceSpanSnapshot",
    "encode_trace_span_snapshot",
    "decode_trace_span_snapshot",
    "TraceCounterSnapshot",
    "encode_trace_counter_snapshot",
    "decode_trace_counter_snapshot",
    "TraceStageSnapshot",
    "encode_trace_stage_snapshot",
    "decode_trace_stage_snapshot",
    "TraceTimeSnapshot",
    "encode_trace_time_snapshot",
    "decode_trace_time_snapshot",
    "ArtifactAttemptSnapshot",
    "encode_artifact_attempt_snapshot",
    "decode_artifact_attempt_snapshot",
]
