import { schema } from "@destack/schema";
import { CubicBezier } from "../bezier/index.ts";
import { Duration } from "../duration/index.ts";

/** How a change plays out, as the design token format's transition. */
const transitionJson = schema
    .object({
        /** How long it takes. */
        duration: Duration,
        /** How long it waits before it starts. */
        delay: Duration,
        /** How it eases. */
        timingFunction: CubicBezier,
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
            Duration.css(transition.duration),
            CubicBezier.css(transition.timingFunction),
            Duration.css(transition.delay),
        ].join(" ");

        return properties.map((property) => `${property} ${timing}`).join(", ");
    },
});
