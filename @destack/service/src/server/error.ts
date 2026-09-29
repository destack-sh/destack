import { telemetry, trace } from "@destack/telemetry";
import type { Controller, Follower } from "../control/index.ts";
import { ValidationError } from "@orpc/contract";
import { ServiceError } from "../error/index.ts";
import { AccessError } from "@destack/access";
import { SyncError } from "@destack/sync";
import { DatabaseError } from "@destack/db/error";
import type {} from "@destack/package/import-meta";

/** The failure log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** Record unexpected failures and return an error safe for clients. */
export function reportError(error: unknown): ServiceError<string, unknown> {
    // report a concurrent update as retriable
    if (error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE") {
        return new ServiceError("CONFLICT", { message: error.message });
    }

    // describe invalid input by its issues, leaving invalid output an internal failure
    if (
        error instanceof ServiceError &&
        error.code === "BAD_REQUEST" &&
        error.cause instanceof ValidationError
    ) {
        const issues = error.cause.issues.map((issue) => {
            const path = (issue.path ?? [])
                .map((key) => (typeof key === "object" ? key.key : key))
                .join(".");

            const message = issue.message.charAt(0).toLowerCase() + issue.message.slice(1);

            return path === "" ? message : `${path}: ${message}`;
        });

        return new ServiceError("BAD_REQUEST", {
            message: `invalid input: ${issues.join("; ")}`,
            cause: error.cause,
        });
    }
    // pass client failures on
    else if (error instanceof ServiceError && error.status < 500) {
        return error;
    }

    // map domain failures
    const failure = domainFailure(error);
    if (failure !== undefined) {
        return failure;
    }

    // record unexpected failures
    trace.getActiveSpan()?.recordException(error instanceof Error ? error : String(error));
    log.error("service.request.failed", telemetry.exceptionAttributes(error));

    // pass deliberate unavailability on, and hide the details of anything unexpected
    if (error instanceof ServiceError && error.code !== "INTERNAL_SERVER_ERROR") {
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
    // report an unknown scope
    else if (error instanceof SyncError && error.code === "NOT_FOUND") {
        return new ServiceError("NOT_FOUND", { message: error.message });
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
    log.warn("controller.reconcile.failed", {
        "destack.controller": controller.name,
        "destack.key": key,
        ...telemetry.exceptionAttributes(error),
    });
}
