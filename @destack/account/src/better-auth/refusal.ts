import { CALL_ERROR_STATUSES, Failure } from "@destack/error";
import { APIError } from "better-auth/api";
import { AccountError } from "../error/error.ts";

/** Answer the account's refusals inside Better Auth as Better Auth answers its own. */
export async function refusing<Result>(work: () => Promise<Result>): Promise<Result> {
    try {
        return await work();
    } catch (error) {
        // pass on any other failure
        if (!AccountError.is(error)) {
            throw error;
        }
        const failure = Failure.of(error);

        throw new APIError(
            "BAD_REQUEST",
            { code: failure.reason, message: failure.message },
            undefined,
            CALL_ERROR_STATUSES[failure.code],
        );
    }
}
