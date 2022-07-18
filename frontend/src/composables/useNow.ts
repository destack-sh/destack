import { DateTime } from "luxon";
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
