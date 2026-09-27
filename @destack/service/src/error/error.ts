import { ORPCError } from "@orpc/client";
import { AccessError } from "@destack/access";
import { DatabaseError } from "@destack/db/error";

export { ORPCError as ServiceError };

/** Report a domain failure a retry would repeat as the service failure of the same meaning: an access decision, a duplicate record or an invalid query. */
export function domainFailure(error: unknown): ORPCError<string, unknown> | undefined {
    // report access decisions with their own classification, invalid context as a denial, and stale copies as unavailable until they catch up
    if (error instanceof AccessError && error.code !== "INVALID_DECLARATION") {
        const code =
            error.code === "INVALID_CONTEXT"
                ? "FORBIDDEN"
                : error.code === "STALE"
                  ? "SERVICE_UNAVAILABLE"
                  : error.code;

        return new ORPCError(code, { message: error.message });
    }
    // report a duplicate unique key as a conflict
    else if (error instanceof DatabaseError && error.code === "DUPLICATE") {
        return new ORPCError("CONFLICT", { message: error.message });
    }
    // report a query the caller shaped wrongly as a bad request
    else if (error instanceof DatabaseError && error.code === "INVALID_QUERY") {
        return new ORPCError("BAD_REQUEST", { message: error.message });
    }

    return undefined;
}
