import { Temporal } from "temporal-polyfill";

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

/** Parse a duration string into a Temporal.Duration. */
export function timedeltaFromISOFormat(duration: string): Temporal.Duration {
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
    const milliseconds = Math.round((totalSeconds - seconds) * 1000);
    return Temporal.Duration.from({ seconds: seconds * sign, milliseconds: milliseconds * sign });
  } catch (error) {
    throw new Error(`could not parse duration '${duration}': ${(error as any).message}`);
  }
}

/** Format a duration as an ISO string. */
export function timedeltaToISOFormat(duration: number | Temporal.Duration): string {
  if (typeof duration !== "number") {
    duration = duration.total("milliseconds");
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
export function timeOfDayToISOFormat(timeOfDay: { hours: number; minutes: number; seconds: number; nanos?: number }): string {
  const hours = timeOfDay.hours.toString().padStart(2, "0");
  const minutes = timeOfDay.minutes.toString().padStart(2, "0");
  const seconds = timeOfDay.seconds.toString().padStart(2, "0");
  const nanos = timeOfDay.nanos ? `.${timeOfDay.nanos.toString().padStart(9, "0")}` : "";
  return `${hours}:${minutes}:${seconds}${nanos}`;
}

/** Convert an ISO string to TimeOfDay. */
export function timeOfDayFromISOFormat(ISOString: string): { hours: number; minutes: number; seconds: number; nanos: number } {
  const [time, fractions] = ISOString.split(".");
  const [hours, minutes, seconds] = time.split(":").map(Number);
  const nanos = fractions ? parseInt(fractions.padEnd(9, "0")) : 0;

  return { hours, minutes, seconds, nanos };
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
export function parseDurationString(input: string): Temporal.Duration | null {
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
  return hasValues ? Temporal.Duration.from(values) : null;
}