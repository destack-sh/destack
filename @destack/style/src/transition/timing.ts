import { Duration, schema } from "@destack/schema";
import { CubicBezier } from "../bezier/index.ts";
import { type Bindable, Reference } from "../reference/index.ts";

/** How close a spring comes to its end to look settled, a thousandth of its travel. */
const SETTLED = 0.001;

/** The least damping a spring keeps, so even the bounciest one settles. */
const LEAST_DAMPING = 0.05;

/** The points a spring's curve takes per second, one per frame of a 60Hz screen. */
const POINTS_PER_SECOND = 60;

/** The most points a spring's curve takes, enough for a smooth curve of two seconds. */
const MOST_POINTS = 120;

/** The digits a spring's points keep, finer than a pixel of any box. */
const PRECISION = 1000;

/** The milliseconds of a second. */
const MILLISECONDS_PER_SECOND = 1000;

/** Where steps jump: at the start, the end, neither or both, as CSS's `steps()` names it. */
export const StepPosition = schema.enum(["jump-start", "jump-end", "jump-none", "jump-both"]);
/** Where steps jump. */
export type StepPosition = schema.Infer<typeof StepPosition>;

/** Steps a change jumps through, as CSS's `steps()`. */
const stepsJson = schema.object({
    /** Steps. */
    kind: schema.literal("steps"),
    /** How many steps. */
    count: schema.number().int().min(1),
    /** Where the steps jump. */
    position: StepPosition,
});
/** Steps a change jumps through. */
export type Steps = schema.Infer<typeof stepsJson>;

/** Steps a change jumps through, and how far they have jumped at a share of its time. */
export const Steps = Object.assign(stepsJson, {
    /** Read how far steps have jumped at a share of their time, as CSS's `steps()` jumps. */
    at(steps: Steps, progress: number): number {
        // count the jumps, one more where steps jump at the start, one fewer where at neither
        const jumps =
            steps.position === "jump-none"
                ? steps.count - 1
                : steps.position === "jump-both"
                  ? steps.count + 1
                  : steps.count;
        const isStart = steps.position === "jump-start" || steps.position === "jump-both";
        const step = Math.min(Math.floor(progress * steps.count) + (isStart ? 1 : 0), jumps);

        return progress >= 1 ? 1 : step / jumps;
    },
});

/** A spring, as SwiftUI and Framer Motion describe one: its duration and its bounce. */
const springJson = schema.object({
    /** A spring. */
    kind: schema.literal("spring"),
    /** How long the spring takes to look settled, its perceived duration. */
    duration: Duration.forward,
    /** How much it bounces past its end, 0 for none and 1 for the most. */
    bounce: schema.number().min(0).max(1),
});
/** A spring. */
export type Spring = schema.Infer<typeof springJson>;

/** A spring, as SwiftUI and Framer Motion describe one, and the curve CSS plays it as. */
export const Spring = Object.assign(springJson, {
    /** Read how far a spring has moved from 0 to 1 a number of seconds after it starts. */
    at(spring: Spring, seconds: number): number {
        // read its natural frequency and damping ratio from its duration and bounce
        const { frequency, damping } = motionOf(spring);
        const decay = Math.exp(-damping * frequency * seconds);

        // move as a critically damped spring at full damping, else as an underdamped one
        if (damping >= 1) {
            return 1 - decay * (1 + frequency * seconds);
        }
        const damped = frequency * Math.sqrt(1 - damping * damping);
        const ratio = (damping * frequency) / damped;

        return 1 - decay * (Math.cos(damped * seconds) + ratio * Math.sin(damped * seconds));
    },

    /** Read how many seconds a spring takes to stay within a thousandth of its end. */
    settle(spring: Spring): number {
        // bound the swing by its decaying envelope, where it falls below the settled distance
        const { frequency, damping } = motionOf(spring);
        if (damping >= 1) {
            // step through a critically damped spring, which nears its end without swinging
            let seconds = 0;
            while (Math.exp(-frequency * seconds) * (1 + frequency * seconds) > SETTLED) {
                seconds += 1 / POINTS_PER_SECOND;
            }

            return seconds;
        }
        const damped = frequency * Math.sqrt(1 - damping * damping);
        const amplitude = Math.hypot(1, (damping * frequency) / damped);

        return Math.log(amplitude / SETTLED) / (damping * frequency);
    },

    /** Write a spring as CSS's `linear()` over its settle time, a point per frame. */
    css(spring: Spring): string {
        // sample the spring evenly over its settle time, ending exactly at its end
        const seconds = Spring.settle(spring);
        const count = Math.min(Math.max(Math.ceil(seconds * POINTS_PER_SECOND), 2), MOST_POINTS);
        const points = Array.from({ length: count + 1 }, (_, index) =>
            index === count
                ? 1
                : Math.round(Spring.at(spring, (seconds * index) / count) * PRECISION) / PRECISION,
        );

        return `linear(${points.join(", ")})`;
    },
});

/** How a change eases over its time: a curve, steps, or a spring, as CSS's easing functions. */
const timingFunctionJson = schema
    .union([CubicBezier.bindable, stepsJson, springJson])
    .meta({ id: "style.timingFunction", title: "Timing function" });
/** How a change eases over its time. */
export type TimingFunction = schema.Infer<typeof timingFunctionJson>;

/** How a change eases over its time, and how CSS writes it and times it. */
export const TimingFunction = Object.assign(timingFunctionJson, {
    /** CSS's ease, which transitions and animations start from. */
    initial: [0.25, 0.1, 0.25, 1] satisfies CubicBezier,

    /** Write a timing function as CSS's easing function. */
    css(timing: TimingFunction): string {
        // write a curve or its token, then steps, then a spring
        if (Reference.is(timing) || Array.isArray(timing)) {
            return CubicBezier.css(timing);
        } else if (timing.kind === "steps") {
            return `steps(${String(timing.count)}, ${timing.position})`;
        }

        return Spring.css(timing);
    },

    /** Read how far a timing function has eased at a share of its time, its curve as drawn. */
    at(timing: Exclude<TimingFunction, string>, progress: number): number {
        // read a curve, then steps, then a spring over its settle time
        if (Array.isArray(timing)) {
            return CubicBezier.at(timing, progress);
        } else if (timing.kind === "steps") {
            return Steps.at(timing, progress);
        }

        return Spring.at(timing, progress * Spring.settle(timing));
    },

    /** Read how long CSS plays a change for: a spring's settle time, else the duration given. */
    duration(timing: TimingFunction, given: Bindable<Duration>): Bindable<Duration> {
        return !Reference.is(timing) && !Array.isArray(timing) && timing.kind === "spring"
            ? { milliseconds: Math.round(Spring.settle(timing) * MILLISECONDS_PER_SECOND) }
            : given;
    },
});

/** Read a spring's natural frequency and damping ratio, as SwiftUI derives them. */
function motionOf(spring: Spring): { readonly frequency: number; readonly damping: number } {
    return {
        frequency: (2 * Math.PI) / Duration.seconds(spring.duration),
        damping: Math.max(1 - spring.bounce, LEAST_DAMPING),
    };
}
