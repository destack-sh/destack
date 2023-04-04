export const METRIC_METER_UNITS = 4;

export type MetricFamily = "clarity" | "difficulty" | "performance" | "speed";

export function toPercent(value?: number, alt = "??"): string {
  return value != null ? (value * 100).toFixed(0) : alt;
}

export function toFixed(value?: number, digits = 1, alt = "??"): string {
  return value != null ? value.toFixed(digits) : alt;
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
  stale: boolean;
};

export type MetricSet = {
  label: string;
  description: string;
  metrics: Metric[];
};
