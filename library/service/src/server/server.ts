import { context, extractContext, telemetry, trace } from "@destack/telemetry";
import { OpenAPIHandler, type OpenAPIHandlerOptions } from "@orpc/openapi/fetch";
import type { Context, Router } from "@orpc/server";
import type { Service } from "../service/index.ts";
import { ServiceError } from "../error/index.ts";
import manifest from "../../package.json" with { type: "json" };

/** Instruments shared by HTTP service implementations. */
const instruments = telemetry.scope(manifest);

/** Implement service procedures with typed context and middleware. */
export { implement } from "@orpc/server";

/** Dispatch Fetch requests to service procedures using their HTTP routes. */
export class ServiceHandler<T extends Context> extends OpenAPIHandler<T> {
    /** Configure HTTP handling and extract trace context for each request. */
    constructor(router: Router<Service, T>, options: OpenAPIHandlerOptions<T> = {}) {
        super(router, {
            ...options,
            clientInterceptors: [
                async ({ next }) => {
                    try {
                        return await next();
                    } catch (error) {
                        // retain expected service failures and hide unexpected exception details
                        if (error instanceof ServiceError && error.status < 500) throw error;
                        const exception = error instanceof Error ? error : new Error(String(error));
                        trace.getActiveSpan()?.recordException(exception);
                        instruments.logger.emit({
                            severityNumber: 17,
                            severityText: "ERROR",
                            body: "Service request failed",
                            attributes: {
                                "exception.type": exception.name,
                                "exception.message": exception.message,
                                ...(exception.stack
                                    ? { "exception.stacktrace": exception.stack }
                                    : {}),
                            },
                        });
                        if (error instanceof ServiceError) throw error;

                        throw new ServiceError("INTERNAL_SERVER_ERROR", {
                            message: "Internal server error",
                            cause: error,
                        });
                    }
                },
                ...options.clientInterceptors ?? [],
            ],
            adapterInterceptors: [
                ({ request, next }) => context.with(extractContext(request.headers), next),
                ...options.adapterInterceptors ?? [],
            ],
        });
    }
}

export type { OpenAPIHandlerOptions as HandlerOptions } from "@orpc/openapi/fetch";
export type { Context, Middleware, Router } from "@orpc/server";
export type { FetchHandleResult as HandleResult } from "@orpc/server/fetch";
