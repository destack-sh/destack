import { schema } from "@destack/schema";

/** An error carrying a code, as service, database and package errors do. */
const CodedError = schema.looseObject({
    /** The error's code. */
    code: schema.string(),
    /** The error's message. */
    message: schema.string(),
});

/** Answer a pending call's refusal as its code and message, or "done" when it succeeds. */
export function refusal(pending: Promise<unknown>): Promise<"done" | readonly [string, string]> {
    return pending.then(
        () => "done" as const,
        (error: unknown) => {
            // read the code and message the error carries
            const coded = CodedError.safeParse(error);

            // rethrow an error without them, failing the test that awaits it
            if (!coded.success) {
                throw error;
            }

            return [coded.data.code, coded.data.message] as const;
        },
    );
}
