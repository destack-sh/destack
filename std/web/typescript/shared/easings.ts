import { Easing } from "@destack/language";

export const EASING_FUNCTIONS: Record<Easing, (t: number) => number> = {
  [Easing.LINEAR]: (t: number) => t,
  [Easing.EASE_IN_QUAD]: (t: number) => t * t,
  [Easing.EASE_OUT_QUAD]: (t: number) => t * (2 - t),
  [Easing.EASE_IN_OUT_QUAD]: (t: number) => (t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t),
  [Easing.EASE_IN_CUBIC]: (t: number) => t * t * t,
  [Easing.EASE_OUT_CUBIC]: (t: number) => --t * t * t + 1,
  [Easing.EASE_IN_OUT_CUBIC]: (t: number) =>
    t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1,
  [Easing.EASE_IN_QUART]: (t: number) => t * t * t * t,
  [Easing.EASE_OUT_QUART]: (t: number) => 1 - --t * t * t * t,
  [Easing.EASE_IN_OUT_QUART]: (t: number) =>
    t < 0.5 ? 8 * t * t * t * t : 1 - 8 * --t * t * t * t,
  [Easing.EASE_IN_QUINT]: (t: number) => t * t * t * t * t,
  [Easing.EASE_OUT_QUINT]: (t: number) => 1 + --t * t * t * t * t,
  [Easing.EASE_IN_OUT_QUINT]: (t: number) =>
    t < 0.5 ? 16 * t * t * t * t * t : 1 + 16 * --t * t * t * t * t,
  [Easing.EASE_IN_SINE]: (t: number) => 1 - Math.cos((t * Math.PI) / 2),
  [Easing.EASE_OUT_SINE]: (t: number) => Math.sin((t * Math.PI) / 2),
  [Easing.EASE_IN_OUT_SINE]: (t: number) => -(Math.cos(Math.PI * t) - 1) / 2,
  [Easing.EASE_IN_EXPO]: (t: number) => (t <= 0 ? 0 : 2 ** (10 * t - 10)),
  [Easing.EASE_OUT_EXPO]: (t: number) => (t >= 1 ? 1 : 1 - 2 ** (-10 * t)),
  [Easing.EASE_IN_OUT_EXPO]: (t: number) =>
    t <= 0 ? 0 : t >= 1 ? 1 : t < 0.5 ? 2 ** (20 * t - 10) / 2 : (2 - 2 ** (-20 * t + 10)) / 2,
  [Easing.EASE_PEN]: (t: number) => t * 0.65 + Math.sin((t * Math.PI) / 2) * 0.35,
} as const;
