import { SeverityNumber, telemetry, trace } from "@destack/telemetry";
import type { Controller, Follower } from "../control/index.ts";
import { ServiceError } from "../error/index.ts";
import { AccessError } from "@destack/access";
import { DatabaseError } from "@destack/db/error";
import type {} from "@destack/package/import-meta";

/** The failure instruments. */
const instruments = telemetry.scope(import.meta.destack.package);

/** Record unexpected failures and return an error safe for clients. */
export function reportError(error: unknown): ServiceError<string, unknown> {
    // report a concurrent update as retriable
    if (error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE") {
        return new ServiceError("CONFLICT", { message: error.message });
    }

    // pass client failures on
    if (error instanceof ServiceError && error.status < 500) {
        return error;
    }

    // map domain failures
    const failure = domainFailure(error);
    if (failure !== undefined) {
        return failure;
    }

    // record unexpected failures
    const exception = error instanceof Error ? error : new Error(String(error));
    trace.getActiveSpan()?.recordException(exception);
    instruments.logger.emit({
        severityNumber: SeverityNumber.ERROR,
        severityText: "ERROR",
        body: "Service request failed",
        attributes: {
            "exception.type": exception.name,
            "exception.message": exception.message,
            ...(exception.stack ? { "exception.stacktrace": exception.stack } : {}),
        },
    });

    // hide unexpected exception details
    if (error instanceof ServiceError) {
        return error;
    }

    return new ServiceError("INTERNAL_SERVER_ERROR", {
        message: "internal server error",
        cause: error,
    });
}

/** Map a domain failure to a service failure. */
export function domainFailure(error: unknown): ServiceError<string, unknown> | undefined {
    // challenge for stronger authentication
    if (error instanceof AccessError && error.code === "INSUFFICIENT_AUTHENTICATION") {
        return new ServiceError(error.code, {
            status: 401,
            message: error.message,
            data: error.stepUp,
        });
    }
    // report other access decisions
    else if (error instanceof AccessError && error.code !== "INVALID_DECLARATION") {
        const code =
            error.code === "INVALID_CONTEXT"
                ? "FORBIDDEN"
                : error.code === "STALE"
                  ? "SERVICE_UNAVAILABLE"
                  : error.code;

        return new ServiceError(code, { message: error.message });
    }
    // report a duplicate key or a broken reference as a conflict
    else if (
        error instanceof DatabaseError &&
        (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE")
    ) {
        return new ServiceError("CONFLICT", { message: error.message });
    }
    // report a bad query or record as a bad request
    else if (
        error instanceof DatabaseError &&
        (error.code === "INVALID_QUERY" || error.code === "INVALID_RECORD")
    ) {
        return new ServiceError("BAD_REQUEST", { message: error.message });
    }

    return undefined;
}

/** Log a failed reconciliation. */
export function reportReconciliation(
    controller: Controller | Follower,
    key: string,
    error: unknown,
): void {
    const exception = error instanceof Error ? error : new Error(String(error));
    instruments.logger.emit({
        severityNumber: SeverityNumber.WARN,
        severityText: "WARN",
        body: "Reconciliation failed",
        attributes: {
            "destack.controller": controller.name,
            "destack.key": key,
            "exception.type": exception.name,
            "exception.message": exception.message,
            ...(exception.stack ? { "exception.stacktrace": exception.stack } : {}),
        },
    });
}
