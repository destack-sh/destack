import { RequestBody } from "@destack/runtime/request";
import { AccessContext } from "@destack/access";
import type { JsonCondition } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { RequestId } from "@destack/runtime/request";
import type { RequestContext } from "@destack/runtime/server";
import { type Identifier, present, schema } from "@destack/schema";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import { action, log, metric, session, span, visit } from "../object/index.ts";
import { Exception } from "../issue/exception.ts";
import { Grouper, type Occurrence } from "../issue/grouper.ts";
import { Visitor } from "./visitor.ts";
import { OtlpExport, type OtlpResource } from "./otlp.ts";

/** The largest OTLP/JSON export taken, the OpenTelemetry Collector's default of 20 MiB. */
const EXPORT_BYTES = 20 * 1024 * 1024;

/** The event name of a log record that records an exception. */
const EXCEPTION = "exception";

/** The OTLP/HTTP signals by their paths below the adapter's. */
const SIGNALS: ReadonlyMap<string, OtlpSignal> = new Map([
    ["/traces", "traces"],
    ["/logs", "logs"],
    ["/metrics", "metrics"],
]);

/** One exception a decoded export recorded, once per trace, with the events recording it. */
interface Recorded {
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
        // take only posted exports of a signal
        const signal = SIGNALS.get(new URL(request.url).pathname);
        if (signal === undefined) {
            return new Response(null, { status: 404 });
        } else if (request.method !== "POST") {
            return new Response(null, { status: 405 });
        }

        // append the export as its forwarder
        const scope = present(context.scope, "the scope of an OTLP export");
        const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`, EXPORT_BYTES);
        await this.append(context.access(scope), scope, signal, body);

        return new Response(null, { status: 204 });
    }

    /** Append an export of a signal into a scope as the machine forwarding it. */
    async append(
        caller: AccessContext,
        scope: string,
        signal: OtlpSignal,
        body: unknown,
    ): Promise<void> {
        // decode each resource's records, group their exceptions and hash their visitors
        const resources = await new OtlpExport(signal, body).decode(scope);
        for (const resource of resources) {
            await this.#group(resource);
            await this.#visit(resource);
        }

        // append each type's records in one mutation as the forwarder
        const appended = [
            [log, resources.flatMap((each) => each.events.log)],
            [span, resources.flatMap((each) => each.events.span)],
            [metric, resources.flatMap((each) => each.events.metric)],
            [visit, resources.flatMap((each) => each.events.visit)],
            [action, resources.flatMap((each) => each.events.action)],
            [session, resources.flatMap((each) => each.events.session)],
        ] as const;
        const calls = appended
            .filter(([, items]) => items.length > 0)
            .map(([type, items]) => type.call("append", { ...type.in(scope), items }));
        if (calls.length > 0) {
            const now = this.#store.clock.now();
            await this.#store.mutate(caller, { id: RequestId.create(now), calls });
        }
    }

    /** Group a space resource's exceptions into issues, setting each recording event's issue. */
    async #group(resource: OtlpResource): Promise<void> {
        // group the exceptions of a space's installations only
        const space = schema.identifier("space").safeParse(resource.emitter.scope);
        const recorded = this.#recorded(resource);
        if (!space.success || recorded.length === 0) {
            return;
        }

        // reuse the issues earlier exports grouped the same exceptions of their traces into, and group the rest
        const known = await this.#known(space.data, recorded);
        const fresh = recorded.filter((each) => !known.has(each));
        const grouped = await this.#grouper.group(
            space.data,
            fresh.map((each) => each.occurrence),
            this.#store.clock.now(),
        );

        // set each log record's and span's issue
        const issues = new Map(
            recorded.flatMap((each) => {
                const issue = known.get(each) ?? grouped.get(each.occurrence);

                return issue === undefined
                    ? []
                    : each.events.map((event) => [event, issue] as const);
            }),
        );
        resource.setIssues(issues);
    }

    /** Find the issues earlier exports grouped the same exceptions of their traces into. */
    async #known(
        scope: Identifier<"space">,
        recorded: readonly Recorded[],
    ): Promise<Map<Recorded, string>> {
        // look only for exceptions recorded in a trace
        const traces = [...new Set(recorded.flatMap((each) => each.trace ?? []))];
        if (traces.length === 0) {
            return new Map();
        }

        // read the grouped spans and exception log records of every trace at once
        const events = this.#store.events;
        const where: JsonCondition = { trace: { in: traces }, issue: { isNotNull: true } };
        const [spans, logs] = await Promise.all([
            events.query(span.eventKind(), { scopes: [scope], where }),
            events.query(log.eventKind(), {
                scopes: [scope],
                where: { ...where, name: EXCEPTION },
            }),
        ]);

        // key each grouped log record, then each span annotation, by its trace and exception
        const issues = new Map<string, string>();
        for (const event of logs.events) {
            const exception = Exception.read(event.keys.attributes, event.keys.severity);
            if (exception !== undefined && event.keys.issue !== null) {
                issues.set(`${event.keys.trace} ${exception.identity}`, event.keys.issue);
            }
        }
        for (const event of spans.events) {
            const { annotations } = present(event.data, `the data of span ${event.id}`);
            const annotation = annotations.find((each) => each.name === EXCEPTION);
            const exception =
                annotation === undefined ? undefined : Exception.read(annotation.attributes, 0);
            if (exception !== undefined && event.keys.issue !== null) {
                issues.set(`${event.keys.trace} ${exception.identity}`, event.keys.issue);
            }
        }

        // match each traced exception to its earlier issue
        return new Map(
            recorded.flatMap((each) => {
                const key = `${each.trace} ${each.occurrence.exception.identity}`;
                const issue = each.trace === undefined ? undefined : issues.get(key);

                return issue === undefined ? [] : [[each, issue] as const];
            }),
        );
    }

    /** Set the visitor's hash on the visits and actions of a space page's resource. */
    async #visit(resource: OtlpResource): Promise<void> {
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
            return;
        }

        // set the hash on each visit and action
        const now = this.#store.clock.now();
        const hashed = await new Visitor(this.#store, space.data).hash(
            { installation, ...visitor },
            now,
        );
        resource.setVisitor(hashed);
    }

    /** Read a resource's exceptions of installations once per trace. */
    #recorded(resource: OtlpResource): Recorded[] {
        // read an exception of an installation as an occurrence
        const { build } = resource.emitter;
        const occurrence = (
            exception: Exception | undefined,
            event: {
                readonly installationId: Identifier<"installation"> | null;
                readonly person: string | null;
            },
            time: number,
        ): Occurrence | undefined =>
            exception === undefined || event.installationId === null
                ? undefined
                : {
                      exception,
                      installation: event.installationId,
                      build,
                      person: event.person,
                      time,
                  };

        // read the exception log records
        const logs = resource.events.log.flatMap((event, index): Recorded[] => {
            const exception =
                event.name === EXCEPTION
                    ? Exception.read(event.attributes, event.severity)
                    : undefined;
            const found = occurrence(exception, event, event.time);

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
        const spans = resource.events.span.flatMap((event, index): Recorded[] => {
            const annotation = event.data.annotations.find((each) => each.name === EXCEPTION);
            const found =
                annotation === undefined
                    ? undefined
                    : occurrence(Exception.read(annotation.attributes, 0), event, annotation.time);

            return found === undefined
                ? []
                : [{ occurrence: found, events: [`span:${index}`], trace: event.trace }];
        });

        // collapse the recordings of one exception in one trace, keeping each untraced one apart
        const collapsed = new Map<string, Recorded>();
        for (const each of [...logs, ...spans]) {
            const key =
                each.trace === undefined
                    ? each.events.join()
                    : `${each.trace} ${each.occurrence.exception.identity}`;
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
