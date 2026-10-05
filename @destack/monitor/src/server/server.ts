import { principal } from "@destack/access";
import { Scope, type ObjectReference } from "@destack/sync";
import { Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import { ServiceError } from "@destack/service";
import {
    implement,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { installation, installationRevision } from "@destack/space/object";
import { setting } from "@destack/setting/object";
import type { ObjectType } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { CallKey } from "@destack/service/request";
import type { OpenBuild } from "@destack/space/server";
import type { Uplink } from "@destack/sync";
import { Grouper } from "../issue/group.ts";
import { entry, monitorUnmask, SENSITIVE_PREFIX } from "../access/index.ts";
import { type Entry, Otlp } from "../entry/index.ts";
import { type EntryStream, type Monitor, SegmentController } from "../monitor/index.ts";
import { monitorService } from "../service/index.ts";
import { DeclarationController } from "./declaration.ts";
import { serveIssues } from "./issue.ts";

/** The value readers see in place of a sensitive attribute value, as CloudWatch masks it. */
const MASK = "****";

/** The OTLP/HTTP paths below the monitor's mount, by signal. */
const OTLP_PATHS: ReadonlyMap<string, OtlpSignal> = new Map([
    ["/v1/logs", "logs"],
    ["/v1/traces", "traces"],
    ["/v1/metrics", "metrics"],
]);

/** The store, spaces and builds a cell serves the monitor service with. */
export interface MonitorOptions {
    /** The telemetry store the service reads and receives into, whose catalog keeps the issues beside copies of the spaces' installations and settings. */
    readonly monitor: Monitor;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The cell serving the spaces, which keys the copy of their rows. */
    readonly cell: string;
    /** The kinds of the scopes whose telemetry it keeps, such as spaces on a cell or the host on a device. */
    readonly scopes: readonly ObjectType[];
    /** The uplink to those scopes' rows, their installations and settings. */
    readonly uplink: Uplink;
    /** Open a package's build in a space, whose source maps and graph resolve its exceptions. */
    readonly openBuild: OpenBuild;
    /** Report failures grouping exceptions. */
    readonly report: (error: unknown) => void;
}

/** The monitor service with the telemetry store and the object server keeping its issues. */
export interface MonitorImplementation extends ServiceImplementation {
    /** The telemetry store the service reads and receives into. */
    readonly monitor: Monitor;
    /** The object server keeping the issues, alert rules and alerts. */
    readonly objects: ObjectServer;
}

/** Implement the monitor service on a monitor, deciding reads by the spaces' policies its catalog copies. */
export function implementMonitor(options: MonitorOptions): MonitorImplementation {
    // implement the monitor's own procedures beside its objects'
    const { monitor } = options;
    const implementation = implement(monitorService.router).$context<ServiceContext>();

    // serve the issues, alert rules and alerts of the spaces it follows, grouping exceptions into issues
    const objects: ObjectServer = new ObjectServer({
        objects: serveIssues({ monitor }),
        policies: [...options.scopes, installation, installationRevision, setting, entry],
        database: monitor.database,
        callKey: options.callKey,
        origin: { package: monitorService.package, service: monitorService.name },
        subscriber: Subscriber.of(options.uplink, () =>
            objects.source.workloadSubscriptions(`${options.cell}/${monitorService.name}`),
        ),
    });
    const grouper = new Grouper(objects, options.openBuild);

    return {
        ...objects.implement(monitorService, [
            new SegmentController(monitor, monitor.database),
            new DeclarationController(objects, options.openBuild),
        ]),
        monitor,
        objects,
        route: (request, context) =>
            receive(request, context, async (scope, signal, entries) => {
                // group the exceptions into issues, keeping the export as it came when grouping fails
                let kept = entries;
                if (signal === "logs") {
                    try {
                        kept = await grouper.group(scope, entries);
                    } catch (error) {
                        options.report(error);
                    }
                }
                monitor.ingest(scope, kept);
            }),
        router: {
            ...objects.router(),
            series: implementation.series.handler(async ({ input, context }) => {
                // aggregate as the caller may read metrics
                const emitter = await governing(objects.access, input);
                await context
                    .requireAuthorization()
                    .require(entry.permission("read-metrics"), emitter);

                return monitor.series(input.scope, input);
            }),
            search: implementation.search.handler(async ({ input, context }) => {
                // search as the caller may read, revealing sensitive values only to those who may unmask them
                const emitter = await governing(objects.access, input);
                await context
                    .requireAuthorization()
                    .require(entry.permission("read-logs"), emitter);
                const reveal = await unmasking(context, objects, input, emitter, "search");
                const page = await monitor.search(input.scope, input);

                return { ...page, entries: page.entries.map(reveal) };
            }),
            tail: implementation.tail.handler(async ({ input, context }) => {
                // follow as the caller may tail, revealing sensitive values only to those who may unmask them
                const emitter = await governing(objects.access, input);
                await context
                    .requireAuthorization()
                    .require(entry.permission("tail-logs"), emitter);
                const reveal = await unmasking(context, objects, input, emitter, "tail");

                return mapped(monitor.tail(input, context.signal), reveal);
            }),
            trace: implementation.trace.handler(async ({ input, context }) => {
                // read the trace as the caller may read it, revealing sensitive values only to those who may unmask them
                const emitter = await governing(objects.access, input);
                await context
                    .requireAuthorization()
                    .require(entry.permission("read-traces"), emitter);
                const reveal = await unmasking(context, objects, input, emitter, "trace");
                const entries = await monitor.trace(input.scope, input.installation, input.trace);

                return { entries: entries.map(reveal) };
            }),
        },
    };
}

/** Read an OTLP/HTTP export from an installation into its space's entries and pass them on. */
async function receive(
    request: Request,
    context: ServiceContext,
    received: (scope: string, signal: OtlpSignal, entries: Entry[]) => Promise<void>,
): Promise<Response | undefined> {
    // serve only the OTLP paths, by POST
    const path = new URL(request.url).pathname;
    const signal = OTLP_PATHS.get(path);
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
    const emitter = schema.identifier("installation").parse(subject.id);
    await received(subject.scope, signal, Otlp.read(signal, await request.json(), emitter));

    return Response.json({});
}

/** Decide how a read shows sensitive values: unmasked and audited for a caller who may unmask them, else masked. */
async function unmasking(
    context: ServiceContext,
    objects: Pick<ObjectServer, "audit">,
    target: { readonly scope: string },
    emitter: ObjectReference,
    operation: "search" | "tail" | "trace",
): Promise<(found: Entry) => Entry> {
    // mask the values of a caller who may not unmask them
    const decision = await context
        .requireAuthorization()
        .check(entry.permission("unmask"), emitter);
    if (!decision.isAllowed) {
        return mask;
    }

    // record the unmasked read in the scope's history
    const targets = { emitter: { type: emitter.type, id: emitter.id } };
    await objects
        .audit(target.scope, context)
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
function mapped(stream: EntryStream, transform: (found: Entry) => Entry): EntryStream {
    return {
        next: async () => {
            const result = await stream.next();

            return result.done === true ? result : { done: false, value: transform(result.value) };
        },
        return: () => stream.return(),
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
