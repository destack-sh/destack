import type { ObjectReference, Subject } from "@destack/sync";
import type { Audience } from "../object/announcement.ts";
import type { Reason } from "../object/subscription.ts";

/** What a notifier posts about a source. */
export type Notice<Payload> = {
    /** The object it is about. */
    readonly source: ObjectReference;
    /** The payload. */
    readonly payload: Payload;
    /** The identity a later notice with the same key replaces. */
    readonly key?: string;
    /** The thread, the source's identifier by default. */
    readonly thread?: string;
} & (
    | {
          /** The principals it notifies, at most 100. */
          readonly recipients: readonly Subject[];
          /** Why they receive it. */
          readonly reason: Reason;
      }
    | {
          /** The principals it notifies, and why. */
          readonly audience: Audience;
          /** The principals it skips besides the caller, such as those it told already. */
          readonly excluded?: readonly Subject[];
      }
);
