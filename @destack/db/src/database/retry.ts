import { classifyError, DatabaseError } from "../error/error.ts";

/** How often a transaction runs at most before a lost serialization race answers as unavailable. */
const TRANSACTION_ATTEMPTS = 5;

/** The backoff before the second attempt, in milliseconds, doubled for each later one and jittered. */
const BACKOFF_MILLISECONDS = 10;

/** Run a transaction again while it loses a serialization race to a concurrent one, with jittered exponential backoff, answering unavailable once the attempts run out. */
export async function retried<Value>(
    run: () => Promise<Value>,
    signal: AbortSignal | undefined,
): Promise<Value> {
    for (let attempt = 1; ; attempt++) {
        try {
            return await run();
        } catch (error) {
            // rethrow any other failure, and a lost race once the attempts run out
            const classified = classifyError(error);
            if (!isLostRace(classified) || attempt >= TRANSACTION_ATTEMPTS) {
                throw classified;
            }

            // wait a jittered, doubling while before the next attempt
            const ceiling = BACKOFF_MILLISECONDS * 2 ** (attempt - 1);
            await new Promise((resolve) => {
                setTimeout(resolve, Math.random() * ceiling);
            });
            signal?.throwIfAborted();
        }
    }
}

/** Report whether a failure is a lost serialization race: a serialization failure, a deadlock or a busy database. */
function isLostRace(error: unknown): boolean {
    return error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE";
}
