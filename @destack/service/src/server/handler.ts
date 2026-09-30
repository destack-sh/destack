import { context, extractContext } from "@destack/telemetry";
import { OpenAPIHandler, type OpenAPIHandlerOptions } from "@orpc/openapi/fetch";
import {
    isProcedure,
    isLazy,
    type AnyProcedure,
    type AnyRouter,
    type Lazyable,
    type Context,
    type Router,
} from "@orpc/server";
import type { ServiceRouter } from "../service/index.ts";
import { ProcedureMeta } from "../procedure/procedure.ts";
import { Expression } from "@destack/schema/expression";
import type { JsonValue } from "@destack/db";
import { Version } from "@destack/schema";
import type { Service } from "../declare/service.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { ServiceError } from "../error/index.ts";
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
export class ServiceHandler<State extends ServiceState> extends OpenAPIHandler<State> {
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
                ({ path, next, context }) =>
                    telemetry.invoke(path, next, {
                        "destack.caller.version": ServiceHandler.#observedRelease(context.request),
                    }),
                (call) => {
                    // refuse releases the service does not serve, then convert earlier inputs
                    const caller = ServiceHandler.#requireRelease(
                        call.context.request,
                        options.service,
                    );
                    const input = ServiceHandler.#convert(
                        call.procedure,
                        call.input,
                        caller,
                        options.service.package.version,
                    );

                    return call.next({ ...call, input });
                },
                async ({ next, procedure, path, input, context, signal }) => {
                    // check access before the handler
                    const { convert: _convert, ...access } = ProcedureMeta.of(procedure);
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

    /** Read the release a request speaks for telemetry, bounded to releases and two markers. */
    static #observedRelease(request: Request): string {
        const header = request.headers.get(VERSION_HEADER);

        return header === null ? "absent" : Version.safeParse(header).success ? header : "invalid";
    }

    /** Require a request's release to lie within the releases the service serves. */
    static #requireRelease(request: Request, service: Service): Version {
        // read the release the caller speaks
        const header = request.headers.get(VERSION_HEADER);
        const caller = header === null ? undefined : Version.safeParse(header).data;
        const served = service.package.version;

        // require a release
        if (header === null) {
            throw new ServiceError("BAD_REQUEST", {
                message: `service ${service.name} requires ${VERSION_HEADER}`,
            });
        }
        // refuse a header that is no release
        else if (caller === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `invalid ${VERSION_HEADER}: ${header}`,
            });
        }
        // refuse a release newer than this one
        else if (Version.compare(caller, served) > 0) {
            throw new ServiceError("CONFLICT", {
                message: `service ${service.name} serves ${served}, the caller speaks ${caller}`,
            });
        }
        // refuse a retired release
        else if (service.since !== undefined && Version.compare(caller, service.since) < 0) {
            throw new ServiceError("CONFLICT", {
                message: `service ${service.name} no longer serves releases before ${service.since}`,
            });
        }

        return caller;
    }

    /** Convert an input of an earlier release through each later release's conversion, dropping the fields this release no longer declares. */
    static #convert(
        procedure: AnyProcedure,
        input: unknown,
        caller: Version,
        served: Version,
    ): unknown {
        // keep an input of this release as it is
        const convert = ProcedureMeta.of(procedure).convert ?? {};
        if (Version.between(Object.keys(convert), caller, served).length === 0) {
            return input;
        }

        // require an object input, and assign each release's fields in order
        if (typeof input !== "object" || input === null || Array.isArray(input)) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a converted input must be an object",
            });
        }
        const converted = Expression.upgrade(
            convert,
            input as Readonly<Record<string, JsonValue>>,
            caller,
            served,
        );

        // keep only the fields this release declares, the earlier ones the conversions read
        const shape = (procedure["~orpc"].inputSchema as { readonly shape?: object } | undefined)
            ?.shape;

        return shape === undefined
            ? converted
            : Object.fromEntries(Object.entries(converted).filter(([field]) => field in shape));
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
            const access = ProcedureMeta.of(router);
            if (
                (access.authentication !== "public" || access.permission !== null) &&
                !options.authorize
            ) {
                throw new TypeError("protected procedures require authorization");
            }
            if (access.audit !== false && !options.audit) {
                throw new TypeError("audited procedures require audit recording");
            }

            // require payloads the HTTP layer describes and refuse others at start
            const { inputSchema, outputSchema } = router["~orpc"];
            for (const described of [inputSchema, outputSchema]) {
                if (described instanceof schema.Schema) {
                    toJsonSchema(described);
                }
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

/** The state every call of a service handler carries: at least its request. */
export type ServiceState = Context & { readonly request: Request };

/** The options of a service handler. */
export interface HandlerOptions<State extends Context> extends OpenAPIHandlerOptions<State> {
    /** The service the handler serves, whose release callers must speak. */
    service: Service;
    /** The health. */
    health: Health;
    /** Authorize a call. */
    authorize?(call: ProcedureCall<State>): Promise<void>;
    /** Record an audit event. */
    audit?(event: ProcedureAudit<State>): Promise<void>;
}

export type { Context, Middleware, Router } from "@orpc/server";
