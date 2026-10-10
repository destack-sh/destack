import { telemetry, trace } from "@destack/telemetry";
import type { Controller } from "../control/index.ts";
import { ValidationError } from "@orpc/contract";
import { describeIssues, ServiceError } from "../error/index.ts";
import type {} from "@destack/package/import-meta";

/** The instruments capturing unexpected failures. */
const instruments = telemetry.scope(import.meta.destack.package);

/** Record unexpected failures and return an error safe for clients. */
export function reportError(error: unknown): ServiceError<string, unknown> {
    // read the service error a failure is or carries
    const failure = ServiceError.of(error);

    // describe invalid input by its issues
    if (
        failure !== undefined &&
        failure.code === "BAD_REQUEST" &&
        failure.cause instanceof ValidationError
    ) {
        return new ServiceError("BAD_REQUEST", {
            message: describeIssues(failure.cause.issues),
            cause: failure.cause,
        });
    }
    // pass client failures on
    else if (failure !== undefined && failure.status < 500) {
        return failure;
    }

    // capture unexpected failures as escaping their request
    trace.getActiveSpan()?.recordException(error instanceof Error ? error : String(error));
    instruments.captureException(error, { isEscaped: true });

    // pass deliberate unavailability on and hide the rest
    if (failure !== undefined && failure.code !== "INTERNAL_SERVER_ERROR") {
        return failure;
    }

    return new ServiceError("INTERNAL_SERVER_ERROR", {
        message: "internal server error",
        cause: error,
    });
}

/** Answer a failure with its status and the body clients read errors from. */
export function refusal(error: unknown): Response {
    const reported = reportError(error);

    return Response.json(reported.toJSON(), { status: reported.status });
}

/** Capture a failed reconciliation as escaping its controller, tagged by the controller. */
export function reportReconciliation(controller: Controller, key: string, error: unknown): void {
    instruments.captureException(error, {
        isEscaped: true,
        tags: { controller: controller.name },
        attributes: { "destack.key": key },
    });
}
