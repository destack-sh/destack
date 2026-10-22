import { Failure } from "@destack/error";
import { schema } from "@destack/schema";

/** How a call ended. */
export const Outcome = schema.discriminatedUnion("kind", [
    schema.object({
        /** The call succeeded. */
        kind: schema.literal("success"),
        /** The result a retry of the request replays, stored only while it may retry. */
        value: schema.json().exactOptional(),
    }),
    schema.object({
        /** The call failed, was denied or was cancelled. */
        kind: schema.enum(["failure", "denied", "cancelled"]),
        /** The failure. */
        error: Failure,
    }),
]);
/** How a call ended. */
export type Outcome = schema.Infer<typeof Outcome>;
