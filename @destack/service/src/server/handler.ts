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
import { schema, toJsonSchema } from "@destack/schema";
import type { ConditionalSchemaConverter, JSONSchema } from "@orpc/openapi";

/** Convert Destack schemas for HTTP decoding and OpenAPI. */
const schemaConverter: ConditionalSchemaConverter = {
    condition: (validator) => validator instanceof schema.Schema,
    convert: (validator) => {
        if (!(validator instanceof schema.Schema)) {
            throw new TypeError("expected a Destack schema");
        }

        return [true, toJsonSchema(validator) as JSONSchema];
    },
};

/** Implement service procedures. */
export { implement } from "@orpc/server";

/** Dispatch requests to service procedures. */
export class ServiceHandler<State extends Context> extends OpenAPIHandler<State> {
    /** The health. */
    readonly health: Health;

    /** Create the handler. */
    constructor(router: Router<ServiceRouter, State>, options: HandlerOptions<State>) {
        // reject missing enforcement
        ServiceHandler.#checkAccess(router, options, new Set());
        const telemetry = new ServiceTelemetry("server");

        // report failures and extract trace context
        super(router, {
            ...options,
            plugins: [
                new SmartCoercionPlugin({ schemaConverters: [schemaConverter] }),
                ...(options.plugins ?? []),
            ],
            clientInterceptors: [
                ({ path, next }) => telemetry.invoke(path, next),
                async ({ next, procedure, path, input, context, signal }) => {
                    // check access before the handler
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

    /** Answer probes, then dispatch. */
    override async handle(
        ...args: Parameters<OpenAPIHandler<State>["handle"]>
    ): ReturnType<OpenAPIHandler<State>["handle"]> {
        // answer probes
        const response = this.health.probe(args[0]);
        if (response) {
            return { matched: true, response };
        }

        return super.handle(...args);
    }

    /** Require enforcement for every procedure. */
    static #checkAccess<State extends Context>(
        router: Lazyable<AnyRouter>,
        options: Pick<HandlerOptions<State>, "authorize" | "audit">,
        ancestors: Set<object>,
    ): void {
        // reject lazy routers
        if (isLazy(router)) {
            throw new TypeError("service procedures must be declared before hosting");
        }

        // reject a router that contains itself
        if (ancestors.has(router)) {
            throw new TypeError("service routers must not contain cycles");
        }

        // require the callbacks each procedure needs
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
        // check nested routers
        else {
            ancestors.add(router);
            for (const child of Object.values(router)) {
                ServiceHandler.#checkAccess(child, options, ancestors);
            }
            ancestors.delete(router);
        }
    }
}

/** The options of a service handler. */
export interface HandlerOptions<State extends Context> extends OpenAPIHandlerOptions<State> {
    /** The health. */
    health: Health;
    /** Authorize a call. */
    authorize?(call: ProcedureCall<State>): Promise<void>;
    /** Record an audit event. */
    audit?(event: ProcedureAudit<State>): Promise<void>;
}

export type { Context, Middleware, Router } from "@orpc/server";
