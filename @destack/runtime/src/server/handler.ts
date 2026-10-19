import { context, extractContext } from "@destack/telemetry";
import { OpenAPIHandler, type OpenAPIHandlerOptions } from "@orpc/openapi/fetch";
import {
    type AnyProcedure,
    isProcedure,
    isLazy,
    type AnyRouter,
    type Lazyable,
    type Context,
    type Router,
} from "@orpc/server";
import type { ProcedureRouter } from "../runtime/index.ts";
import { ProcedureMeta } from "../procedure/procedure.ts";
import {
    mapValues,
    present,
    schema,
    toJsonSchema,
    Version,
    type JsonSchema,
} from "@destack/schema";
import { Expression } from "@destack/db";
import type { PackageHandle } from "../declare/package.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { CallError } from "../error/index.ts";
import type { Health } from "../health/health.ts";
import { ProcedureContext, type ProcedureAudit } from "./procedure-context.ts";
import { CallTelemetry } from "../procedure/call-telemetry.ts";
import { SmartCoercionPlugin } from "@orpc/json-schema";
import type { ConditionalSchemaConverter, JSONSchema } from "@orpc/openapi";

/** An input as a request decodes it before coercion: a JSON object. */
const JsonInput = schema.jsonObject();

/** Read a Destack JSON Schema as oRPC's declaration of the same draft 2020-12 document. */
function toOpenApiSchema(description: JsonSchema): Exclude<JSONSchema, boolean> {
    return mapValues<JsonSchema, Exclude<JSONSchema, boolean>>(description, (entry) => entry);
}

/** Implement a package's procedures. */
export { createRouterClient, implement } from "@orpc/server";

/** Dispatch requests to a package's procedures. */
export class RequestHandler<State extends RequestState> extends OpenAPIHandler<State> {
    /** The health. */
    readonly health: Health;

    /** Create the handler. */
    constructor(router: Router<ProcedureRouter, State>, options: HandlerOptions<State>) {
        // reject missing enforcement, and collect the payloads and each procedure's requirements
        const metas = new Map<string, ProcedureMeta>();
        const payloads = new Set(
            RequestHandler.#checkAccess(router, options, [], new Set(), metas),
        );

        // report failures and extract trace context
        super(router, {
            ...options,
            plugins: [
                new SmartCoercionPlugin({
                    schemaConverters: [RequestHandler.#converter(payloads)],
                }),
                RequestHandler.#releasePlugin<State>(options.package, metas),
                ...(options.plugins ?? []),
            ],
            clientInterceptors: [
                ...RequestHandler.#callInterceptors(options, metas),
                ...(options.clientInterceptors ?? []),
            ],

            adapterInterceptors: [
                (call) => context.with(extractContext(call.request.headers), () => call.next()),
                ...(options.adapterInterceptors ?? []),
            ],
        });
        this.health = options.health;
    }

    /** Convert a procedure's payload schema for HTTP decoding once a request needs it. */
    static #converter(payloads: ReadonlySet<schema.Schema>): ConditionalSchemaConverter {
        return {
            condition: (validator) => validator instanceof schema.Schema,
            convert: (validator) => {
                // refuse a schema none of the procedures takes or answers
                if (!(validator instanceof schema.Schema) || !payloads.has(validator)) {
                    throw new TypeError("the handler converts only its procedures' payloads");
                }

                return [true, toOpenApiSchema(toJsonSchema(validator))];
            },
        };
    }

    /** Refuse unserved releases and convert earlier inputs as JSON. */
    static #releasePlugin<State extends RequestState>(
        served: PackageHandle,
        metas: ReadonlyMap<string, ProcedureMeta>,
    ): NonNullable<OpenAPIHandlerOptions<State>["plugins"]>[number] {
        return {
            order: 1,
            init: (handler) => {
                handler.clientInterceptors ??= [];
                handler.clientInterceptors.unshift((call) => {
                    // refuse unserved releases and convert earlier inputs as JSON
                    const caller = RequestHandler.#requireRelease(call.context.request, served);
                    const input = RequestHandler.#convert(
                        call.procedure,
                        RequestHandler.#meta(metas, call.path),
                        call.input,
                        caller,
                        served.version,
                    );

                    return call.next({ ...call, input });
                });
            },
        };
    }

    /** Trace each call with the caller's release, and check access before its handler. */
    static #callInterceptors<State extends RequestState>(
        options: HandlerOptions<State>,
        metas: ReadonlyMap<string, ProcedureMeta>,
    ): NonNullable<OpenAPIHandlerOptions<State>["clientInterceptors"]> {
        const telemetry = new CallTelemetry("server");

        return [
            (call) =>
                telemetry.invoke(call.path, () => call.next(), {
                    "destack.caller.version": RequestHandler.#observedRelease(call.context.request),
                }),
            async (interception) => {
                // check access before the handler
                const { convert: _convert, ...access } = RequestHandler.#meta(
                    metas,
                    interception.path,
                );
                const procedure: ProcedureContext<State> = {
                    access,
                    path: interception.path,
                    input: interception.input,
                    request: interception.context,
                    ...(interception.signal === undefined ? {} : { signal: interception.signal }),
                };

                return ProcedureContext.invoke(procedure, () => interception.next(), options);
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

    /** Read a request's release for telemetry, bounded to releases and two markers. */
    static #observedRelease(request: Request): string {
        const header = request.headers.get(VERSION_HEADER);

        return header === null ? "absent" : Version.safeParse(header).success ? header : "invalid";
    }

    /** Require a request's release to be within the releases the package serves. */
    static #requireRelease(request: Request, served: PackageHandle): Version {
        // read the caller's release
        const header = request.headers.get(VERSION_HEADER);
        const caller = header === null ? undefined : Version.safeParse(header).data;
        const { name, version, since } = served;

        // require a release
        if (header === null) {
            throw new CallError("BAD_REQUEST", {
                message: `package ${name} requires ${VERSION_HEADER}`,
            });
        }
        // refuse a header that is not a release
        else if (caller === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `invalid ${VERSION_HEADER}: ${header}`,
            });
        }
        // refuse a release newer than this one
        else if (Version.compare(caller, version) > 0) {
            throw new CallError("CONFLICT", {
                message: `package ${name} serves ${version}, but the caller uses ${caller}`,
            });
        }
        // refuse a retired release
        else if (since !== undefined && Version.compare(caller, since) < 0) {
            throw new CallError("CONFLICT", {
                message: `package ${name} no longer serves releases before ${since}`,
            });
        }

        return caller;
    }

    /** Convert an input of an earlier release through each later release's conversion. */
    static #convert(
        procedure: AnyProcedure,
        meta: ProcedureMeta,
        input: unknown,
        caller: Version,
        served: Version,
    ): unknown {
        // take an input of this release as it is
        const convert = meta.convert ?? {};
        if (Version.between(convert, caller, served).length === 0) {
            return input;
        }

        // require a decoded JSON object input and assign each release's fields in order
        const decoded = JsonInput.safeParse(input);
        if (!decoded.success) {
            throw new CallError("BAD_REQUEST", {
                message: "a converted input must be a JSON object",
            });
        }
        const converted = Expression.upgrade(convert, decoded.data, caller, served);

        // take only the fields this release declares and the earlier ones the conversions read
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

    /** Read the requirements the handler recorded for the procedure at a path of its router. */
    static #meta(
        metas: ReadonlyMap<string, ProcedureMeta>,
        path: readonly string[],
    ): ProcedureMeta {
        return present(
            metas.get(path.join(".")),
            `the requirements of procedure ${path.join(".")}`,
        );
    }

    /** Require enforcement for every procedure, recording its requirements by path. */
    static #checkAccess<State extends Context>(
        router: Lazyable<AnyRouter>,
        options: Pick<HandlerOptions<State>, "authorize" | "audit">,
        path: readonly string[],
        ancestors: Set<object>,
        metas: Map<string, ProcedureMeta>,
    ): schema.Schema[] {
        // reject lazy routers
        if (isLazy(router)) {
            throw new TypeError("procedures must be declared before hosting");
        }

        // reject a router that contains itself
        if (ancestors.has(router)) {
            throw new TypeError("routers must not contain cycles");
        }

        // require the callbacks each procedure needs
        if (isProcedure(router)) {
            const access = ProcedureMeta.of(router);
            metas.set(path.join("."), access);
            if (
                (access.authentication !== "public" || access.permission !== "anyone") &&
                !options.authorize
            ) {
                throw new TypeError("protected procedures require authorization");
            }
            if (access.audit !== false && !options.audit) {
                throw new TypeError("audited procedures require audit recording");
            }

            // collect the payloads the HTTP layer decodes
            const { inputSchema, outputSchema } = router["~orpc"];

            return [inputSchema, outputSchema].filter(
                (payload) => payload instanceof schema.Schema,
            );
        }
        // check nested routers
        else {
            ancestors.add(router);
            const described = Object.entries(router).flatMap(([name, child]) =>
                RequestHandler.#checkAccess(child, options, [...path, name], ancestors, metas),
            );
            ancestors.delete(router);

            return described;
        }
    }
}

/** The state of every call of a request handler: at least its request. */
export type RequestState = Context & { readonly request: Request };

/** The options of a request handler. */
export interface HandlerOptions<State extends Context> extends OpenAPIHandlerOptions<State> {
    /** The package the handler serves, with the release callers must use. */
    package: PackageHandle;
    /** The health. */
    health: Health;
    /** Authorize a call. */
    authorize?(call: ProcedureContext<State>): Promise<void>;
    /** Record an audit event. */
    audit?(event: ProcedureAudit<State>): Promise<void>;
}

export type { Context, Middleware, Router } from "@orpc/server";
