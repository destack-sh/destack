# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class TraceReport:
    """One bridge trace report."""

    """The wall time of the traced operation in microseconds."""
    total_micros: int
    """The number of workers that recorded attempts."""
    workers: int
    """Operation-level spans around artifact execution."""
    spans: Sequence[TraceSpan]
    """Operation-level counters."""
    counters: Sequence[TraceCounter]
    """Busy time per toolchain stage."""
    stages: Sequence[TraceStage]
    """Summed time per named trace span."""
    times: Sequence[TraceTime]
    """Detailed artifact attempts."""
    artifacts: Sequence[TraceArtifact]


def encode_trace_report(writer: Writer, value: TraceReport) -> None:
    writer.write_unsigned(value.total_micros)
    writer.write_unsigned(value.workers)
    writer.write_unsigned(len(value.spans))
    for item_0 in value.spans:
        encode_trace_span(writer, item_0)
    writer.write_unsigned(len(value.counters))
    for item_0 in value.counters:
        encode_trace_counter(writer, item_0)
    writer.write_unsigned(len(value.stages))
    for item_0 in value.stages:
        encode_trace_stage(writer, item_0)
    writer.write_unsigned(len(value.times))
    for item_0 in value.times:
        encode_trace_time(writer, item_0)
    writer.write_unsigned(len(value.artifacts))
    for item_0 in value.artifacts:
        encode_trace_artifact(writer, item_0)


def decode_trace_report(reader: Reader) -> TraceReport:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = [decode_trace_span(reader) for _ in range(reader.read_number())]
    field_3 = [decode_trace_counter(reader) for _ in range(reader.read_number())]
    field_4 = [decode_trace_stage(reader) for _ in range(reader.read_number())]
    field_5 = [decode_trace_time(reader) for _ in range(reader.read_number())]
    field_6 = [decode_trace_artifact(reader) for _ in range(reader.read_number())]

    return TraceReport(
        total_micros=field_0,
        workers=field_1,
        spans=field_2,
        counters=field_3,
        stages=field_4,
        times=field_5,
        artifacts=field_6,
    )


@dataclass(frozen=True, slots=True)
class TraceStage:
    """Busy time of one toolchain stage."""

    """The stage display name."""
    name: str
    """The summed attempt time in microseconds."""
    micros: int


def encode_trace_stage(writer: Writer, value: TraceStage) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_stage(reader: Reader) -> TraceStage:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceStage(
        name=field_0,
        micros=field_1,
    )


@dataclass(frozen=True, slots=True)
class TraceTime:
    """Summed time of one named trace span."""

    """The span name."""
    name: str
    """The summed span time in microseconds."""
    micros: int


def encode_trace_time(writer: Writer, value: TraceTime) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_time(reader: Reader) -> TraceTime:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceTime(
        name=field_0,
        micros=field_1,
    )


@dataclass(frozen=True, slots=True)
class TraceArtifact:
    """One artifact attempt in a detailed trace report."""

    """The artifact kind name."""
    name: str
    """The toolchain stage display name."""
    stage: str
    """The resolved artifact label."""
    label: str | None
    """The resolved target name."""
    target: str | None
    """The worker that executed the attempt."""
    worker: int
    """The offset from the run start in microseconds."""
    start_micros: int
    """The attempt duration in microseconds."""
    micros: int
    """The attempt outcome name."""
    outcome: str
    """Interior phases recorded by the executor or provider."""
    spans: Sequence[TraceSpan]
    """Counters recorded by the executor or provider."""
    counters: Sequence[TraceCounter]


def encode_trace_artifact(writer: Writer, value: TraceArtifact) -> None:
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
        encode_trace_span(writer, item_0)
    writer.write_unsigned(len(value.counters))
    for item_0 in value.counters:
        encode_trace_counter(writer, item_0)


def decode_trace_artifact(reader: Reader) -> TraceArtifact:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_number()
    field_5 = reader.read_number()
    field_6 = reader.read_number()
    field_7 = reader.read_string()
    field_8 = [decode_trace_span(reader) for _ in range(reader.read_number())]
    field_9 = [decode_trace_counter(reader) for _ in range(reader.read_number())]

    return TraceArtifact(
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


@dataclass(frozen=True, slots=True)
class TraceSpan:
    """One detailed span in a trace report."""

    """The phase name."""
    name: str
    """The offset from the run start in microseconds."""
    start_micros: int
    """The phase duration in microseconds."""
    micros: int


def encode_trace_span(writer: Writer, value: TraceSpan) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.start_micros)
    writer.write_unsigned(value.micros)


def decode_trace_span(reader: Reader) -> TraceSpan:
    field_0 = reader.read_string()
    field_1 = reader.read_number()
    field_2 = reader.read_number()

    return TraceSpan(
        name=field_0,
        start_micros=field_1,
        micros=field_2,
    )


@dataclass(frozen=True, slots=True)
class TraceCounter:
    """One detailed counter in a trace report."""

    """The counter name."""
    name: str
    """The counter value."""
    value: int


def encode_trace_counter(writer: Writer, value: TraceCounter) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(value.value)


def decode_trace_counter(reader: Reader) -> TraceCounter:
    field_0 = reader.read_string()
    field_1 = reader.read_number()

    return TraceCounter(
        name=field_0,
        value=field_1,
    )


__all__ = [
    "TraceReport",
    "encode_trace_report",
    "decode_trace_report",
    "TraceStage",
    "encode_trace_stage",
    "decode_trace_stage",
    "TraceTime",
    "encode_trace_time",
    "decode_trace_time",
    "TraceArtifact",
    "encode_trace_artifact",
    "decode_trace_artifact",
    "TraceSpan",
    "encode_trace_span",
    "decode_trace_span",
    "TraceCounter",
    "encode_trace_counter",
    "decode_trace_counter",
]
