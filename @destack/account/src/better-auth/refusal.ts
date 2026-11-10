import { CALL_ERROR_STATUSES, CallErrorCode, Failure, FailureReason } from "@destack/error";
import { schema } from "@destack/schema";
import { APIError } from "better-auth/api";
import { AccountError } from "../error/error.ts";

/** The domain of Better Auth's own refusal codes, such as INVALID_OTP. */
const BETTER_AUTH_DOMAIN = "better-auth";

/** A refusal's body as Better Auth answers it: its code and message, or an account's failure. */
const AnsweredRefusal = schema.looseObject({
    code: schema.string().exactOptional(),
    message: schema.string().exactOptional(),
});

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
            { ...failure },
            undefined,
            CALL_ERROR_STATUSES[failure.code],
        );
    }
}

/** Better Auth's refusals, read as the call errors Destack's callers receive. */
export const BetterAuthRefusal = {
    /** Answer a refused response with its call error, passing on every other response. */
    async failure(response: Response): Promise<Response> {
        // pass on a success or redirect, and a refusal that is no JSON
        const type = response.headers.get("content-type");
        if (response.status < 400 || type?.startsWith("application/json") !== true) {
            return response;
        }

        // keep an account's failure, and read Better Auth's code as its reason
        const body: unknown = await response.json();
        const declared = Failure.safeParse(body);
        const answered = AnsweredRefusal.safeParse(body);
        const code = CallErrorCode.of(response.status);
        const reason = answered.success ? answered.data.code : undefined;
        const message =
            (answered.success ? answered.data.message : undefined) ?? response.statusText;
        const failure = declared.success
            ? declared.data
            : reason !== undefined && FailureReason.safeParse(reason).success
              ? { code, reason, domain: BETTER_AUTH_DOMAIN, message }
              : Failure.http(code, message);

        // answer it with the response's headers, such as its cookies
        const headers = new Headers(response.headers);
        headers.delete("content-length");

        return Response.json(failure, { status: response.status, headers });
    },
};
