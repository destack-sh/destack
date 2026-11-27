import { schema } from "@destack/schema";
import { Time } from "./time.ts";
import { TimingFunction } from "./timing.ts";

/** How a change plays out, as the design token format's transition, timed by durations. */
const transitionJson = schema
    .object({
        /** How long it takes. */
        duration: Time.forward,
        /** How long it waits before it starts, a negative delay starting it partway, as in CSS. */
        delay: Time.bindable,
        /** How it eases: a curve, steps or a spring. */
        timingFunction: TimingFunction,
    })
    .meta({ id: "style.transition", title: "Transition" });
/** How a change plays out. */
export type Transition = schema.Infer<typeof transitionJson>;

/** How a change plays out, as the design token format's transition. */
export const Transition = Object.assign(transitionJson, {
    /** Play out changes of some CSS properties, as CSS's `transition` writes it. */
    css(transition: Transition, properties: readonly string[]): string {
        // write the timing once, then list it for each property
        const timing = Transition.timing(transition, Time.css(transition.delay));

        return properties.map((property) => `${property} ${timing}`).join(", ");
    },

    /** Write a transition's duration, easing and a given delay, in the order animations share. */
    timing(transition: Transition, delay: string): string {
        return [
            Time.css(TimingFunction.duration(transition.timingFunction, transition.duration)),
            TimingFunction.css(transition.timingFunction),
            delay,
        ].join(" ");
    },
});
