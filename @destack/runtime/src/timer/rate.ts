import { defineSchema, Instant, schema } from "@destack/schema";

/** The milliseconds in a second, as rates count per second. */
const MILLISECONDS = 1000;

/** The tokens a bucket held when it was last counted. */
export const RateLimitState = defineSchema(
    schema.object({
        /** The tokens left, at most the burst. */
        tokens: schema.number().nonnegative(),
        /** When the tokens were counted, in UTC epoch milliseconds. */
        at: Instant,
    }),
);
/** The tokens a bucket held when it was last counted. */
export type RateLimitState = schema.Infer<typeof RateLimitState>;

/** One request's admission. */
export interface Admission {
    /** Whether the request is admitted. */
    readonly isAdmitted: boolean;
    /** The bucket after the request, to keep for the next one. */
    readonly state: RateLimitState;
    /** The milliseconds until the next whole token, zero for an admitted request. */
    readonly wait: number;
}

/** A token bucket. */
export class RateLimit {
    /** The tokens refilled each second. */
    readonly rate: number;
    /** The most tokens a bucket has: the burst of requests it admits after a quiet spell. */
    readonly burst: number;

    /** Limit a bucket to a rate a second with a burst. */
    constructor(options: { readonly rate: number; readonly burst: number }) {
        this.rate = options.rate;
        this.burst = options.burst;
    }

    /** Take a token of a bucket kept by the caller, a full one when absent. */
    admit(state: RateLimitState | undefined, now: number): Admission {
        // refill the bucket since it was last counted
        const kept = state ?? { tokens: this.burst, at: now };
        const refilled = (Math.max(0, now - kept.at) / MILLISECONDS) * this.rate;
        const tokens = Math.min(this.burst, kept.tokens + refilled);

        // refuse a request without a whole token until one refills
        if (tokens < 1) {
            const wait = Math.ceil(((1 - tokens) / this.rate) * MILLISECONDS);

            return { isAdmitted: false, state: { tokens, at: now }, wait };
        }

        return { isAdmitted: true, state: { tokens: tokens - 1, at: now }, wait: 0 };
    }
}
