import { SeverityNumber, telemetry, trace } from "@destack/telemetry";
import { domainFailure, ServiceError } from "../error/index.ts";
import { DatabaseError } from "@destack/db/error";
import type {} from "@destack/package/import-meta";

/** Service failure instrumentation. */
const instruments = telemetry.scope(import.meta.destack.package);

/** Record unexpected failures and return an error safe to send to clients. */
export function reportError(error: unknown): ServiceError<string, unknown> {
    // let clients retry requests that lost to a concurrent change of the same records
    if (error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE") {
        return new ServiceError("CONFLICT", { message: error.message });
    }

    // pass declared client failures on unchanged
    if (error instanceof ServiceError && error.status < 500) {
        return error;
    }

    // report domain failures, such as access denials and duplicates, as the failures they mean
    const failure = domainFailure(error);
    if (failure !== undefined) {
        return failure;
    }

    // record unexpected failures in the active trace and service log
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

    // retain explicitly declared failures and conceal unexpected exception details
    if (error instanceof ServiceError) {
        return error;
    }

    return new ServiceError("INTERNAL_SERVER_ERROR", {
        message: "internal server error",
        cause: error,
    });
}
