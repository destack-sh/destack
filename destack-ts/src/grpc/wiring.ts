import { Temporal } from "temporal-polyfill";
import { Duration } from "../proto/google/protobuf/duration";
import { Timestamp } from "../proto/google/protobuf/timestamp";

/**
 * Convert a Temporal.ZonedDateTime to a protobuf Timestamp.
 */
export function packProtoTimestamp(dt: Temporal.ZonedDateTime): Timestamp {
  const epochNanos = dt.epochNanoseconds;
  const seconds = epochNanos / 1_000_000_000n;
  const nanos = Number(epochNanos % 1_000_000_000n);

  return Timestamp.create({
    seconds,
    nanos,
  });
}

/**
 * Convert a protobuf Timestamp to a Temporal.ZonedDateTime.
 */
export function unpackProtoTimestamp(timestamp: Timestamp): Temporal.ZonedDateTime {
  const epochNanos = timestamp.seconds * 1_000_000_000n + BigInt(timestamp.nanos);
  const instant = Temporal.Instant.fromEpochNanoseconds(epochNanos);
  return instant.toZonedDateTimeISO("UTC");
}

/**
 * Convert a Temporal.Duration to a protobuf Duration.
 */
export function packProtoDuration(td: Temporal.Duration): Duration {
  const totalSeconds = td.total("seconds");
  const seconds = BigInt(Math.floor(totalSeconds));
  const nanos = Math.round((totalSeconds - Number(seconds)) * 1_000_000_000);

  return Duration.create({
    seconds,
    nanos,
  });
}

/**
 * Convert a protobuf Duration to a Temporal.Duration.
 */
export function unpackProtoDuration(duration: Duration): Temporal.Duration {
  const totalSeconds = Number(duration.seconds) + duration.nanos / 1_000_000_000;
  return Temporal.Duration.from({ seconds: totalSeconds });
}
