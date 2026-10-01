import type { notification } from "../object/notification.ts";
import type { InstanceOf } from "@destack/object";
import { RetryPolicy } from "@destack/service/timer";
import type { Notification } from "../notification/notification.ts";
import type { Banner, DeliveryError, Delivery, SkipReason } from "../object/delivery.ts";
import type { Decision } from "./decide.ts";
import type { Outcome } from "./outcome.ts";

/** What one attempt makes of one delivery, as its send commits it. */
export interface Attempt {
    /** The delivery. */
    readonly id: string;
    /** Where it stands after the attempt. */
    readonly state: Delivery["state"];
    /** When it is due next, for deliveries left pending. */
    readonly dueAt?: number;
    /** Whether it waits for a summary, for deliveries left pending. */
    readonly isSummarized?: boolean;
    /** The refusals so far. */
    readonly attempts?: number;
    /** Why it was skipped. */
    readonly reason?: SkipReason;
    /** Why the channel refused it. */
    readonly error?: DeliveryError;
    /** The banner a desktop shows, for desktop deliveries sent. */
    readonly banner?: Banner;
}

/** What deliveries' attempts make of them. */
export const Attempt = {
    /** Read a sent delivery's attempt from its channel's outcome. */
    settled(row: Delivery, outcome: Outcome, now: number, retry: RetryPolicy): Attempt {
        // count refusals, retrying within the policy
        const attempts = row.attempts + 1;
        if (outcome.outcome === "sent") {
            return { id: row.id, state: "sent" };
        } else if (outcome.outcome === "gone") {
            return { id: row.id, state: "skipped", reason: "gone" };
        } else if (outcome.outcome === "retry" && RetryPolicy.isRetried(retry, attempts)) {
            const wait = outcome.after ?? RetryPolicy.interval(retry, attempts);

            return {
                id: row.id,
                state: "pending",
                dueAt: now + wait,
                attempts,
                error: outcome.error,
            };
        }

        return { id: row.id, state: "failed", attempts, error: outcome.error };
    },
};

/** One pending delivery a send decides. */
export interface Candidate {
    /** The delivery. */
    readonly row: Delivery;
    /** Its notification. */
    readonly notification: InstanceOf<typeof notification>;
    /** The notification's declaration. */
    readonly declaration: Notification;
    /** What the channel does with it now. */
    readonly decision: Decision;
}
