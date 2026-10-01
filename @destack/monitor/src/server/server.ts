import { principal } from "@destack/access";
import { Scope, type ObjectReference } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service";
import {
    implement,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { installation } from "@destack/space/object";
import { AuditRecorder } from "@destack/audit";
import { entry, monitorUnmask, SENSITIVE_PREFIX } from "../access/index.ts";
import type { Entry } from "../entry/index.ts";
import { type Monitor, SegmentController } from "../monitor/index.ts";
import { monitorService } from "../service/index.ts";

/** The value readers see in place of a sensitive attribute value, as CloudWatch masks it. */
const MASK = "****";

/** The OTLP/HTTP paths below the monitor's mount, by signal. */
const OTLP_PATHS = {
    "/v1/logs": "logs",
    "/v1/traces": "traces",
    "/v1/metrics": "metrics",
} as const;

/** The access and audit a host supplies to the monitor service. */
export interface MonitorServerOptions {
    /** The host's policies. */
    readonly access: ServiceAccess;
    /** Create the recorder of unmasked reads in a scope's history. */
    record(scope: string, context: ServiceContext): Pick<AuditRecorder, "record">;
}

/** Implement the monitor service on a monitor, deciding reads by the host's policies. */
export function implementService(
    monitor: Monitor,
    options: MonitorServerOptions,
): ServiceImplementation {
    const implementation = implement(monitorService.router).$context<ServiceContext>();

    return {
        service: monitorService,
        access: options.access,
        audit: AuditRecorder.procedure(({ context }) =>
            options.record(context.scope ?? Scope.universe.id, context),
        ),
        route: (request, context) => receive(monitor, request, context),
        controllers: [new SegmentController(monitor)],
        router: implementation.router({
            series: implementation.series.handler(async ({ input, context }) => {
                // aggregate as the caller may read metrics
                const emitter = await governing(options.access, input);
                await context.authorization!.require(entry.permission("read-metrics"), emitter);

                return monitor.series(input.scope, input);
            }),
            search: implementation.search.handler(async ({ input, context }) => {
                // search as the caller may read, revealing sensitive values only to those who may unmask them
                const emitter = await governing(options.access, input);
                await context.authorization!.require(entry.permission("read-logs"), emitter);
                const reveal = await unmasking(context, options, input, emitter, "search");
                const page = await monitor.search(input.scope, input);

                return { ...page, entries: page.entries.map(reveal) };
            }),
            tail: implementation.tail.handler(async ({ input, context }) => {
                // follow as the caller may tail, revealing sensitive values only to those who may unmask them
                const emitter = await governing(options.access, input);
                await context.authorization!.require(entry.permission("tail-logs"), emitter);
                const reveal = await unmasking(context, options, input, emitter, "tail");

                return mapped(monitor.tail(input.scope, input, context.signal), reveal);
            }),
            trace: implementation.trace.handler(async ({ input, context }) => {
                // read the trace as the caller may read it, revealing sensitive values only to those who may unmask them
                const emitter = await governing(options.access, input);
                await context.authorization!.require(entry.permission("read-traces"), emitter);
                const reveal = await unmasking(context, options, input, emitter, "trace");
                const entries = await monitor.trace(input.scope, input.installation, input.trace);

                return { entries: entries.map(reveal) };
            }),
        }),
    };
}

/** Take an OTLP/HTTP export from an installation into its space's entries. */
async function receive(
    monitor: Monitor,
    request: Request,
    context: ServiceContext,
): Promise<Response | undefined> {
    // serve only the OTLP paths, by POST
    const path = new URL(request.url).pathname;
    const signal = Object.hasOwn(OTLP_PATHS, path)
        ? OTLP_PATHS[path as keyof typeof OTLP_PATHS]
        : undefined;
    if (signal === undefined) {
        return undefined;
    } else if (request.method !== "POST") {
        return new Response(null, { status: 405, headers: { allow: "POST" } });
    }

    // accept exports from installations only, stamped with the installation and its space
    const subject = context.requireAuthentication().claims.subject;
    if (!principal.installation.is(subject)) {
        throw new ServiceError("FORBIDDEN", { message: "only installations export telemetry" });
    }
    monitor.receive(
        subject.scope,
        subject.id as Identifier<"installation">,
        signal,
        await request.json(),
    );

    return Response.json({});
}

/** Decide how a read shows sensitive values: unmasked and audited for a caller who may unmask them, else masked. */
async function unmasking(
    context: ServiceContext,
    options: MonitorServerOptions,
    target: { readonly scope: string },
    emitter: ObjectReference,
    operation: "search" | "tail" | "trace",
): Promise<(found: Entry) => Entry> {
    // mask the values of a caller who may not unmask them
    const decision = await context.authorization!.check(entry.permission("unmask"), emitter);
    if (!decision.isAllowed) {
        return mask;
    }

    // record the unmasked read in the scope's history
    const targets = { emitter: { type: emitter.type, id: emitter.id } };
    await options
        .record(target.scope, context)
        .record(
            undefined,
            monitorUnmask,
            { targets, details: { operation }, outcome: { kind: "success" } },
            "access",
        );

    return (found) => found;
}

/** Replace an entry's sensitive attribute values with the mask. */
function mask(found: Entry): Entry {
    const attributes = Object.fromEntries(
        Object.entries(found.attributes).map(([key, value]) => [
            key,
            key.startsWith(SENSITIVE_PREFIX) ? MASK : value,
        ]),
    );

    return { ...found, attributes };
}

/** Transform each entry of a stream, ending it when the reader stops. */
function mapped(
    stream: AsyncIteratorObject<Entry, undefined, void>,
    transform: (found: Entry) => Entry,
): AsyncIteratorObject<Entry, undefined, void> {
    return {
        next: async () => {
            const result = await stream.next();

            return result.done === true ? result : { done: false, value: transform(result.value) };
        },
        return: () => stream.return!(),
        [Symbol.asyncIterator]() {
            return this;
        },
        [Symbol.asyncDispose]: () => stream[Symbol.asyncDispose](),
    };
}

/** Find the object deciding reads: the installation, or the scope's own object for its host's entries. */
async function governing(
    access: ServiceAccess,
    target: { readonly scope: string; readonly installation?: string | undefined },
): Promise<ObjectReference> {
    // decide on the installation, granted on it or through its space
    if (target.installation !== undefined) {
        return installation.reference(target.scope, target.installation);
    }

    // decide on the scope's own object, such as the host
    const [link] = await Scope.chain(Snapshot.live(access.database), target.scope);
    if (link === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no scope ${target.scope}` });
    }

    return link.object;
}
