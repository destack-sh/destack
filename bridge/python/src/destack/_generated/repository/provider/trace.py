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
    json_int,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class TraceSnapshot:
    """Serializable snapshot of one trace."""

    # the wall time of the traced operation in microseconds
    total_micros: int
    # the number of workers that recorded attempts
    workers: int
    # artifact stats
    stats: TraceStats
    # operation-level spans around artifact execution
    spans: Sequence[TraceSpanSnapshot]
    # operation-level counters
    counters: Sequence[TraceCounterSnapshot]
    # busy time per toolchain stage, ordered by stage
    stages: Sequence[TraceStageSnapshot]
    # summed time per named trace span
    times: Sequence[TraceTimeSnapshot]
    # the recorded artifact attempts, present only in detailed snapshots
    artifacts: Sequence[ArtifactAttemptSnapshot]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceSnapshot:
        """Decode one TraceSnapshot."""
        return decode_trace_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceSnapshot:
        """Return one TraceSnapshot from one JSON value."""
        return from_json_trace_snapshot(value)


def encode_trace_snapshot(writer: BinaryWriter, value: TraceSnapshot) -> None:
    """Encode one TraceSnapshot."""
    writer.write_unsigned(value.total_micros)
    writer.write_unsigned(value.workers)
    encode_trace_stats(writer, value.stats)
    writer.write_unsigned(len(value.spans))
    for item_value_spans_0 in value.spans:
        encode_trace_span_snapshot(writer, item_value_spans_0)
    writer.write_unsigned(len(value.counters))
    for item_value_counters_0 in value.counters:
        encode_trace_counter_snapshot(writer, item_value_counters_0)
    writer.write_unsigned(len(value.stages))
    for item_value_stages_0 in value.stages:
        encode_trace_stage_snapshot(writer, item_value_stages_0)
    writer.write_unsigned(len(value.times))
    for item_value_times_0 in value.times:
        encode_trace_time_snapshot(writer, item_value_times_0)
    writer.write_unsigned(len(value.artifacts))
    for item_value_artifacts_0 in value.artifacts:
        encode_artifact_attempt_snapshot(writer, item_value_artifacts_0)


def decode_trace_snapshot(reader: BinaryReader) -> TraceSnapshot:
    """Decode one TraceSnapshot."""
    total_micros = reader.read_number()
    workers = reader.read_number()
    stats = decode_trace_stats(reader)
    spans = [decode_trace_span_snapshot(reader) for _ in range(reader.read_number())]
    counters = [
        decode_trace_counter_snapshot(reader) for _ in range(reader.read_number())
    ]
    stages = [decode_trace_stage_snapshot(reader) for _ in range(reader.read_number())]
    times = [decode_trace_time_snapshot(reader) for _ in range(reader.read_number())]
    artifacts = [
        decode_artifact_attempt_snapshot(reader) for _ in range(reader.read_number())
    ]

    return TraceSnapshot(
        total_micros=total_micros,
        workers=workers,
        stats=stats,
        spans=spans,
        counters=counters,
        stages=stages,
        times=times,
        artifacts=artifacts,
    )


def to_json_trace_snapshot(value: TraceSnapshot) -> Json:
    """Return one JSON value for one TraceSnapshot."""
    return {
        "totalMicros": value.total_micros,
        "workers": value.workers,
        "stats": to_json_trace_stats(value.stats),
        "spans": [to_json_trace_span_snapshot(item_0) for item_0 in value.spans],
        "counters": [
            to_json_trace_counter_snapshot(item_0) for item_0 in value.counters
        ],
        "stages": [to_json_trace_stage_snapshot(item_0) for item_0 in value.stages],
        "times": [to_json_trace_time_snapshot(item_0) for item_0 in value.times],
        "artifacts": [
            to_json_artifact_attempt_snapshot(item_0) for item_0 in value.artifacts
        ],
    }


def from_json_trace_snapshot(value: Json) -> TraceSnapshot:
    """Return one TraceSnapshot from one JSON value."""
    object_ = json_object(value)

    return TraceSnapshot(
        total_micros=json_int(json_field(object_, "totalMicros")),
        workers=json_int(json_field(object_, "workers")),
        stats=from_json_trace_stats(json_field(object_, "stats")),
        spans=[
            from_json_trace_span_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "spans"))
        ],
        counters=[
            from_json_trace_counter_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "counters"))
        ],
        stages=[
            from_json_trace_stage_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "stages"))
        ],
        times=[
            from_json_trace_time_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "times"))
        ],
        artifacts=[
            from_json_artifact_attempt_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "artifacts"))
        ],
    )


"""Trace detail returned to a caller."""
TraceView: typing.TypeAlias = typing.Literal["summary"] | typing.Literal["detailed"]


def encode_trace_view(writer: BinaryWriter, value: TraceView) -> None:
    """Encode one TraceView."""
    if value == "summary":
        writer.write_unsigned(0)
    elif value == "detailed":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_trace_view(reader: BinaryReader) -> TraceView:
    """Decode one TraceView."""
    variant = reader.read_number()

    if variant == 0:
        return "summary"
    elif variant == 1:
        return "detailed"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_trace_view(value: TraceView) -> Json:
    """Return one JSON value for one TraceView."""
    return value


def from_json_trace_view(value: Json) -> TraceView:
    """Return one TraceView from one JSON value."""
    variant = json_string(value)

    if variant == "summary":
        return "summary"
    elif variant == "detailed":
        return "detailed"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TraceStats:
    """Artifact attempt outcome counts in one trace."""

    # attempts that produced an artifact
    built: int
    # attempts served from memory
    memory_cached: int
    # attempts restored from the persistent store
    store_cached: int
    # attempts parked on missing requirements
    parked: int
    # attempts that failed
    failed: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_stats(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceStats:
        """Decode one TraceStats."""
        return decode_trace_stats(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_stats(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceStats:
        """Return one TraceStats from one JSON value."""
        return from_json_trace_stats(value)


def encode_trace_stats(writer: BinaryWriter, value: TraceStats) -> None:
    """Encode one TraceStats."""
    writer.write_unsigned(value.built)
    writer.write_unsigned(value.memory_cached)
    writer.write_unsigned(value.store_cached)
    writer.write_unsigned(value.parked)
    writer.write_unsigned(value.failed)


def decode_trace_stats(reader: BinaryReader) -> TraceStats:
    """Decode one TraceStats."""
    built = reader.read_number()
    memory_cached = reader.read_number()
    store_cached = reader.read_number()
    parked = reader.read_number()
    failed = reader.read_number()

    return TraceStats(
        built=built,
        memory_cached=memory_cached,
        store_cached=store_cached,
        parked=parked,
        failed=failed,
    )


def to_json_trace_stats(value: TraceStats) -> Json:
    """Return one JSON value for one TraceStats."""
    return {
        "built": value.built,
        "memoryCached": value.memory_cached,
        "storeCached": value.store_cached,
        "parked": value.parked,
        "failed": value.failed,
    }


def from_json_trace_stats(value: Json) -> TraceStats:
    """Return one TraceStats from one JSON value."""
    object_ = json_object(value)

    return TraceStats(
        built=json_int(json_field(object_, "built")),
        memory_cached=json_int(json_field(object_, "memoryCached")),
        store_cached=json_int(json_field(object_, "storeCached")),
        parked=json_int(json_field(object_, "parked")),
        failed=json_int(json_field(object_, "failed")),
    )


@dataclass(frozen=True, slots=True)
class TraceStageSnapshot:
    """Busy time of one toolchain stage."""

    # the stage display name
    name: str
    # the summed attempt time in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_stage_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceStageSnapshot:
        """Decode one TraceStageSnapshot."""
        return decode_trace_stage_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_stage_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceStageSnapshot:
        """Return one TraceStageSnapshot from one JSON value."""
        return from_json_trace_stage_snapshot(value)


def encode_trace_stage_snapshot(
    writer: BinaryWriter, value: TraceStageSnapshot
) -> None:
    """Encode one TraceStageSnapshot."""
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_stage_snapshot(reader: BinaryReader) -> TraceStageSnapshot:
    """Decode one TraceStageSnapshot."""
    name = reader.read_string()
    micros = reader.read_number()

    return TraceStageSnapshot(
        name=name,
        micros=micros,
    )


def to_json_trace_stage_snapshot(value: TraceStageSnapshot) -> Json:
    """Return one JSON value for one TraceStageSnapshot."""
    return {
        "name": value.name,
        "micros": value.micros,
    }


def from_json_trace_stage_snapshot(value: Json) -> TraceStageSnapshot:
    """Return one TraceStageSnapshot from one JSON value."""
    object_ = json_object(value)

    return TraceStageSnapshot(
        name=json_string(json_field(object_, "name")),
        micros=json_int(json_field(object_, "micros")),
    )


@dataclass(frozen=True, slots=True)
class TraceTimeSnapshot:
    """Summed time of one named trace span."""

    # the span name
    name: str
    # the summed span time in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_time_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTimeSnapshot:
        """Decode one TraceTimeSnapshot."""
        return decode_trace_time_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_time_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceTimeSnapshot:
        """Return one TraceTimeSnapshot from one JSON value."""
        return from_json_trace_time_snapshot(value)


def encode_trace_time_snapshot(writer: BinaryWriter, value: TraceTimeSnapshot) -> None:
    """Encode one TraceTimeSnapshot."""
    writer.write_string(value.name)
    writer.write_unsigned(value.micros)


def decode_trace_time_snapshot(reader: BinaryReader) -> TraceTimeSnapshot:
    """Decode one TraceTimeSnapshot."""
    name = reader.read_string()
    micros = reader.read_number()

    return TraceTimeSnapshot(
        name=name,
        micros=micros,
    )


def to_json_trace_time_snapshot(value: TraceTimeSnapshot) -> Json:
    """Return one JSON value for one TraceTimeSnapshot."""
    return {
        "name": value.name,
        "micros": value.micros,
    }


def from_json_trace_time_snapshot(value: Json) -> TraceTimeSnapshot:
    """Return one TraceTimeSnapshot from one JSON value."""
    object_ = json_object(value)

    return TraceTimeSnapshot(
        name=json_string(json_field(object_, "name")),
        micros=json_int(json_field(object_, "micros")),
    )


@dataclass(frozen=True, slots=True)
class TraceSpanSnapshot:
    """One span in a trace snapshot."""

    # the span name
    name: str
    # the offset from the trace start in microseconds
    start_micros: int
    # the span duration in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_span_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceSpanSnapshot:
        """Decode one TraceSpanSnapshot."""
        return decode_trace_span_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_span_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceSpanSnapshot:
        """Return one TraceSpanSnapshot from one JSON value."""
        return from_json_trace_span_snapshot(value)


def encode_trace_span_snapshot(writer: BinaryWriter, value: TraceSpanSnapshot) -> None:
    """Encode one TraceSpanSnapshot."""
    writer.write_string(value.name)
    writer.write_unsigned(value.start_micros)
    writer.write_unsigned(value.micros)


def decode_trace_span_snapshot(reader: BinaryReader) -> TraceSpanSnapshot:
    """Decode one TraceSpanSnapshot."""
    name = reader.read_string()
    start_micros = reader.read_number()
    micros = reader.read_number()

    return TraceSpanSnapshot(
        name=name,
        start_micros=start_micros,
        micros=micros,
    )


def to_json_trace_span_snapshot(value: TraceSpanSnapshot) -> Json:
    """Return one JSON value for one TraceSpanSnapshot."""
    return {
        "name": value.name,
        "startMicros": value.start_micros,
        "micros": value.micros,
    }


def from_json_trace_span_snapshot(value: Json) -> TraceSpanSnapshot:
    """Return one TraceSpanSnapshot from one JSON value."""
    object_ = json_object(value)

    return TraceSpanSnapshot(
        name=json_string(json_field(object_, "name")),
        start_micros=json_int(json_field(object_, "startMicros")),
        micros=json_int(json_field(object_, "micros")),
    )


@dataclass(frozen=True, slots=True)
class TraceCounterSnapshot:
    """One counter in a trace snapshot."""

    # the counter name
    name: str
    # the counter value
    value: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_counter_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceCounterSnapshot:
        """Decode one TraceCounterSnapshot."""
        return decode_trace_counter_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_counter_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceCounterSnapshot:
        """Return one TraceCounterSnapshot from one JSON value."""
        return from_json_trace_counter_snapshot(value)


def encode_trace_counter_snapshot(
    writer: BinaryWriter, value: TraceCounterSnapshot
) -> None:
    """Encode one TraceCounterSnapshot."""
    writer.write_string(value.name)
    writer.write_unsigned(value.value)


def decode_trace_counter_snapshot(reader: BinaryReader) -> TraceCounterSnapshot:
    """Decode one TraceCounterSnapshot."""
    name = reader.read_string()
    value_ = reader.read_number()

    return TraceCounterSnapshot(
        name=name,
        value=value_,
    )


def to_json_trace_counter_snapshot(value: TraceCounterSnapshot) -> Json:
    """Return one JSON value for one TraceCounterSnapshot."""
    return {
        "name": value.name,
        "value": value.value,
    }


def from_json_trace_counter_snapshot(value: Json) -> TraceCounterSnapshot:
    """Return one TraceCounterSnapshot from one JSON value."""
    object_ = json_object(value)

    return TraceCounterSnapshot(
        name=json_string(json_field(object_, "name")),
        value=json_int(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class ArtifactAttemptSnapshot:
    """One artifact attempt in a detailed trace snapshot."""

    # the artifact kind name
    name: str
    # the toolchain stage display name
    stage: str
    # the resolved artifact label, usually a module display name
    label: str | None
    # the resolved target name for emitted and linked artifacts
    target: str | None
    # the worker that executed the attempt
    worker: int
    # the offset from the trace start in microseconds
    start_micros: int
    # the attempt duration in microseconds
    micros: int
    # the attempt outcome name
    outcome: str
    # interior spans recorded by the executor or provider
    spans: Sequence[TraceSpanSnapshot]
    # counters recorded by the executor or provider
    counters: Sequence[TraceCounterSnapshot]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_attempt_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactAttemptSnapshot:
        """Decode one ArtifactAttemptSnapshot."""
        return decode_artifact_attempt_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_attempt_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactAttemptSnapshot:
        """Return one ArtifactAttemptSnapshot from one JSON value."""
        return from_json_artifact_attempt_snapshot(value)


def encode_artifact_attempt_snapshot(
    writer: BinaryWriter, value: ArtifactAttemptSnapshot
) -> None:
    """Encode one ArtifactAttemptSnapshot."""
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
    for item_value_spans_0 in value.spans:
        encode_trace_span_snapshot(writer, item_value_spans_0)
    writer.write_unsigned(len(value.counters))
    for item_value_counters_0 in value.counters:
        encode_trace_counter_snapshot(writer, item_value_counters_0)


def decode_artifact_attempt_snapshot(reader: BinaryReader) -> ArtifactAttemptSnapshot:
    """Decode one ArtifactAttemptSnapshot."""
    name = reader.read_string()
    stage = reader.read_string()
    label = reader.read_option(lambda: reader.read_string())
    target = reader.read_option(lambda: reader.read_string())
    worker = reader.read_number()
    start_micros = reader.read_number()
    micros = reader.read_number()
    outcome = reader.read_string()
    spans = [decode_trace_span_snapshot(reader) for _ in range(reader.read_number())]
    counters = [
        decode_trace_counter_snapshot(reader) for _ in range(reader.read_number())
    ]

    return ArtifactAttemptSnapshot(
        name=name,
        stage=stage,
        label=label,
        target=target,
        worker=worker,
        start_micros=start_micros,
        micros=micros,
        outcome=outcome,
        spans=spans,
        counters=counters,
    )


def to_json_artifact_attempt_snapshot(value: ArtifactAttemptSnapshot) -> Json:
    """Return one JSON value for one ArtifactAttemptSnapshot."""
    return {
        "name": value.name,
        "stage": value.stage,
        **({} if value.label is None else {"label": value.label}),
        **({} if value.target is None else {"target": value.target}),
        "worker": value.worker,
        "startMicros": value.start_micros,
        "micros": value.micros,
        "outcome": value.outcome,
        "spans": [to_json_trace_span_snapshot(item_0) for item_0 in value.spans],
        "counters": [
            to_json_trace_counter_snapshot(item_0) for item_0 in value.counters
        ],
    }


def from_json_artifact_attempt_snapshot(value: Json) -> ArtifactAttemptSnapshot:
    """Return one ArtifactAttemptSnapshot from one JSON value."""
    object_ = json_object(value)

    return ArtifactAttemptSnapshot(
        name=json_string(json_field(object_, "name")),
        stage=json_string(json_field(object_, "stage")),
        label=json_optional(object_, "label", lambda value: json_string(value)),
        target=json_optional(object_, "target", lambda value: json_string(value)),
        worker=json_int(json_field(object_, "worker")),
        start_micros=json_int(json_field(object_, "startMicros")),
        micros=json_int(json_field(object_, "micros")),
        outcome=json_string(json_field(object_, "outcome")),
        spans=[
            from_json_trace_span_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "spans"))
        ],
        counters=[
            from_json_trace_counter_snapshot(item_0)
            for item_0 in json_array(json_field(object_, "counters"))
        ],
    )


__all__ = [
    "TraceSnapshot",
    "encode_trace_snapshot",
    "decode_trace_snapshot",
    "to_json_trace_snapshot",
    "from_json_trace_snapshot",
    "TraceView",
    "encode_trace_view",
    "decode_trace_view",
    "to_json_trace_view",
    "from_json_trace_view",
    "TraceStats",
    "encode_trace_stats",
    "decode_trace_stats",
    "to_json_trace_stats",
    "from_json_trace_stats",
    "TraceStageSnapshot",
    "encode_trace_stage_snapshot",
    "decode_trace_stage_snapshot",
    "to_json_trace_stage_snapshot",
    "from_json_trace_stage_snapshot",
    "TraceTimeSnapshot",
    "encode_trace_time_snapshot",
    "decode_trace_time_snapshot",
    "to_json_trace_time_snapshot",
    "from_json_trace_time_snapshot",
    "TraceSpanSnapshot",
    "encode_trace_span_snapshot",
    "decode_trace_span_snapshot",
    "to_json_trace_span_snapshot",
    "from_json_trace_span_snapshot",
    "TraceCounterSnapshot",
    "encode_trace_counter_snapshot",
    "decode_trace_counter_snapshot",
    "to_json_trace_counter_snapshot",
    "from_json_trace_counter_snapshot",
    "ArtifactAttemptSnapshot",
    "encode_artifact_attempt_snapshot",
    "decode_artifact_attempt_snapshot",
    "to_json_artifact_attempt_snapshot",
    "from_json_artifact_attempt_snapshot",
]
