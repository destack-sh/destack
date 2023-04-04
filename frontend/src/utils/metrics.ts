import { useNow } from "@/composables/useNow";
import type { recordOptions } from "@sentry/replay/types/types/rrweb";
import { DateTime } from "luxon";
import { ref, type Ref } from "vue";

export const METRIC_METER_UNITS = 4;

export type MetricFamily = "clarity" | "difficulty" | "performance" | "speed";

export function toPercent(fraction?: number, alt = "??"): string {
  return fraction != null ? (fraction * 100).toFixed(0).padStart(2, "0") : alt;
}

export function toFixed(value?: number, digits = 1, alt = "??"): string {
  return value != null ? value.toFixed(digits).padStart(2, "0") : alt;
}

export function toTime(seconds?: number, alt = "??"): string {
  if (seconds == null) return alt;
  if (seconds < 10) {
    return seconds.toFixed(1).padStart(2, "0");
  } else {
    return seconds.toFixed(0);
  }
}

export function toBars(value?: number, kind: MetricFamily): number {
  if (kind == "clarity" || kind == "performance") {
    // linear percentage
    return Math.round((value ?? 0) * METRIC_METER_UNITS);
  } else if (kind == "difficulty") {
    // exponent, e.g. 11 -> 0 bars, 30 -> 1 bar
    const exponent = Math.floor(Math.log2(value ?? 0));
    return Math.min(exponent - 3, METRIC_METER_UNITS);
  } else if (kind == "speed") {
    // inverse exponent, e.g. <= 1 -> all bars, <= 2 -> -1, etc.
    const exponent = Math.floor(Math.log2(value ?? 0));
    return METRIC_METER_UNITS - exponent - 1;
  } else {
    // shouldn't happen - error?
    return -1;
  }
}

export type Metric = {
  label: string;
  description: string;
  value: number | string;
  bars: number;
  unit?: string;
  stale?: boolean;
  unavailable?: boolean;
};

export type MetricSet = {
  label: string;
  description: string;
  metrics: Metric[];
};

export function useTween(duration = 0.5) {
  /*
  Simple value tween for multiple values keyed by id
  Does not require maintaining state (other than id) by caller
  */

  const memoryLast: Ref<Record<string, number>> = ref({});
  const memoryTarget: Ref<Record<string, number>> = ref({});
  const memoryChanged: Ref<Record<string, DateTime>> = ref({});
  const now = useNow(25);

  function easeInOutQuad(t: number): number {
    return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
  }

  function _doTween(start: number, end: number, progress: number): number {
    const easedProgress = easeInOutQuad(progress);
    return start + (end - start) * easedProgress;
  }

  function tween(target: number, id: string): number {
    if (target == null) {
      return memoryLast.value[id];
    } else if (memoryLast.value[id] == null) {
      memoryLast.value[id] = target;
      memoryChanged.value[id] = now.value;
      return target;
    }

    const startValue = memoryLast.value[id] ?? target;
    if (startValue === target) {
      return target;
    }
    if (memoryTarget.value[id] != target) {
      memoryTarget.value[id] = target;
      memoryChanged.value[id] = now.value;
    }
    const lastChanged = memoryChanged.value[id] ?? now;
    const elapsed = now.value.diff(lastChanged, "seconds").seconds;
    const progress = Math.min(elapsed / duration, 1);

    console.log("tween", id, elapsed, now.value, lastChanged);
    if (progress < 1) {
      return _doTween(startValue, target, progress);
    } else {
      memoryLast.value[id] = target;
      return target;
    }
  }

  return {
    tween,
  };
}
