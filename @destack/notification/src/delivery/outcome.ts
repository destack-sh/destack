import type { DeliveryError } from "../object/delivery.ts";

/** A channel's result for one message. */
export type Outcome =
    | { readonly outcome: "sent" }
    | { readonly outcome: "gone" }
    | { readonly outcome: "retry"; readonly after?: number; readonly error: DeliveryError }
    | { readonly outcome: "failed"; readonly error: DeliveryError };
