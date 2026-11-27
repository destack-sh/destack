import { schema } from "@destack/schema";

/** A CSS time: a number of seconds or milliseconds. */
const TIME = /^(?:\d+|\d*\.\d+)(?:ms|s)$/u;

/** A CSS easing function: a keyword, or a cubic Bézier curve's four numbers. */
const EASING =
    /^(?:linear|ease|ease-in|ease-out|ease-in-out|cubic-bezier\(\s*-?[\d.]+\s*,\s*-?[\d.]+\s*,\s*-?[\d.]+\s*,\s*-?[\d.]+\s*\))$/u;

/** How a change plays out: how long it takes, how long it waits, and how it eases. */
const transitionJson = schema
    .object({
        /** How long it takes, as CSS writes a time. */
        duration: schema.string().regex(TIME, { message: "a duration is a time, such as 200ms" }),
        /** How long it waits before it starts. */
        delay: schema.string().regex(TIME, { message: "a delay is a time, such as 0ms" }),
        /** How it eases, as CSS writes an easing function. */
        easing: schema
            .string()
            .regex(EASING, { message: "an easing is a keyword or a cubic-bezier()" }),
    })
    .meta({ id: "transition", title: "Transition" });
/** How a change plays out. */
export type Transition = schema.Infer<typeof transitionJson>;

/** How a change plays out: how long it takes, how long it waits, and how it eases. */
export const Transition = Object.assign(transitionJson, {
    /** Play out changes of some CSS properties, as CSS's `transition` writes it. */
    css(transition: Transition, properties: readonly string[]): string {
        return properties
            .map((property) =>
                [property, transition.duration, transition.easing, transition.delay].join(" "),
            )
            .join(", ");
    },
});
