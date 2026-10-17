import type { Duration } from "@destack/schema";

/** How a store delivers notifications. */
export interface NotificationOptions {
    /** The recipients of one announcement batch, 100 by default. */
    readonly batch?: number;
    /** The keys each controller reconciles at once, 16 by default. */
    readonly concurrency?: number;
    /** How long a push service stores a message for an offline browser, a day by default. */
    readonly pushTtl?: Duration;
    /** How long an email waits for the notification to be read elsewhere, 15 minutes by default. */
    readonly emailDelay?: Duration;
}
