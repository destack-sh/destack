import { telemetry, trace } from "@destack/telemetry";
import { ValidationError } from "@orpc/contract";
import { describeIssues, CallError } from "../error/call.ts";
import type {} from "@destack/package/import-meta";

/** The instruments capturing unexpected failures. */
const instruments = telemetry.scope(import.meta.destack.package);

/** Record unexpected failures and return an error safe for clients. */
export function reportError(error: unknown): CallError<string, unknown> {
    // read the call error a failure is or has
    const failure = CallError.of(error);

    // describe invalid input by its issues
    if (
        failure !== undefined &&
        failure.code === "BAD_REQUEST" &&
        failure.cause instanceof ValidationError
    ) {
        return new CallError("BAD_REQUEST", {
            message: describeIssues(failure.cause.issues),
            cause: failure.cause,
        });
    }
    // pass deliberate refusals on, such as a client failure or a declared unavailability
    else if (failure !== undefined && failure.code !== "INTERNAL_SERVER_ERROR") {
        return failure;
    }

    // capture unexpected failures as escaping their request, and hide them
    trace.getActiveSpan()?.recordException(error instanceof Error ? error : String(error));
    instruments.captureException(error, { isEscaped: true });

    return new CallError("INTERNAL_SERVER_ERROR", {
        message: "internal server error",
        cause: error,
    });
}

/** Answer a failure with its status and the body clients read errors from. */
export function refusal(error: unknown): Response {
    const reported = reportError(error);

    return Response.json(reported.toJSON(), { status: reported.status });
}
