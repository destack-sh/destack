import { Duration } from "@destack/schema";
import { type Bindable, Reference } from "../reference/index.ts";

/** A length of time or its token, as CSS's `<time>` value type. */
export const Time = {
    /** A time or its token, negative as a delay that starts partway. */
    bindable: Reference.or(Duration, "duration"),

    /** A time or its token that runs forward, as a duration. */
    forward: Reference.or(Duration.forward, "duration"),

    /** Write a time as CSS's `<time>` in milliseconds, such as `200ms`, a token as its variable. */
    css(time: Bindable<Duration>): string {
        return Reference.write(time, (written) => `${String(Duration.milliseconds(written))}ms`);
    },
};
