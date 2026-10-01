import { ORPCError } from "@orpc/client";

/** The client failures worth attempting again: a request timeout, a request too early, and a throttle (RFC 9110, 8470, 6585). */
export const TRANSIENT_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

export { ORPCError as ServiceError };

/** Hide a denial from the caller as a missing resource and keep the denial as the cause for audit. */
export function conceal(
    denial: ORPCError<string, unknown>,
    message: string,
): ORPCError<"NOT_FOUND", unknown> {
    return new ORPCError("NOT_FOUND", { message, cause: denial });
}

/** Read the denial a failure carries: itself for a 401 or 403, the cause of a concealed one. */
export function denialOf(
    failure: ORPCError<string, unknown>,
): ORPCError<string, unknown> | undefined {
    // take a refusal of access, or the refusal a missing resource conceals
    if (failure.status === 401 || failure.status === 403) {
        return failure;
    } else if (failure.status === 404 && failure.cause instanceof ORPCError) {
        return denialOf(failure.cause as ORPCError<string, unknown>);
    }

    return undefined;
}
