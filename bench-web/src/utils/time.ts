import type { Timestamp } from "@/proto/wire";
import { DateTime, Duration } from "luxon";
import { ref, type Ref } from "vue";

export enum TimeUpdateInterval {
  MILLISECOND = 50, // ms-ish
  SECOND = 1000,
  MINUTE = 60000,
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
};

/** Gets a reactive now, updated every updateInterval (ms).*/
export function useNow(updateInterval: TimeUpdateInterval) {
  return NOW_TRACKERS[updateInterval].now;
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
const TIME_UNITS: TimeUnit[] = ["ms", "s", "m", "h", "d", "w", "y"];

type DurationFormat =
  | "exact" // 1h, 2m, 3.1s
  | "approximate" // this hour, last month, etc.
  | "absolute"; // 2024-04-02 12:34:56

export type FormatDurationOptions = {
  format?: DurationFormat | ((duration: number) => DurationFormat);
  minUnit?: TimeUnit;
  maxUnit?: TimeUnit;
  precision?: number;
};

/** Formats a time duration into a short string. */
export function formatDuration(duration: number | Duration, options?: FormatDurationOptions): string {
  const durationMs = duration instanceof Duration ? duration.as("milliseconds") : duration;

  // eslint-disable-next-line prefer-const
  let { format = "exact", minUnit = "s", maxUnit = "y", precision = 1 } = options ?? {};
  if (typeof format == "function") format = format(durationMs);

  if (format == "exact") {
    // 1h, 2m, 3.1s
    let unit = maxUnit;
    let value = durationMs / TIME_UNIT_MILLIS[unit];
    while (value < 1 && unit != minUnit) {
      unit = TIME_UNITS[TIME_UNITS.indexOf(unit) - 1];
      value = durationMs / TIME_UNIT_MILLIS[unit];
    }
    return `${value.toFixed(precision)}${unit}`;
  } else if (format == "approximate") {
    // this hour, last month, etc.
    let unit = maxUnit;
    let value = durationMs / TIME_UNIT_MILLIS[unit];
    while (value < 1 && unit != minUnit) {
      unit = TIME_UNITS[TIME_UNITS.indexOf(unit) - 1];
      value = durationMs / TIME_UNIT_MILLIS[unit];
    }
    // shift to the next higher unit
    const unitIdx = TIME_UNITS.indexOf(unit);
    if (unitIdx < TIME_UNITS.length - 1) {
      unit = TIME_UNITS[unitIdx + 1];
      value = durationMs / TIME_UNIT_MILLIS[unit];
    }
    if (value < 1) return `this ${TIME_UNIT_NAMES[unit]}`;
    else if (value < 2) return `last ${TIME_UNIT_NAMES[unit]}`;
    else return `${value.toFixed(0)} ${TIME_UNIT_NAMES[unit]}s ago`;
  } else if (format == "absolute") {
    // Oct 2, 2024
    const dt = DateTime.now().minus(durationMs);
    return dt.toLocaleString(DateTime.DATE_MED);
  } else {
    throw new Error(`unexpected duration format: ${format}`);
  }
}

export type FormatDurationRelativeOptions = FormatDurationOptions & {
  updateInterval?: TimeUpdateInterval;
};

/** Formats a duration implied by a datetime in the past to now */
export function formatDurationFromNow(dt: Timestamp | DateTime, options?: FormatDurationRelativeOptions) {
  if (!(dt instanceof DateTime)) dt = tsToDt(dt);
  const interval = options?.updateInterval ?? TimeUpdateInterval.MINUTE;
  const now = useNow(interval);
  const duration = now.value.diff(dt, "milliseconds").as("milliseconds");

  if (interval == TimeUpdateInterval.SECOND) options = { ...options, minUnit: "s" };
  else if (interval == TimeUpdateInterval.MINUTE) options = { ...options, minUnit: "m" };

  return formatDuration(duration, options);
}

export enum DateFormat {
  DATE = "Date",
  DATE_TIME = "DateTime",
}

/** Formats an absolute datetime */
export function formatDt(dt: DateTime, options?: { format: DateFormat }) {
  const format = options?.format ?? DateFormat.DATE_TIME;
  if (format == DateFormat.DATE) return dt.toLocaleString(DateTime.DATE_MED);
  else if (format == DateFormat.DATE_TIME) return dt.toLocaleString(DateTime.DATETIME_MED);
  else throw new Error(`unexpected date format: ${format}`);
}

/**
 * Formats numbers into their highest 3-exponent of 10 (k, m, b)
 *  (like 57 -> 57, 7207 -> 7.2k, 2000000 -> 2m)
 */
export function humanizeNumber(num: number): string {
  if (num < 1000) {
    return num.toString();
  } else if (num < 1000000) {
    return `${Math.round(num / 100) / 10}k`;
  } else if (num < 1000000000) {
    return `${Math.round(num / 100000) / 10}m`;
  } else {
    return `${Math.round(num / 100000000) / 10}b`;
  }
}

/** Formats a number into bytes. */
export function humanizeBytes(bytes: number, options?: { cutoff?: number; round?: boolean }) {
  const cutoff = options?.cutoff ?? 100;
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let unit = 0;
  while (bytes >= cutoff && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  if (options?.round) {
    return `${Math.round(bytes)}${units[unit]}`;
  } else if (unit == 0) {
    return `${bytes.toFixed(0)}${units[unit]}`;
  } else if (bytes < 10) {
    return `${bytes.toFixed(1)}${units[unit]}`;
  } else {
    return `${bytes.toFixed(0)}${units[unit]}`;
  }
}

/** Convert a proto Timestamp to a Luxon DateTime */
export function tsToDt(timestamp: Timestamp): DateTime {
  return DateTime.fromSeconds(Number(timestamp.seconds), { zone: "utc" }).plus({
    milliseconds: Math.ceil(timestamp.nanos / 1000000),
  });
}

/** Convert a Luxon DateTime to a proto Timestamp */
export function dtToTs(dt: DateTime): Timestamp {
  return {
    seconds: BigInt(Math.floor(dt.toSeconds())),
    nanos: dt.millisecond * 1000000,
  };
}
