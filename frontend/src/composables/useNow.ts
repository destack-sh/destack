import { DateTime } from "luxon";
import { onMounted, onUnmounted, ref, type Ref } from "vue";

export function useNow(updateInterval = 60000) {
  /**
   * Gets a computed reference to 'now' DateTime, updated every updateInterval (ms).
   */
  const now: Ref<DateTime> = ref(DateTime.now());

  let interval: number;
  onMounted(() => {
    interval = setInterval(() => (now.value = DateTime.now()), updateInterval);
  });
  onUnmounted(() => clearInterval(interval));

  return now;
}

export function useTimeFromNow(updateInterval = 60000) {
  const now = useNow(updateInterval);
  function getTimeFromNow(dt: DateTime): string | null {
    const delta = now.value.diff(dt);
    // get relative like 2h or 6d if less than 1 week
    // get absolute if more than 1 week
    if (delta.as("days") < 7) {
      // format as 10m, 2h, 3d
      // round to nearest whole number
      const minutes = Math.round(delta.as("minutes"));
      const hours = Math.round(delta.as("hours"));
      const days = Math.round(delta.as("days"));
      if (minutes < 1) {
        return "now";
      } else if (hours < 1) {
        return `${minutes}m`;
      } else if (days < 1) {
        return `${hours}h`;
      } else {
        return `${days}d`;
      }
    } else {
      // format as e.g., Nov 4, 2021
      return dt.toLocaleString(DateTime.DATE_MED);
    }
  }

  function getTimeFromNowString(dt: string): string | null {
    return getTimeFromNow(DateTime.fromISO(dt));
  }

  return { now, getTimeFromNow, getTimeFromNowString };
}

export function formatDiffSeconds(fromStr: string, toStr: string | DateTime): string {
  // format runtime diff into smallest reasonable unit
  // like 1723.4ms -> 1.7s, 22.47ms -> 22ms, 0.0002ms -> <1ms, 72000ms -> 1.1min
  const from = DateTime.fromISO(fromStr);
  const to = typeof toStr == "string" ? DateTime.fromISO(toStr) : toStr;
  const diffMs = to.diff(from).as("milliseconds");
  if (diffMs < 1) {
    return "<1ms";
  } else if (diffMs < 10000) {
    return `${Math.round(diffMs)}ms`;
  } else if (diffMs < 60000) {
    return `${Math.round(diffMs / 100) / 10}s`;
  } else {
    return `${Math.round(diffMs / 6000) / 10}min`;
  }
}

export function humanizeNumber(num: number): string {
  // format numbers into their highest 3-exponent of 10 (k, m, b)
  // like 57 -> 57, 7207 -> 7.2k, 2000000 -> 2m
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
