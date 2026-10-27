import { CallSite, RequestBody } from "@destack/object";
import { AccessContext } from "@destack/access";
import type { ObjectStore, RequestContext } from "@destack/object/server";
import { RequestId } from "@destack/sync";
import { type Identifier, present, schema } from "@destack/schema";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import { action, exception, log, metric, session, span, visit } from "../object/index.ts";
import { Exception } from "../issue/exception.ts";
import { Grouper, type Grouping, type Occurrence } from "../issue/grouper.ts";
import { Visitor } from "./visitor.ts";
import { OtlpExport, type OtlpResource } from "./otlp.ts";

/** The largest OTLP/JSON export taken, the OpenTelemetry Collector's default of 20 MiB. */
const EXPORT_BYTES = 20 * 1024 * 1024;

/** The media type of OTLP/JSON exports. */
const JSON_TYPE = "application/json";

/** The event name of a log record that records an exception. */
const EXCEPTION = "exception";

/** The OTLP/HTTP signals by their paths below the adapter's. */
const SIGNALS: ReadonlyMap<string, OtlpSignal> = new Map([
    ["/traces", "traces"],
    ["/logs", "logs"],
    ["/metrics", "metrics"],
]);

/** One exception a decoded export recorded, once per trace, with the events recording it. */
interface Recording {
    /** The occurrence the grouper reads. */
    readonly occurrence: Occurrence;
    /** The events recording it, as their type and index in the type's list, such as `span:2`. */
    readonly events: readonly string[];
    /** The trace it was recorded in. */
    readonly trace: string | undefined;
}

/** Takes the OTLP exports of a space's installations and pages, or of a machine's own work. */
export class OtlpIntake {
    /** The store of the scope the exports are appended in. */
    readonly #store: ObjectStore;
    /** The grouping of a space's exceptions into issues. */
    readonly #grouper: Grouper;

    /** Append exports through a scope's store, grouping a space's exceptions into its issues. */
    constructor(store: ObjectStore) {
        this.#store = store;
        this.#grouper = new Grouper(store);
    }

    /** Answer an OTLP/HTTP export request as the machine forwarding it into the request's scope. */
    async fetch(request: Request, context: RequestContext): Promise<Response> {
        // take only posted OTLP/JSON exports of a signal
        const signal = SIGNALS.get(new URL(request.url).pathname);
        const type = request.headers.get("content-type")?.split(";")[0]?.trim().toLowerCase();
        if (signal === undefined) {
            return new Response(null, { status: 404 });
        } else if (request.method !== "POST") {
            return new Response(null, { status: 405, headers: { allow: "POST" } });
        } else if (type !== JSON_TYPE) {
            return new Response(null, { status: 415 });
        }

        // append the export as its forwarder
        const scope = present(context.scope, "the scope of an OTLP export");
        const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`, EXPORT_BYTES);
        await this.append(context.access(scope), scope, signal, body);

        return Response.json({});
    }

    /** Append an export of a signal into a scope as the machine forwarding it. */
    async append(
        caller: AccessContext,
        scope: string,
        signal: OtlpSignal,
        body: unknown,
    ): Promise<void> {
        // decode each resource's records, then build its items with their issues and visitor
        const resources = await new OtlpExport(signal, body).decode(
            scope,
            this.#store.environment.crypto,
        );
        const recordings = resources.map((resource) => this.#recorded(resource));
        const grouped = await this.#group(scope, recordings.flat());
        const items = [];
        for (const [index, resource] of resources.entries()) {
            const issues = OtlpIntake.#issues(recordings[index] ?? [], grouped.issues);
            const visitor = await this.#visit(resource);
            items.push(resource.items(visitor === undefined ? { issues } : { issues, visitor }));
        }

        // append the exceptions and the records in one mutation as the forwarder
        const appended = [
            [log, items.flatMap((each) => each.log)],
            [span, items.flatMap((each) => each.span)],
            [metric, items.flatMap((each) => each.metric)],
            [visit, items.flatMap((each) => each.visit)],
            [action, items.flatMap((each) => each.action)],
            [session, items.flatMap((each) => each.session)],
        ] as const;
        const { exceptions } = grouped;
        const records = appended
            .filter(([, events]) => events.length > 0)
            .map(([type, events]) => type.call("append", { ...type.in(scope), items: events }));
        const calls = [
            ...(exceptions.length === 0
                ? []
                : [exception.call("append", { ...exception.in(scope), items: [...exceptions] })]),
            ...records,
        ];
        if (calls.length > 0) {
            await this.#store.mutate(
                caller,
                { id: RequestId.create(this.#store.environment), calls },
                { site: CallSite.package(log.package.id) },
            );
        }
    }

    /** Group an export's exceptions into a space's issues once, across its resources. */
    async #group(scope: string, recorded: readonly Recording[]): Promise<Grouping> {
        // group the exceptions of a space's installations only
        const space = schema.identifier("space").safeParse(scope);
        if (!space.success || recorded.length === 0) {
            return { exceptions: [], issues: new Map() };
        }

        return await this.#grouper.group(
            space.data,
            recorded.map((each) => each.occurrence),
        );
    }

    /** Read the issue of each log record and span a resource's recordings name. */
    static #issues(
        recorded: readonly Recording[],
        issues: Grouping["issues"],
    ): ReadonlyMap<string, string> {
        return new Map(
            recorded.flatMap((each) => {
                const issue = issues.get(each.occurrence);

                return issue === undefined
                    ? []
                    : each.events.map((event) => [event, issue] as const);
            }),
        );
    }

    /** Hash the visitor of a space page's resource, absent for a resource without one. */
    async #visit(resource: OtlpResource): Promise<string | undefined> {
        // hash the visitor of a space's page only
        const { visitor, installation, scope } = resource.emitter;
        const space = schema.identifier("space").safeParse(scope);
        const { visit: visits, action: actions, session: sessions } = resource.events;
        if (
            !space.success ||
            visitor === undefined ||
            installation === undefined ||
            visits.length + actions.length + sessions.length === 0
        ) {
            return undefined;
        }

        // hash the visitor under the day's salt
        const now = this.#store.clock.now();

        return new Visitor(this.#store, space.data).hash({ installation, ...visitor }, now);
    }

    /** Read a resource's exceptions of installations once per trace. */
    #recorded(resource: OtlpResource): Recording[] {
        // read an exception of an installation as an occurrence
        const { build } = resource.emitter;
        const occurrence = (
            caught: Exception | undefined,
            event: {
                readonly id: string;
                readonly trace: string | null | undefined;
                readonly installationId: Identifier<"installation"> | null;
                readonly person: string | null;
            },
            time: number,
        ): Occurrence | undefined =>
            caught === undefined || event.installationId === null
                ? undefined
                : {
                      identity: event.id,
                      exception: caught,
                      installation: event.installationId,
                      build: build ?? null,
                      person: event.person,
                      time,
                  };

        // read the exception log records
        const logs = resource.events.log.flatMap((event, index): Recording[] => {
            const caught =
                event.name === EXCEPTION
                    ? Exception.read(event.attributes, event.severity)
                    : undefined;
            const found = occurrence(caught, event, event.time);

            return found === undefined
                ? []
                : [
                      {
                          occurrence: found,
                          events: [`log:${index}`],
                          trace: event.trace ?? undefined,
                      },
                  ];
        });

        // read the first exception each span annotated
        const spans = resource.events.span.flatMap((event, index): Recording[] => {
            const annotation = event.data.annotations.find((each) => each.name === EXCEPTION);
            const found =
                annotation === undefined
                    ? undefined
                    : occurrence(Exception.read(annotation.attributes, 0), event, annotation.time);

            return found === undefined
                ? []
                : [
                      {
                          occurrence: found,
                          events: [`span:${index}`],
                          trace: event.trace,
                      },
                  ];
        });

        // collapse the recordings of one exception in one trace, keeping each untraced one apart
        const collapsed = new Map<string, Recording>();
        for (const each of [...logs, ...spans]) {
            const key =
                each.trace === undefined
                    ? each.events.join()
                    : each.occurrence.exception.identity(each.trace);
            const found = collapsed.get(key);

            // merge the events, keeping the occurrence naming a person
            collapsed.set(
                key,
                found === undefined
                    ? each
                    : {
                          occurrence:
                              found.occurrence.person === null ? each.occurrence : found.occurrence,
                          events: [...found.events, ...each.events],
                          trace: found.trace,
                      },
            );
        }

        return [...collapsed.values()];
    }
}
