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

export type FormatDurationOptions = {
  hideMillis?: boolean;
  hideSeconds?: boolean;
  absoluteDate?: DateTime;
  absoluteDateCutoffMs?: number;
  absoluteDateFormat?: DateFormat;
};

const FORMAT_DURATION_OPTIONS_DEFAULT: FormatDurationOptions = {
  hideMillis: false,
  hideSeconds: false,
  absoluteDateCutoffMs: 1000 * 60 * 60 * 24 * 7 * 30, // 1 month
};

/** Formats a time duration into a short string. */
export function formatDuration(duration: number | Duration, options?: FormatDurationOptions): string {
  if (duration instanceof Duration) duration = duration.as("milliseconds");

  const { hideMillis, hideSeconds, absoluteDate, absoluteDateCutoffMs } = {
    ...FORMAT_DURATION_OPTIONS_DEFAULT,
    ...options,
  };

  // absolute date
  if (absoluteDateCutoffMs && absoluteDate && duration < absoluteDateCutoffMs) {
    return formatDt(absoluteDate, { format: options?.absoluteDateFormat ?? DateFormat.DATE });
  }

  // relative date
  if (duration < 100 && !hideMillis) {
    return `${Math.round(duration)}ms`;
  } else if (duration < 60 * 1000 && !hideSeconds) {
    if (hideMillis) {
      if (duration < 1000) return "<1s";
      else return `${Math.round(duration / 1000)}s`;
    } else {
      return `${(duration / 1000).toFixed(1)}s`;
    }
  } else if (duration < 60 * 60 * 1000) {
    if (hideSeconds) {
      if (duration < 60 * 1000) return "<1m";
      else return `${Math.round(duration / 60000)}m`;
    } else {
      return `${(duration / 60000).toFixed(hideSeconds ? 0 : 1)}m`;
    }
  } else if (duration < 60 * 60 * 24 * 1000) {
    return `${(duration / 3600000).toFixed(1)}h`;
  } else if (duration < 60 * 60 * 24 * 1000 * 7) {
    return `${(duration / 86400000).toFixed(1)}d`;
  } else {
    return `${(duration / 604800000).toFixed(1)}w`;
  }
}

export type FormatDurationRelativeOptions = FormatDurationOptions & {
  updateInterval?: TimeUpdateInterval;
};

const FORMAT_DURATION_RELATIVE_OPTIONS_DEFAULT: FormatDurationRelativeOptions = {
  ...FORMAT_DURATION_OPTIONS_DEFAULT,
  updateInterval: TimeUpdateInterval.MINUTE,
};

/** Formats a duration implied by a datetime in the past to now */
export function formatDurationFromNow(dt: Timestamp | DateTime, options?: FormatDurationRelativeOptions) {
  if (!(dt instanceof DateTime)) dt = tsToDt(dt);
  const interval = options?.updateInterval ?? FORMAT_DURATION_RELATIVE_OPTIONS_DEFAULT.updateInterval!;
  const now = useNow(interval);
  const duration = now.value.diff(dt, "milliseconds").as("milliseconds");

  if (interval == TimeUpdateInterval.SECOND) options = { ...options, hideMillis: true };
  else if (interval == TimeUpdateInterval.MINUTE) options = { ...options, hideMillis: true, hideSeconds: true };

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
