import { context, extractContext } from "@destack/telemetry";
import { OpenAPIHandler, type OpenAPIHandlerOptions } from "@orpc/openapi/fetch";
import {
    isProcedure,
    isLazy,
    type AnyRouter,
    type Lazyable,
    type Context,
    type Router,
} from "@orpc/server";
import type { ServiceRouter } from "../service/index.ts";
import { ProcedureAccess } from "../procedure/procedure.ts";
import type { Health } from "../health/health.ts";
import { invokeProcedure, type ProcedureCall, type ProcedureAudit } from "./access.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";
import { SmartCoercionPlugin } from "@orpc/json-schema";
import { schemaConverter } from "../openapi/document.ts";

/** Implement service procedures with typed context and middleware. */
export { implement } from "@orpc/server";

/** Dispatch Fetch requests to service procedures using their HTTP routes. */
export class ServiceHandler<State extends Context> extends OpenAPIHandler<State> {
    /** Readiness shared with the hosting lifecycle. */
    readonly health: Health;

    /** Configure HTTP handling and extract trace context for each request. */
    constructor(router: Router<ServiceRouter, State>, options: HandlerOptions<State>) {
        // reject missing enforcement before serving any request
        ServiceHandler.#checkAccess(router, options, new Set());
        const telemetry = new ServiceTelemetry("server");

        // report failures and establish trace context before application interceptors
        super(router, {
            ...options,
            plugins: [
                new SmartCoercionPlugin({ schemaConverters: [schemaConverter] }),
                ...(options.plugins ?? []),
            ],
            clientInterceptors: [
                ({ path, next }) => telemetry.invoke(path, next),
                async ({ next, procedure, path, input, context, signal }) => {
                    // evaluate declared access before entering application middleware
                    const access = ProcedureAccess.parse(procedure["~orpc"].meta);
                    const call: ProcedureCall<State> = { access, path, input, context, signal };

                    return invokeProcedure(call, next, options);
                },
                ...(options.clientInterceptors ?? []),
            ],

            adapterInterceptors: [
                ({ request, next }) => context.with(extractContext(request.headers), next),
                ...(options.adapterInterceptors ?? []),
            ],
        });
        this.health = options.health;
    }

    /** Answer probes through the same HTTP path on every host. */
    override async handle(
        ...args: Parameters<OpenAPIHandler<State>["handle"]>
    ): ReturnType<OpenAPIHandler<State>["handle"]> {
        const response = this.health.probe(args[0]);
        if (response) {
            return { matched: true, response };
        }

        return super.handle(...args);
    }

    /** Require enforcement for every declared procedure before hosting. */
    static #checkAccess<State extends Context>(
        router: Lazyable<AnyRouter>,
        options: Pick<HandlerOptions<State>, "authorize" | "audit">,
        ancestors: Set<object>,
    ): void {
        // reject deferred or recursive routers before entering the HTTP adapter
        if (isLazy(router)) {
            throw new TypeError("service procedures must be declared before hosting");
        }
        if (ancestors.has(router)) {
            throw new TypeError("service routers must not contain cycles");
        }

        // require the callbacks declared by each procedure
        if (isProcedure(router)) {
            const access = ProcedureAccess.parse(router["~orpc"].meta);
            if (
                (access.authentication !== "public" || access.permission !== null) &&
                !options.authorize
            ) {
                throw new TypeError("protected procedures require authorization");
            }
            if (access.audit && !options.audit) {
                throw new TypeError("audited procedures require audit recording");
            }
        }
        // inspect each nested router while permitting reuse at independent addresses
        else {
            ancestors.add(router);
            for (const child of Object.values(router)) {
                ServiceHandler.#checkAccess(child, options, ancestors);
            }
            ancestors.delete(router);
        }
    }
}

/** HTTP transport settings and required host enforcement. */
export interface HandlerOptions<State extends Context> extends OpenAPIHandlerOptions<State> {
    /** Readiness shared with the host. */
    health: Health;
    /** Require authenticated request context and enforce the procedure's declared access. */
    authorize?(call: ProcedureCall<State>): Promise<void>;
    /** Persist required audit events before acknowledging their completion. */
    audit?(event: ProcedureAudit<State>): Promise<void>;
}
export type { Context, Middleware, Router } from "@orpc/server";
export type { FetchHandleResult as HandleResult } from "@orpc/server/fetch";
