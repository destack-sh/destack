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
