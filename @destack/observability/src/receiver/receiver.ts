import type { DatabaseConnection, JsonCondition } from "@destack/db";
import type { EventStore } from "@destack/event";
import { type Digest, type Identifier, schema } from "@destack/schema";
import type { OtlpEmitter, OtlpReceiver, OtlpSignal } from "@destack/telemetry/otlp";
import { action, log, metric, span, visit } from "../event/index.ts";
import { Exception } from "../issue/exception.ts";
import type { Grouper, Occurrence } from "../issue/group.ts";
import { type Decoded, Otlp } from "./otlp.ts";
import { VisitorSalt } from "./visitor.ts";

/** The event name of a log record recording an exception, from the OpenTelemetry semantic conventions. */
const EXCEPTION = "exception";

/** What a receiver keeps exports with: the space's events, the grouping of its exceptions and the visitor salt. */
export interface ReceiverOptions {
    /** The event store the exports are appended to. */
    readonly events: EventStore;
    /** Group a space's exceptions into issues, absent on a machine keeping only its own telemetry. */
    readonly grouper?: Grouper;
    /** The database keeping the day's visitor salt, absent where no views export. */
    readonly salt?: DatabaseConnection;
    /** Report a failure grouping exceptions, which keeps the export as it came. */
    readonly report: (error: unknown) => void;
    /** Read the current time in Unix milliseconds, the server's clock. */
    readonly clock: () => number;
}

/** One exception a decoded export recorded, once per trace, with the events recording it. */
interface Recorded {
    /** The occurrence the grouper reads. */
    readonly occurrence: Occurrence;
    /** The events recording it, as their kind and index in the kind's list, such as `span:2`. */
    readonly events: readonly string[];
    /** The trace it was recorded in. */
    readonly trace: string | undefined;
}

/** Receives the OTLP exports of a space's installations and pages: decoded into events, exceptions grouped into issues, visitors hashed, appended once each. */
export class ObservabilityReceiver implements OtlpReceiver {
    /** What the receiver keeps exports with. */
    readonly #options: ReceiverOptions;

    /** Keep exports in a space's events. */
    constructor(options: ReceiverOptions) {
        this.#options = options;
    }

    /** Take an export of a signal under the emitter its receiver verified. */
    async receive(emitter: OtlpEmitter, signal: OtlpSignal, body: unknown): Promise<void> {
        // decode the export, then group its exceptions and hash its visitor
        const decoded = await this.#visited(
            await this.#grouped(await Otlp.read(signal, body, emitter), emitter),
            emitter,
        );

        // append each kind's events once, dropping a failed write as at-most-once kinds do
        const { events } = this.#options;
        if (decoded.log.length > 0) {
            await events.append(log, decoded.log);
        }
        if (decoded.span.length > 0) {
            await events.append(span, decoded.span);
        }
        if (decoded.metric.length > 0) {
            await events.append(metric, decoded.metric);
        }
        if (decoded.visit.length > 0) {
            await events.append(visit, decoded.visit);
        }
        if (decoded.action.length > 0) {
            await events.append(action, decoded.action);
        }
    }

    /** Group a space's exceptions into issues and stamp each recording event with its issue, keeping the export as it came when grouping fails. */
    async #grouped(decoded: Decoded, emitter: OtlpEmitter): Promise<Decoded> {
        // group the exceptions of a space's installations only
        const { grouper, report } = this.#options;
        const space = schema.identifier("space").safeParse(emitter.scope);
        const recorded = recordedOf(decoded, emitter.build);
        if (grouper === undefined || !space.success || recorded.length === 0) {
            return decoded;
        }

        // reuse the issue an earlier export grouped the same exception of the trace into, and group the rest
        const issues = new Map<Recorded, string>();
        try {
            const fresh: Recorded[] = [];
            for (const each of recorded) {
                const known = await this.#known(space.data, each);
                if (known === undefined) {
                    fresh.push(each);
                } else {
                    issues.set(each, known);
                }
            }
            const grouped = await grouper.group(
                space.data,
                fresh.map((each) => each.occurrence),
                this.#options.clock(),
            );
            for (const each of fresh) {
                const issue = grouped.get(each.occurrence);
                if (issue !== undefined) {
                    issues.set(each, issue);
                }
            }
        } catch (error) {
            report(error);

            return decoded;
        }

        // stamp each log record and span recording an exception with its issue
        const byEvent = new Map(
            [...issues].flatMap(([each, issue]) => each.events.map((event) => [event, issue])),
        );

        return {
            ...decoded,
            log: decoded.log.map((event, index) => ({
                ...event,
                keys: { ...event.keys, issue: byEvent.get(`log:${index}`) ?? event.keys.issue },
            })),
            span: decoded.span.map((event, index) => ({
                ...event,
                keys: { ...event.keys, issue: byEvent.get(`span:${index}`) ?? event.keys.issue },
            })),
        };
    }

    /** Find the issue an earlier export grouped the same exception of a trace into, as every span it escapes and its log record record it again. */
    async #known(scope: Identifier<"space">, recorded: Recorded): Promise<string | undefined> {
        // look only for exceptions recorded in a trace
        const { events } = this.#options;
        const { trace } = recorded;
        if (trace === undefined) {
            return undefined;
        }

        // read the grouped spans and exception log records of the trace
        const identity = identityOf(recorded.occurrence.exception);
        const where: JsonCondition = { trace, issue: { isNotNull: true } };
        const [spans, logs] = await Promise.all([
            events.query(span, { scope, where }),
            events.query(log, { scope, where: { ...where, name: EXCEPTION } }),
        ]);

        // find one recording the same exception
        const annotated = spans.events.find((event) => {
            const annotation = event.data.annotations.find((each) => each.name === EXCEPTION);
            const exception =
                annotation === undefined ? undefined : Exception.read(annotation.attributes, 0);

            return exception !== undefined && identityOf(exception) === identity;
        });
        const logged = logs.events.find((event) => {
            const exception = Exception.read(event.keys.attributes, event.keys.severity);

            return exception !== undefined && identityOf(exception) === identity;
        });

        return annotated?.keys.issue ?? logged?.keys.issue ?? undefined;
    }

    /** Stamp the visits and actions of a page's export with its visitor's hash under the day's salt. */
    async #visited(decoded: Decoded, emitter: OtlpEmitter): Promise<Decoded> {
        // hash the visitor of a page's analytics only
        const { salt } = this.#options;
        const { visitor, installation } = emitter;
        if (
            salt === undefined ||
            visitor === undefined ||
            installation === undefined ||
            decoded.visit.length + decoded.action.length === 0
        ) {
            return decoded;
        }
        const hashed = await VisitorSalt.hash(salt, this.#options.clock(), {
            installation,
            ...visitor,
        });

        // stamp each visit and action
        return {
            ...decoded,
            visit: decoded.visit.map((event) => ({
                ...event,
                keys: { ...event.keys, visitor: hashed },
            })),
            action: decoded.action.map((event) => ({
                ...event,
                keys: { ...event.keys, visitor: hashed },
            })),
        };
    }
}

/** Read the exceptions of an export's installations once per trace: each exception log record, and each exception a span annotated. */
function recordedOf(decoded: Decoded, build: Digest | undefined): Recorded[] {
    // read the exception log records
    const logs = decoded.log.flatMap((event, index): Recorded[] => {
        const found =
            event.keys.name === EXCEPTION
                ? occurrenceOf(
                      Exception.read(event.keys.attributes, event.keys.severity),
                      event.keys.installation,
                      event.keys.person,
                      event.time,
                      build,
                  )
                : undefined;

        return found === undefined
            ? []
            : [
                  {
                      occurrence: found,
                      events: [`log:${index}`],
                      trace: event.keys.trace ?? undefined,
                  },
              ];
    });

    // read the first exception each span annotated
    const spans = decoded.span.flatMap((event, index): Recorded[] => {
        const annotation = event.data.annotations.find((each) => each.name === EXCEPTION);
        const found =
            annotation === undefined
                ? undefined
                : occurrenceOf(
                      Exception.read(annotation.attributes, 0),
                      event.keys.installation,
                      event.keys.person,
                      annotation.time,
                      build,
                  );

        return found === undefined
            ? []
            : [{ occurrence: found, events: [`span:${index}`], trace: event.keys.trace }];
    });

    // collapse the recordings of one exception in one trace, keeping each untraced one apart
    const collapsed = new Map<string, Recorded>();
    for (const each of [...logs, ...spans]) {
        const key =
            each.trace === undefined
                ? each.events.join()
                : `${each.trace} ${identityOf(each.occurrence.exception)}`;
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

/** Read an exception's identity within its trace: its type, message and stack trace. */
function identityOf(exception: Exception): string {
    return JSON.stringify([exception.type, exception.message, exception.stacktrace]);
}

/** Read an exception of an installation as an occurrence, none without an exception or an installation. */
function occurrenceOf(
    exception: Exception | undefined,
    installation: string | null,
    person: string | null,
    time: number,
    build: Digest | undefined,
): Occurrence | undefined {
    const parsed = schema.identifier("installation").safeParse(installation);

    return exception === undefined || !parsed.success
        ? undefined
        : { exception, installation: parsed.data, build, person, time };
}
