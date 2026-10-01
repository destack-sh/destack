import type { ObjectReference } from "@destack/sync";
import type { Reason } from "../object/subscription.ts";

/** What a notifier posts about a source. */
export interface Notice<Payload> {
    /** The object it is about. */
    readonly source: ObjectReference;
    /** Why its recipients receive it. */
    readonly reason: Reason;
    /** The payload. */
    readonly payload: Payload;
    /** The identity a later notice with the same key replaces. */
    readonly key?: string;
    /** The thread, the source's identifier by default. */
    readonly thread?: string;
}
