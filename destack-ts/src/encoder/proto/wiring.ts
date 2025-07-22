import { Value as RawValueProto } from "@destack/proto";
import { Duration } from "@destack/proto/google/protobuf/duration";
import { Timestamp } from "@destack/proto/google/protobuf/timestamp";
import { Temporal } from "temporal-polyfill";

/**
 * Convert a Temporal.ZonedDateTime to a protobuf Timestamp.
 */
export function packProtoTimestamp(timestamp: Temporal.ZonedDateTime): Timestamp {
  const epochNanos = timestamp.epochNanoseconds;
  const seconds = epochNanos / 1_000_000_000n;
  const nanos = Number(epochNanos % 1_000_000_000n);

  return Timestamp.create({ seconds, nanos });
}

/**
 * Convert a protobuf Timestamp to a Temporal.ZonedDateTime.
 */
export function unpackProtoTimestamp(timestampPacked: Timestamp): Temporal.ZonedDateTime {
  const epochNanos = timestampPacked.seconds * 1_000_000_000n + BigInt(timestampPacked.nanos);
  const instant = Temporal.Instant.fromEpochNanoseconds(epochNanos);
  return instant.toZonedDateTimeISO("UTC");
}

/**
 * Convert a Temporal.Duration to a protobuf Duration.
 */
export function packProtoDuration(duration: Temporal.Duration): Duration {
  const totalSeconds = duration.total("seconds");
  const seconds = BigInt(Math.floor(totalSeconds));
  const nanos = Math.round((totalSeconds - Number(seconds)) * 1_000_000_000);

  return Duration.create({ seconds, nanos });
}

/**
 * Convert a protobuf Duration to a Temporal.Duration.
 */
export function unpackProtoDuration(durationPacked: Duration): Temporal.Duration {
  const totalSeconds = Number(durationPacked.seconds) + durationPacked.nanos / 1_000_000_000;
  return Temporal.Duration.from({ seconds: totalSeconds });
}

/**
 * Convert a protobuf Value to a TypeScript object.
 */
export function packProtoJson(json: any): RawValueProto {
  return RawValueProto.fromJson(json);
}

/**
 * Convert a protobuf Value to a TypeScript object.
 */
export function unpackProtoJson(jsonPacked: RawValueProto): any {
  return RawValueProto.toJson(jsonPacked);
}
