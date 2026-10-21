import { CallError } from "../error/call.ts";
import { type Outcome } from "@destack/sync";
import { Failure } from "@destack/error";

/** The statuses of failures a retry replays instead of running again. */
const FINAL_STATUSES = new Set([400, 403, 404, 409, 412, 422]);

/** Decide which outcomes a retry of a request replays instead of running the call again. */
export const Replay = {
    /** Describe a final failure as the outcome a retry replays. */
    failure(error: unknown): { readonly kind: "failure"; readonly error: Failure } | undefined {
        const failure = CallError.of(error);
        const outcome =
            failure === undefined
                ? undefined
                : ({ kind: "failure", error: Failure.of(failure) } as const);

        return Replay.isFinal(outcome) ? outcome : undefined;
    },

    /** Report whether a retry replays an outcome instead of running the call again. */
    isFinal(outcome: Outcome | undefined): boolean {
        return (
            outcome !== undefined &&
            (outcome.kind === "success" ||
                (outcome.kind !== "cancelled" &&
                    FINAL_STATUSES.has(Failure.status(outcome.error)) &&
                    outcome.error.code !== "INSUFFICIENT_GRANT"))
        );
    },
};
