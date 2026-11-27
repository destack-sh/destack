import { Duration, schema } from "@destack/schema";
import { CubicBezier } from "../bezier/index.ts";
import { type Bindable, Reference } from "../reference/index.ts";

/** How a change plays out, as the design token format's transition, timed by durations. */
const transitionJson = schema
    .object({
        /** How long it takes. */
        duration: Reference.or(Duration.forward, "duration"),
        /** How long it waits before it starts, a negative delay starting it partway, as in CSS. */
        delay: Reference.or(Duration, "duration"),
        /** How it eases. */
        timingFunction: CubicBezier.bindable,
    })
    .meta({ id: "style.transition", title: "Transition" });
/** How a change plays out. */
export type Transition = schema.Infer<typeof transitionJson>;

/** How a change plays out, as the design token format's transition. */
export const Transition = Object.assign(transitionJson, {
    /** Play out changes of some CSS properties, as CSS's `transition` writes it. */
    css(transition: Transition, properties: readonly string[]): string {
        // write the timing once, then list it for each property
        const timing = [
            time(transition.duration),
            CubicBezier.css(transition.timingFunction),
            time(transition.delay),
        ].join(" ");

        return properties.map((property) => `${property} ${timing}`).join(", ");
    },
});

/** Write a duration as CSS's time in milliseconds, such as `200ms`, a reference as its variable. */
function time(duration: Bindable<Duration>): string {
    return Reference.write(duration, (written) => `${String(Duration.milliseconds(written))}ms`);
}
