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

/** Answer a pending call's refusal as its code, reason and message, or "done" when it succeeds. */
export function refusal(
    pending: Promise<unknown>,
): Promise<"done" | readonly [string, string, string]> {
    return pending.then(
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
