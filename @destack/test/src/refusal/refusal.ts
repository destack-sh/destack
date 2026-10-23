import { DomainError } from "@destack/error";
import { schema } from "@destack/schema";

/** A failure as its caller receives it: its code, its reason within its domain, and its message. */
const ReceivedFailure = schema.looseObject({
    /** The call error code. */
    code: schema.string(),
    /** The failure's stable cause within its domain. */
    reason: schema.string(),
    /** The package whose reason it is, or http. */
    domain: schema.string(),
    /** The readable message. */
    message: schema.string(),
});

/** A call's settlement: done, or refused with its code, reason and message. */
export type Refusal = "done" | readonly [code: string, reason: string, message: string];

/** Answer a pending call's or a function's refusal as its code, reason and message, or "done". */
export function refusal(pending: Promise<unknown> | (() => unknown)): Promise<Refusal> {
    // run a function, refusing its throw like a rejection
    const settled = typeof pending === "function" ? Promise.try(pending) : pending;

    return settled.then(
        () => "done" as const,
        (error: unknown) => {
            // read the failure the caller receives: a package error's, else the call error itself
            const received = ReceivedFailure.safeParse(
                DomainError.is(error) ? error.toCallError() : error,
            );

            // rethrow an error without one, failing the test that awaits it
            if (!received.success) {
                throw error;
            }

            return [received.data.code, received.data.reason, received.data.message] as const;
        },
    );
}
