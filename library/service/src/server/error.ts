import { telemetry, trace } from "@destack/telemetry";
import { ServiceError } from "../error/index.ts";
import manifest from "../../package.json" with { type: "json" };

/** Service failure instrumentation. */
const instruments = telemetry.scope(manifest);

/** Record unexpected failures and return an error safe to send to clients. */
export function reportError(error: unknown): ServiceError<string, unknown> {
    if (error instanceof ServiceError && error.status < 500) {
        return error;
    }

    // record unexpected failures in the active trace and service log
    const exception = error instanceof Error ? error : new Error(String(error));
    trace.getActiveSpan()?.recordException(exception);
    instruments.logger.emit({
        severityNumber: 17,
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
        message: "Internal server error",
        cause: error,
    });
}
