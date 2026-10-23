import { motion } from "@destack/theme/tokens.json";

/** The theme's motion durations in milliseconds, for timing that script sets. */
export const duration = {
    /** Small changes such as hover and press. */
    short: motion.duration.short.$value.value,
    /** Elements entering and leaving. */
    medium: motion.duration.medium.$value.value,
    /** Large surfaces and full-screen changes. */
    long: motion.duration.long.$value.value,
};

/** The theme's easings as CSS cubic-bezier curves, for animations that script starts. */
export const easing = {
    /** Movement within the screen. */
    standard: curveOf(motion.easing.standard.$value),
    /** Elements entering the screen. */
    emphasised: curveOf(motion.easing.emphasised.$value),
};

/** The milliseconds between groups entering or leaving: a quarter of a short change. */
export const stagger = duration.short / 4;

/** Write a cubic Bézier curve's control points as a CSS easing. */
function curveOf(points: readonly number[]) {
    return `cubic-bezier(${points.join(", ")})`;
}
