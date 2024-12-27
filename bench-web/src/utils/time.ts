import type { TimeOfDay, Timestamp } from "@/proto/wire";
import { DateTime, Duration } from "luxon";
import { ref, type Ref } from "vue";
import { Duration as ProtoDuration } from "@/proto/wire/google/protobuf/duration";

export enum TimeUpdateInterval {
  MILLISECOND = 100, // ms-ish
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

const TIME_UNIT_FORMATS: Record<TimeUnit, { short: string; regular: string; long: string }> = {
  ms: { short: "ms", regular: "msec", long: "millisecond" },
  s: { short: "s", regular: "sec", long: "second" },
  m: { short: "m", regular: "min", long: "minute" },
  h: { short: "h", regular: "hr", long: "hour" },
  d: { short: "d", regular: "day", long: "day" },
  w: { short: "w", regular: "wk", long: "week" },
  y: { short: "y", regular: "yr", long: "year" },
};

const TIME_UNITS_SHORT: TimeUnit[] = ["ms", "s", "m", "h", "d", "w", "y"];
const DIGITS_PER_UNIT: Partial<Record<TimeUnit, number>> = {
  ms: 2,
  s: 2,
  m: 1,
  h: 1,
  d: 1,
  w: 1,
  y: 1,
};

export type FormatDurationOptions = {
  minUnit?: TimeUnit;
  minValue?: number;
  tooSmall?: string;
  maxUnit?: TimeUnit;
  format?: "short" | "regular" | "long";
  digits?: number;
  extended?: boolean;
};

/**
 * Formats a duration into a human readable string.
 * Like:
 * - short: 3.7s, 48m, 2d, 1w, 3y (rounded to nearest unit)
 * - regular: 1 sec 30 msec, 48 min, 2 days 4 hrs, 1 wk 2 days, 3 yr 6 mo (all non-zero parts)
 * - long: 1 second 30 milliseconds, 48 minutes, 2 days 4 hours, 1 week 2 days, 3 years 6 months (all non-zero parts)
 */
export function formatDuration(duration: number | ProtoDuration | Duration, options?: FormatDurationOptions): string {
  const { minUnit = "ms", maxUnit = "y", minValue, tooSmall = "now", format = "short", extended = false } = options ?? {};
  let durationMs: number;
  if (duration instanceof Duration) {
    durationMs = duration.as("milliseconds");
  } else if (typeof duration == "number") {
    durationMs = duration;
  } else {
    durationMs = Number(duration.seconds) * 1000 + duration.nanos / 1e6;
  }

  // if using condensed format, find largest unit that fits and round to it
  if (format === "short" && !extended) {
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
    const digits = options?.digits ?? DIGITS_PER_UNIT[currentUnit];
    if (digits != null) {
      const numDigits = Math.round(unitValue).toString().length;
      const precision = Math.max(0, digits - numDigits);
      roundedValue = unitValue.toFixed(precision);
    } else {
      roundedValue = unitValue.toFixed(0);
    }

    const unitFormat = TIME_UNIT_FORMATS[currentUnit];
    return `${roundedValue}${unitFormat.short}`;
  }

  // for extended format, build up all non-zero parts
  const parts: string[] = [];
  let remaining = durationMs;

  for (let i = TIME_UNITS_SHORT.indexOf(maxUnit); i >= TIME_UNITS_SHORT.indexOf(minUnit); i--) {
    const unit = TIME_UNITS_SHORT[i];
    const unitMs = TIME_UNIT_MILLIS[unit];
    const value = Math.floor(remaining / unitMs);
    if (value > 0) {
      const unitFormat = TIME_UNIT_FORMATS[unit];
      if (format === "short") {
        parts.push(`${value}${unitFormat.short}`);
      } else {
        const unitStr = format === "regular" ? unitFormat.regular : unitFormat.long;
        parts.push(`${value} ${value === 1 ? unitStr : unitStr + "s"}`);
      }
      remaining = remaining % unitMs;
    }
  }

  if (parts.length === 0) {
    return tooSmall;
  } else if (format === "short") {
    return parts.join("");
  } else if (format === "regular") {
    return parts.join(" ");
  } else {
    return parts.join(", ");
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
  const diff = Math.abs(now.diff(dt, "days").as("days"));
  const yesterday = now.minus({ days: 1 });

  if (dt.day == now.day && dt.month == now.month && dt.year == now.year) {
    // if it's today, say "Today at <time>"
    return `Today at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (dt.day == yesterday.day && dt.month == yesterday.month && dt.year == yesterday.year) {
    // if it's yesterday, say "Yesterday at <time>"
    return `Yesterday at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (diff <= 6 && dt.year == now.year) {
    // if it's within the last week in same year, say "<weekday> at <time>"
    return `${dt.toFormat("cccc")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (diff <= 364 && dt.year == now.year) {
    // if it's within the last year in same year, say "<month> <day> at <time>"
    return `${dt.toFormat("LLL d")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else if (diff <= 6) {
    // if it's within the last week but different year
    return `${dt.toFormat("cccc, LLL d, yyyy")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
  } else {
    // Different year or more than a year ago
    return `${dt.toFormat("LLL d, yyyy")} at ${dt.toLocaleString(DateTime.TIME_SIMPLE)}`;
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

//
// Timedelta parsing/formatting
//

type DurationPart = [string, string, number | null, boolean];
type DurationParts = DurationPart[];
type DurationUnits = [string, number][];

function parseISODate(segment: string): DurationParts {
  const parts: DurationParts = [];
  switch (segment.length) {
    case 8:
      if (segment[4] === "-") {
        parts.push([segment.slice(0, 4), "years", null, true]);
        parts.push([segment.slice(5, 8), "days", 366, true]);
        return parts;
      }
      parts.push([segment.slice(0, 4), "years", null, true]);
      parts.push([segment.slice(4, 7), "days", 366, true]);
      return parts;
    case 10:
      if (segment[4] === "-" && segment[7] === "-") {
        parts.push([segment.slice(0, 4), "years", null, true]);
        parts.push([segment.slice(5, 7), "months", 12, true]);
        parts.push([segment.slice(8, 10), "days", 31, true]);
        return parts;
      }
      break;
  }
  throw new Error(`unable to parse '${segment}' into date parts`);
}

function parseISOTime(segment: string): DurationParts {
  const parts: DurationParts = [];
  switch (segment.length) {
    case 8:
      if (segment[2] === ":" && segment[5] === ":") {
        parts.push([segment.slice(0, 2), "hours", 24, true]);
        parts.push([segment.slice(3, 5), "minutes", 60, true]);
        parts.push([segment.slice(6, 8), "seconds", 60, true]);
        return parts;
      }
      break;
    case 15:
      if (segment[2] === ":" && segment[5] === ":" && segment[8] === ".") {
        parts.push([segment.slice(0, 2), "hours", 24, true]);
        parts.push([segment.slice(3, 5), "minutes", 60, true]);
        parts.push([segment.slice(6), "seconds", 60, false]);
        return parts;
      }
      break;
  }
  throw new Error(`unable to parse '${segment}' into time parts`);
}

function parseISODesignators(duration: string): DurationParts {
  const parts: DurationParts = [];
  const dateContext: [string, string][] = [
    ["Y", "years"],
    ["M", "months"],
    ["W", "weeks"],
    ["D", "days"],
  ];
  let context: [string, string][] | null = dateContext;
  let value = "";
  let unit: string | null = null;

  for (const char of duration) {
    if (char.match(/[\d.,]/)) {
      value += char;
      continue;
    }

    if (char === "T" && context === dateContext) {
      if (value !== "") throw new Error(`missing unit designator after '${value}'`);
      context = [
        ["H", "hours"],
        ["M", "minutes"],
        ["S", "seconds"],
      ];
      continue;
    }

    if (char === "W") {
      if (unit !== null) throw new Error("cannot mix weeks with other units");
      parts.push([value, "weeks", null, false]);
      value = "";
      continue;
    }

    if (!context) throw new Error(`unexpected character '${char}'`);

    for (const [delimiter, _unit] of context) {
      if (char === delimiter) {
        parts.push([value, _unit, null, false]);
        value = "";
        unit = _unit;
        break;
      }
    }
  }
  if (parts.length === 0) throw new Error("no units found");
  return parts;
}

function parseISODuration(duration: string): DurationParts {
  if (!duration.startsWith("P")) {
    throw new Error("durations must begin with the character 'P'");
  }

  const parts: DurationParts = [];
  if (/[A-Z]$/.test(duration)) {
    return parseISODesignators(duration.slice(1));
  } else {
    const [dateSegment, timeSegment] = duration.slice(1).split("T");
    if (dateSegment) parts.push(...parseISODate(dateSegment));
    if (timeSegment) parts.push(...parseISOTime(timeSegment));
    return parts;
  }
}

function toISOUnits(parts: DurationParts): DurationUnits {
  const units: DurationUnits = [];
  for (const [value, unit, limit, integerOnly] of parts) {
    if (!((integerOnly && /^\d+$/.test(value)) || /^\d+\.?\d*$/.test(value))) {
      throw new Error(`unable to parse '${value}' as a positive number`);
    }
    const quantity = parseFloat(value);
    if (limit === null) {
      if (quantity < 0) {
        throw new Error(`${unit} value of ${value} exceeds range [0..+∞)`);
      }
    } else if (limit === 24 || limit === 60) {
      if (quantity < 0 || quantity >= limit) {
        throw new Error(`${unit} value of ${value} exceeds range [0..${limit})`);
      }
    } else {
      if (quantity < 0 || quantity > limit) {
        throw new Error(`${unit} value of ${value} exceeds range [0..${limit}]`);
      }
    }
    if (quantity) {
      units.push([unit, quantity]);
    }
  }
  return units;
}

/** Parse a duration string into a ProtoDuration. */
export function timedeltaFromISOFormat(duration: string): ProtoDuration {
  try {
    let sign = 1;
    if (duration.startsWith("-")) {
      sign = -1;
      duration = duration.slice(1);
    }
    const parts = parseISODuration(duration);
    const units = toISOUnits(parts);
    let totalSeconds = 0;
    for (const [unit, quantity] of units) {
      switch (unit) {
        case "weeks":
          totalSeconds += quantity * 7 * 24 * 60 * 60;
          break;
        case "days":
          totalSeconds += quantity * 24 * 60 * 60;
          break;
        case "hours":
          totalSeconds += quantity * 60 * 60;
          break;
        case "minutes":
          totalSeconds += quantity * 60;
          break;
        case "seconds":
          totalSeconds += quantity;
          break;
        default:
          throw new Error(`unexpected unit '${unit}'`);
      }
    }
    const seconds = Math.floor(totalSeconds);
    const nanos = Math.round((totalSeconds - seconds) * 1e9); // round to account for floating point errors
    return { seconds: BigInt(seconds * sign), nanos: nanos * sign };
  } catch (error) {
    throw new Error(`could not parse duration '${duration}': ${(error as any).message}`);
  }
}

/** Format a duration as an ISO string. */
export function timedeltaToISOFormat(duration: number | ProtoDuration): string {
  if (typeof duration !== "number") {
    duration = Number(duration.seconds) * 1e3 + duration.nanos / 1e6;
  }

  if (duration === 0) {
    return "P0D";
  }

  let sign = "";
  if (duration < 0) {
    sign = "-";
    duration = -duration;
  }

  let totalSeconds = duration / 1000;

  let days = Math.floor(totalSeconds / (24 * 60 * 60));
  totalSeconds %= 24 * 60 * 60;
  const weeks = Math.floor(days / 7);
  days = days % 7;
  const hours = Math.floor(totalSeconds / 3600);
  totalSeconds %= 3600;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  let result = `${sign}P`;
  if (weeks > 0) {
    result += `${weeks}W`;
  }
  if (days > 0) {
    result += `${days}D`;
  }
  if (hours > 0 || minutes > 0 || seconds > 0) {
    result += "T";
    if (hours > 0) {
      result += `${hours}H`;
    }
    if (minutes > 0) {
      result += `${minutes}M`;
    }
    if (seconds > 0) {
      result += `${seconds.toFixed(9).replace(/\.0+$/, "")}S`;
    }
  }
  return result;
}

/** Convert a TimeOfDay to an ISO string. */
export function timeOfDayToISOFormat(timeOfDay: TimeOfDay): string {
  const hours = timeOfDay.hours.toString().padStart(2, "0");
  const minutes = timeOfDay.minutes.toString().padStart(2, "0");
  const seconds = timeOfDay.seconds.toString().padStart(2, "0");
  const nanos = timeOfDay.nanos ? `.${timeOfDay.nanos.toString().padStart(9, "0")}` : "";
  return `${hours}:${minutes}:${seconds}${nanos}`;
}

/** Convert an ISO string to TimeOfDay. */
export function timeOfDayFromISOFormat(ISOString: string): TimeOfDay {
  const [time, fractions] = ISOString.split(".");
  const [hours, minutes, seconds] = time.split(":").map(Number);
  const nanos = fractions ? parseInt(fractions.padEnd(9, "0")) : 0;

  return { hours, minutes, seconds, nanos };
}

/** Converts a duration to milliseconds. */
export function durationToMs(duration: ProtoDuration | Duration): number {
  let durationMs: number;
  if (duration instanceof Duration) {
    durationMs = duration.as("milliseconds");
  } else {
    durationMs = Number(duration.seconds) * 1000 + duration.nanos / 1e6;
  }
  return durationMs;
}

/** Converts a timestamp to milliseconds. */
export function timestampToMs(timestamp: Timestamp | DateTime): number {
  let timestampMs: number;
  if (timestamp instanceof DateTime) {
    timestampMs = timestamp.toMillis();
  } else {
    timestampMs = Number(timestamp.seconds) * 1000 + timestamp.nanos / 1e6;
  }
  return timestampMs;
}

/** Compare two timestamps. */
export function compareTimestamps(a: Timestamp | DateTime, b: Timestamp | DateTime): number {
  return timestampToMs(a) - timestampToMs(b);
}

/** Map of unit variations to standardized unit names */
export const DURATION_UNIT_MAP = {
  y: "years",
  yr: "years",
  yrs: "years",
  year: "years",
  years: "years",
  mo: "months",
  mon: "months",
  month: "months",
  months: "months",
  w: "weeks",
  wk: "weeks",
  wks: "weeks",
  week: "weeks",
  weeks: "weeks",
  d: "days",
  day: "days",
  days: "days",
  h: "hours",
  hr: "hours",
  hrs: "hours",
  hour: "hours",
  hours: "hours",
  m: "minutes",
  min: "minutes",
  mins: "minutes",
  minute: "minutes",
  minutes: "minutes",
  s: "seconds",
  sec: "seconds",
  secs: "seconds",
  second: "seconds",
  seconds: "seconds",
  // additional fun variations
  mth: "months",
  mths: "months",
  dy: "days",
  mn: "minutes",
  sc: "seconds",
} as const;

export type DurationUnit = keyof typeof DURATION_UNIT_MAP;

/** Parse duration string like "1y 2mo 3d 4h 5m 6s" with generous formatting flexibility. */
export function parseDurationString(input: string): Duration | null {
  // normalize input by removing extra whitespace, commas, and making case-insensitive
  const normalized = input
    .toLowerCase()
    .trim()
    .replace(/[,\s]+/g, " ") // handle multiple commas/spaces
    .replace(/and/g, " ") // allow "1 hour and 30 minutes"
    .replace(/\+/g, " "); // allow "1h+30m"

  if (!normalized) return null;

  // match duration parts with flexible unit names
  const durationRegex = /(-?\d*\.?\d+)\s*([a-z]+)/g;
  const matches = normalized.matchAll(durationRegex);
  if (!matches) return null;

  const values: Record<string, number> = {
    years: 0,
    months: 0,
    weeks: 0,
    days: 0,
    hours: 0,
    minutes: 0,
    seconds: 0,
  };

  for (const match of matches) {
    const [_, numStr, unitStr] = match;
    if (!numStr || !unitStr) {
      continue;
    }
    const value = parseFloat(numStr);
    if (isNaN(value)) {
      continue;
    }
    const unit = DURATION_UNIT_MAP[unitStr as DurationUnit];
    if (unit) {
      values[unit] += value; // Add to existing value to handle duplicates
    }
  }

  // only create duration if we found valid values
  const hasValues = Object.values(values).some((v) => v !== 0);
  return hasValues ? Duration.fromObject(values) : null;
}
