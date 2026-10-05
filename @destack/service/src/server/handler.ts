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
import { schema, toJsonSchema, Version, type JsonSchema } from "@destack/schema";
import { Expression } from "@destack/db";
import type { Service } from "../declare/service.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { ServiceError } from "../error/index.ts";
import type { Health } from "../health/health.ts";
import { invokeProcedure, type ProcedureCall, type ProcedureAudit } from "./access.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";
import { SmartCoercionPlugin } from "@orpc/json-schema";
import type { ConditionalSchemaConverter, JSONSchema } from "@orpc/openapi";

/** An input as a request decodes it before coercion: a JSON object. */
const JsonInput = schema.record(schema.string(), schema.json());

/** Convert Destack schemas for HTTP decoding and OpenAPI. */
const schemaConverter: ConditionalSchemaConverter = {
    condition: (validator) => validator instanceof schema.Schema,
    convert: (validator) => {
        if (!(validator instanceof schema.Schema)) {
            throw new TypeError("expected a Destack schema");
        }

        return [true, openApiSchemaOf(toJsonSchema(validator))];
    },
};

/** Read a Destack JSON Schema as oRPC's declaration of the same draft 2020-12 document. */
function openApiSchemaOf(description: JsonSchema): JSONSchema;
/**
 * Pass the document through unchanged.
 *
 * @construct Zod and oRPC declare the same JSON Schema draft 2020-12 document and differ only in how they type `$defs`.
 */
function openApiSchemaOf(description: JsonSchema): unknown {
    return description;
}

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

        // report failures and extract trace context
        super(router, {
            ...options,
            plugins: [
                new SmartCoercionPlugin({ schemaConverters: [schemaConverter] }),
                ServiceHandler.#releasePlugin<State>(options.service),
                ...(options.plugins ?? []),
            ],
            clientInterceptors: [
                ...ServiceHandler.#callInterceptors(options),
                ...(options.clientInterceptors ?? []),
            ],

            adapterInterceptors: [
                (call) => context.with(extractContext(call.request.headers), () => call.next()),
                ...(options.adapterInterceptors ?? []),
            ],
        });
        this.health = options.health;
    }

    /** Refuse unserved releases and convert earlier inputs as JSON, before coercion as the earlier plugin. */
    static #releasePlugin<State extends ServiceState>(
        service: Service,
    ): NonNullable<OpenAPIHandlerOptions<State>["plugins"]>[number] {
        return {
            order: 1,
            init: (handler) => {
                handler.clientInterceptors ??= [];
                handler.clientInterceptors.unshift((call) => {
                    // refuse unserved releases and convert earlier inputs as JSON
                    const caller = ServiceHandler.#requireRelease(call.context.request, service);
                    const input = ServiceHandler.#convert(
                        call.procedure,
                        call.input,
                        caller,
                        service.package.version,
                    );

                    return call.next({ ...call, input });
                });
            },
        };
    }

    /** Trace each call with the caller's release, and check access before its handler. */
    static #callInterceptors<State extends ServiceState>(
        options: HandlerOptions<State>,
    ): NonNullable<OpenAPIHandlerOptions<State>["clientInterceptors"]> {
        const telemetry = new ServiceTelemetry("server");

        return [
            (call) =>
                telemetry.invoke(call.path, () => call.next(), {
                    "destack.caller.version": ServiceHandler.#observedRelease(call.context.request),
                }),
            async (interception) => {
                // check access before the handler
                const { convert: _convert, ...access } = ProcedureMeta.of(interception.procedure);
                const call: ProcedureCall<State> = {
                    access,
                    path: interception.path,
                    input: interception.input,
                    context: interception.context,
                    ...(interception.signal === undefined ? {} : { signal: interception.signal }),
                };

                return invokeProcedure(call, () => interception.next(), options);
            },
        ];
    }

    /** Answer probes before dispatching. */
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
        if (Version.between(convert, caller, served).length === 0) {
            return input;
        }

        // require a decoded JSON object input and assign each release's fields in order
        const decoded = JsonInput.safeParse(input);
        if (!decoded.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a converted input must be a JSON object",
            });
        }
        const converted = Expression.upgrade(convert, decoded.data, caller, served);

        // keep only the fields this release declares, the earlier ones the conversions read
        const inputSchema: unknown = procedure["~orpc"].inputSchema;
        const shape =
            typeof inputSchema === "object" &&
            inputSchema !== null &&
            "shape" in inputSchema &&
            typeof inputSchema.shape === "object" &&
            inputSchema.shape !== null
                ? inputSchema.shape
                : undefined;

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

/** The state of every call of a service handler: at least its request. */
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
