import { DateTime, type ToRelativeOptions } from "luxon";
import { onMounted, onUnmounted, ref, type Ref } from "vue";

export function useNow(updateInterval = 60000) {
  /**
   * Gets a computed reference to 'now' DateTime, updated every updateInterval.
   */
  const now: Ref<DateTime> = ref(DateTime.now());

  let interval: NodeJS.Timer;
  onMounted(() => {
    interval = setInterval(() => (now.value = DateTime.now()), updateInterval);
  });
  onUnmounted(() => clearInterval(interval));

  return now;
}
const defaultLuxonOptions: ToRelativeOptions = {
  locale: "en-US",
  style: "narrow",
  unit: ["years", "months", "weeks", "days", "hours", "minutes"],
};

export function useTimeFromNow(updateInterval = 60000, luxonOptions = defaultLuxonOptions) {
  const now = useNow(updateInterval);
  function getTimeFromNow(dt: DateTime): string | null {
    if (now.value.diff(dt, "seconds").seconds < 60) {
      return "just now";
    } else {
      return dt.toRelative({ ...luxonOptions, base: now.value });
    }
  }

  function getTimeFromNowString(dt: string): string | null {
    return getTimeFromNow(DateTime.fromISO(dt));
  }

  return { now, getTimeFromNow, getTimeFromNowString };
}
