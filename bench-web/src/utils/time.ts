import type { Timestamp } from "@/proto/wire";
import { DateTime, Duration } from "luxon";
import { ref, type Ref } from "vue";
import { Duration as ProtoDuration } from "@/proto/wire/google/protobuf/duration";

export enum TimeUpdateInterval {
  MILLISECOND = 50, // ms-ish
  SECOND = 1000,
  MINUTE = 60000,
  HOUR = 3600000,
}

type NowTracker = {
  interval: TimeUpdateInterval;
  now: Ref<DateTime>;
};

function makeNowTracker(interval: TimeUpdateInterval): NowTracker {
  const now: Ref<DateTime> = ref(DateTime.now());

  const updateNow = () => (now.value = DateTime.now());
  setInterval(updateNow, interval);

  return { interval, now };
}

const NOW_TRACKERS: Record<TimeUpdateInterval, NowTracker> = {
  [TimeUpdateInterval.MILLISECOND]: makeNowTracker(TimeUpdateInterval.MILLISECOND),
  [TimeUpdateInterval.SECOND]: makeNowTracker(TimeUpdateInterval.SECOND),
  [TimeUpdateInterval.MINUTE]: makeNowTracker(TimeUpdateInterval.MINUTE),
  [TimeUpdateInterval.HOUR]: makeNowTracker(TimeUpdateInterval.HOUR),
};

/** Gets a reactive now, updated every updateInterval (ms).*/
export function getNow(updateInterval: TimeUpdateInterval) {
  return NOW_TRACKERS[updateInterval].now;
}

/** Gets the reactive absolute duration from now */
export function getDurationFromNow(dt: Timestamp | DateTime, options?: { updateInterval?: TimeUpdateInterval }) {
  if (!(dt instanceof DateTime)) dt = tsToDt(dt);
  const interval = options?.updateInterval ?? TimeUpdateInterval.MINUTE;
  const now = getNow(interval);
  let duration = dt.diff(now.value, "milliseconds");
  if (duration.as("milliseconds") < 0) {
    duration = duration.negate();
  }
  return duration;
}

type TimeUnit = "ms" | "s" | "m" | "h" | "d" | "w" | "y";

const TIME_UNIT_MILLIS: Record<TimeUnit, number> = {
  ms: 1,
  s: 1000 * 1,
  m: 1000 * 60,
  h: 1000 * 60 * 60,
  d: 1000 * 60 * 60 * 24,
  w: 1000 * 60 * 60 * 24 * 7,
  y: 1000 * 60 * 60 * 24 * 365.2425,
};
const TIME_UNIT_NAMES: Record<TimeUnit, string> = {
  ms: "millisecond",
  s: "second",
  m: "minute",
  h: "hour",
  d: "day",
  w: "week",
  y: "year",
};
const TIME_UNITS_SHORT: TimeUnit[] = ["ms", "s", "m", "h", "d", "w", "y"];
const DIGITS_PER_UNIT: Partial<Record<TimeUnit, number>> = {
  ms: 2,
  s: 2,
};

type FormatDurationOptions = {
  minUnit?: TimeUnit;
  minValue?: number;
  tooSmall?: string;
  maxUnit?: TimeUnit;
  short?: boolean;
};

/**
 * Formats a duration into the nearest (ideally >1, less then <1 of next available unit)
 * Like 3.7s, 48m, 2d, 1w, 3y.
 */
export function formatDuration(duration: ProtoDuration | Duration, options?: FormatDurationOptions): string {
  const { minUnit = "ms", maxUnit = "y", minValue, tooSmall = "now", short = true } = options ?? {};
  let durationMs: number;
  if (duration instanceof Duration) {
    durationMs = duration.as("milliseconds");
  } else {
    durationMs = Number(duration.seconds) * 1000 + duration.nanos / 1e6;
  }

  // find largest unit that fits
  let currentUnit: TimeUnit = minUnit;
  for (let i = TIME_UNITS_SHORT.indexOf(minUnit); i <= TIME_UNITS_SHORT.indexOf(maxUnit); i++) {
    const unit = TIME_UNITS_SHORT[i];
    if (durationMs >= TIME_UNIT_MILLIS[unit]) {
      currentUnit = unit;
    }
  }

  // if value is too small, use special string
  if (minValue != null && durationMs < TIME_UNIT_MILLIS[currentUnit] * minValue) {
    return tooSmall;
  }

  // convert/round
  let roundedValue: string;
  const unitValue = durationMs / TIME_UNIT_MILLIS[currentUnit];
  if (DIGITS_PER_UNIT[currentUnit] != null) {
    const numDigits = Math.round(unitValue).toString().length;
    const precision = Math.max(0, DIGITS_PER_UNIT[currentUnit]! - numDigits);
    roundedValue = unitValue.toFixed(precision);
  } else {
    roundedValue = unitValue.toFixed(0);
  }

  // format
  if (short) {
    return `${roundedValue}${currentUnit}`;
  } else {
    const unitName = parseFloat(roundedValue) == 1 ? TIME_UNIT_NAMES[currentUnit] : TIME_UNIT_NAMES[currentUnit] + "s";
    return `${roundedValue} ${unitName}`;
  }
}

/** Formats a duration from a date relative to now (as an absolute value) */
export function formatDurationFromNow(
  dt: Timestamp | DateTime,
  options?: FormatDurationOptions & { updateInterval?: TimeUpdateInterval },
) {
  const duration = getDurationFromNow(dt, options);
  return formatDuration(duration, options);
}

/** Formats a duration implied by a datetime in the past relative to now */
export function formatRelativeDate(
  dt: Timestamp | DateTime,
  options?: FormatDurationOptions & { updateInterval?: TimeUpdateInterval },
) {
  if (!(dt instanceof DateTime)) dt = tsToDt(dt);
  const interval = options?.updateInterval ?? TimeUpdateInterval.MINUTE;
  const now = getNow(interval);
  const duration = now.value.diff(dt, "milliseconds").as("milliseconds");

  if (interval == TimeUpdateInterval.SECOND) options = { ...options, minUnit: "s" };
  else if (interval == TimeUpdateInterval.MINUTE) options = { ...options, minUnit: "m" };

  return formatDuration(Duration.fromMillis(duration), options);
}

/** Format absolute 'duration' implied by a datetime in the past relative to now  */
export function formatAbsoluteDate(dt: Timestamp | DateTime) {
  if (!(dt instanceof DateTime)) dt = tsToDt(dt);

  const now = getNow(TimeUpdateInterval.MINUTE).value;
  const diff = now.diff(dt, "days").as("days");
  const yesterday = now.minus({ days: 1 });

  if (dt.day == now.day && dt.month == now.month) {
    // if it's today, say "Today at <time>"
    return `Today at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (dt.day == yesterday.day && dt.month == yesterday.month) {
    // if it's yesterday, say "Yesterday at <time>"
    return `Yesterday at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (diff <= 6) {
    // if it's within the last week, say "<weekday> at <time>"
    return `${dt.toFormat("cccc")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (diff <= 364) {
    // if it's within the last year, say "<month> <day> at <time>"
    return `${dt.toFormat("LLL d")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else {
    return dt.toLocaleString(DateTime.DATETIME_MED);
  }
}

/** Convert a proto Timestamp to a Luxon DateTime */
export function tsToDt(timestamp: Timestamp): DateTime {
  return DateTime.fromSeconds(Number(timestamp.seconds), { zone: "utc" })
    .plus({
      milliseconds: Math.ceil(timestamp.nanos / 1000000),
    })
    .setZone("local");
}

/** Convert a Luxon DateTime to a proto Timestamp */
export function dtToTs(dt: DateTime): Timestamp {
  return {
    seconds: BigInt(Math.floor(dt.toSeconds())),
    nanos: dt.millisecond * 1000000,
  };
}
