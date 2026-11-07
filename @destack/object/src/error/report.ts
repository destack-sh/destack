import type { Environment } from "@destack/environment";
import { recordSpanException, TelemetryScope, trace } from "@destack/telemetry";

import { ValidationError } from "@orpc/contract";
import { CallError } from "./call.ts";
import { RetryAfter } from "./detail.ts";
import { present } from "@destack/schema";
import { FamilyError } from "@destack/error";
import { refuseInput } from "./error.ts";

import type {} from "@destack/package/import-meta";

/** Record unexpected failures on a telemetry and return an error safe for clients. */
export function reportError(
    error: unknown,
    telemetry: Environment["telemetry"],
): CallError<string, unknown> {
    // read the call error a failure is or has
    const failure = CallError.of(error);

    // describe invalid input by its issues
    if (
        failure !== undefined &&
        failure.code === "BAD_REQUEST" &&
        failure.cause instanceof ValidationError
    ) {
        return present(
            CallError.of(refuseInput(failure.cause.data, failure.cause.issues)),
            "the call error of invalid input",
        );
    }
    // pass deliberate refusals for callers on, such as a client failure or a declared unavailability, never a host family's
    else if (
        failure !== undefined &&
        failure.code !== "INTERNAL_SERVER_ERROR" &&
        !(error instanceof FamilyError && error.audience === "host")
    ) {
        return failure;
    }

    // capture unexpected failures as escaping their request, and hide them
    const span = trace.getSpan(telemetry.context.active());
    if (span !== undefined) {
        recordSpanException(span, error);
    }
    TelemetryScope.of(import.meta.destack.package, telemetry).captureException(error, {
        isEscaped: true,
    });

    return CallError.http("INTERNAL_SERVER_ERROR", {
        message: "internal server error",
        cause: error,
    });
}

/** Answer a failure with its status, the body clients read errors from, and when to retry. */
export function refusal(error: unknown, telemetry: Environment["telemetry"]): Response {
    // answer the reported error with its status
    const reported = reportError(error, telemetry);
    const response = Response.json(reported.toJSON(), { status: reported.status });

    // set the refusal's Retry-After in whole seconds
    const wait = RetryAfter.safeParse(reported.data);
    if (wait.success) {
        response.headers.set("retry-after", String(Math.ceil(wait.data.retryAfter / 1000)));
    }

    return response;
}
