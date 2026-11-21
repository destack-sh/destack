import { present } from "@destack/schema";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import { SpanPath } from "@destack/environment";
import type { Check, Selector } from "@destack/package/declare";

/** The span attribute counting the statements run inside a span, nested ones included. */
const STATEMENTS = "db.client.operation.count";

/** The attribute marking a span that started with no other span running. */
const OUTERMOST = "destack.span.outermost";

/** The span total counting the statements that change rows. */
const WRITES = "db.client.operation.write.count";

/** The attribute of a controller pass with the key it ran over. */
const PASS_KEY = "destack.key";

/** The histogram every statement records its duration in, by database, operation and table. */
const STATEMENT_DURATION = "db.client.operation.duration";

/** The counter of the rows statements returned or changed. */
const ROWS = "destack.db.rows";

/** The counter of keys a committed change woke, by job and changed table. */
const KEY_WAKES = "destack.controller.wakes";

/** The histogram of log reads, by reader and whether the read found changes. */
const LOG_READS = "destack.log.read.duration";

/** The metric counting each listing of a job's keys. */
const LISTINGS = "destack.controller.listings";

/** The spans whose statements a recording totals: a job's pass and an object call. */
const MEASURED_SPANS: ReadonlySet<string> = new Set([
    "destack.controller.pass",
    "destack.object.call",
]);

/** The largest points of a failed metric check its failure lists. */
const LISTED_POINTS = 8;

/** The severity text of an error record, as OpenTelemetry writes it. */
const ERROR = "ERROR";

/** The resource attribute naming the host a signal came from, after OpenTelemetry. */
const HOST_ATTRIBUTE = "host.name";

/** A span a world recorded. */
type Span = SimulatedTelemetry["spans"][number];

/** A log record a world recorded. */
type LogRecord = SimulatedTelemetry["logs"][number];

/** The attribute values of a signal. */
type Attributes = Readonly<Record<string, unknown>>;

/** One metric's data point: its host and attributes, and its records or sum. */
interface Point {
    /** The metric's name, such as `db.client.operation.duration`. */
    readonly name: string;
    /** The host that emitted it, none for the world's own. */
    readonly host: string | undefined;
    /** Its attributes. */
    readonly attributes: Attributes;
    /** A histogram's records, or a counter's sum. */
    readonly value: number;
}

/** What a world's telemetry recorded between two readings: spans, log records and metric growth. */
export class Recording {
    /** The spans that ended. */
    readonly spans: readonly Span[];
    /** The log records emitted. */
    readonly logs: readonly LogRecord[];
    /** Each metric data point's growth. */
    readonly points: readonly Point[];

    /** Take a recording's spans, records and points. */
    constructor(spans: readonly Span[], logs: readonly LogRecord[], points: readonly Point[]) {
        this.spans = spans;
        this.logs = logs;
        this.points = points;
    }

    /** Read everything a world's telemetry recorded so far. */
    static async read(telemetry: SimulatedTelemetry): Promise<Recording> {
        // read every data point of every host's metrics
        const points = (await telemetry.metrics()).flatMap((resource) =>
            resource.scopeMetrics.flatMap((scope) =>
                scope.metrics.flatMap((metric) =>
                    metric.dataPoints.map((point) => ({
                        name: metric.descriptor.name,
                        host: hostOf(resource.resource.attributes),
                        attributes: point.attributes,
                        value: typeof point.value === "number" ? point.value : countOf(point.value),
                    })),
                ),
            ),
        );

        return new Recording([...telemetry.spans], [...telemetry.logs], points);
    }

    /** Read what the world recorded after an earlier reading. */
    since(earlier: Recording): Recording {
        // keep the later spans and records, and each point's growth
        const before = new Map(earlier.points.map((point) => [keyOf(point), point.value]));
        const grown = this.points
            .map((point) => ({ ...point, value: point.value - (before.get(keyOf(point)) ?? 0) }))
            .filter((point) => point.value !== 0);

        return new Recording(
            this.spans.slice(earlier.spans.length),
            this.logs.slice(earlier.logs.length),
            grown,
        );
    }

    /** List the checks an operation's repetitions fail, a metric's growth per repetition. */
    failures(operation: string, times: number, checks: readonly Check[]): string[] {
        return checks.flatMap((check) => {
            // count the spans that ended
            if ("spans" in check) {
                const count = this.spans.filter((span) =>
                    matches(
                        check.spans,
                        span.name,
                        hostOf(span.resource.attributes),
                        span.attributes,
                    ),
                ).length;

                // expect the exact count
                if ("count" in check) {
                    return count === check.count
                        ? []
                        : [
                              `${operation} ended ${count} spans ${describe(check.spans)}, not ${check.count}`,
                          ];
                }
                // refuse a bound on spans that never ended
                else if (count === 0) {
                    return [
                        `${operation} ended no spans ${describe(check.spans)}: bound a cost that occurs`,
                    ];
                }
                // expect at most the bound per repetition
                else {
                    return count / times <= check.atMost
                        ? []
                        : [
                              `${operation} ended ${count / times} spans ${describe(check.spans)} each, more than ${check.atMost}`,
                          ];
                }
            }
            // count the log records emitted
            else if ("logs" in check) {
                const { severity, ...selector } = check.logs;
                const count = this.logs.filter(
                    (record) =>
                        (severity === undefined || record.severityText === severity) &&
                        matches(
                            selector,
                            undefined,
                            hostOf(record.resource.attributes),
                            record.attributes,
                        ),
                ).length;

                return count === check.count
                    ? []
                    : [
                          `${operation} logged ${count} records ${describe(check.logs)}, not ${check.count}`,
                      ];
            }
            // bound a metric's growth per repetition
            else if ("metric" in check) {
                const total = this.count(check.metric);

                // expect the exact growth
                if ("count" in check) {
                    return total === check.count
                        ? []
                        : [
                              `${operation} grew ${describe(check.metric)} by ${total}, not ${check.count}`,
                          ];
                }
                // refuse a bound on a metric that never grew
                else if (total === 0) {
                    return [
                        `${operation} grew no ${describe(check.metric)}: bound a cost that occurs`,
                    ];
                }
                // expect at most the bound per repetition, listing the largest points past it
                else {
                    const each = total / times;
                    const points = this.points
                        .filter((point) =>
                            matches(check.metric, point.name, point.host, point.attributes),
                        )
                        .toSorted((left, right) => right.value - left.value)
                        .slice(0, LISTED_POINTS)
                        .map((point) => `${point.value / times} ${nameOf(point)}`);

                    return each <= check.atMost
                        ? []
                        : [
                              `${operation} grew ${describe(check.metric)} by ${each}, more than ${check.atMost}: ${points.join(", ")}`,
                          ];
                }
            }
            // leave observations to the runner, which reads them through the driver
            else {
                return [];
            }
        });
    }

    /** List the error records no declared error check selects, by their bodies. */
    undeclaredErrors(checks: readonly Check[]): string[] {
        const declared = checks.flatMap((check) =>
            "logs" in check && check.logs.severity === ERROR ? [check.logs] : [],
        );

        return this.logs
            .filter((record) => record.severityText === ERROR)
            .filter(
                (record) =>
                    !declared.some((selector) =>
                        matches(
                            selector,
                            undefined,
                            hostOf(record.resource.attributes),
                            record.attributes,
                        ),
                    ),
            )
            .map((record) =>
                record.body === undefined
                    ? `${record.eventName ?? "record"} ${JSON.stringify(record.attributes)}`
                    : typeof record.body === "string"
                      ? record.body
                      : JSON.stringify(record.body),
            );
    }

    /** Total the growth of the metric points a selector matches, such as statements. */
    count(selector: Selector): number {
        return this.points
            .filter((point) => matches(selector, point.name, point.host, point.attributes))
            .reduce((sum, point) => sum + point.value, 0);
    }

    /** Total each metric's growth by name, and by name and host, as a report's counters. */
    counters(): Record<string, number> {
        const counters: Record<string, number> = {};
        for (const point of this.points) {
            const keys = [
                point.name,
                ...(point.host === undefined ? [] : [`${point.name}@${point.host}`]),
            ];
            for (const key of keys) {
                counters[key] = (counters[key] ?? 0) + point.value;
            }
        }

        return counters;
    }

    /** Count the statements run on every database, by operation and table. */
    queries(): Record<string, number> {
        const queries: Record<string, number> = {};
        for (const point of this.points.filter((each) => each.name === STATEMENT_DURATION)) {
            // read the statement's operation and its table, absent for a transaction
            const operation = String(point.attributes["db.operation.name"]);
            const table = point.attributes["db.collection.name"];
            const target = typeof table === "string" ? `${operation}:${table}` : operation;
            const key = `queries{${target}}`;
            queries[key] = (queries[key] ?? 0) + point.value;
        }

        return queries;
    }

    /** Count the rows statements returned or changed on every database, by operation and table. */
    rows(): Record<string, number> {
        const rows: Record<string, number> = {};
        for (const point of this.points.filter((each) => each.name === ROWS)) {
            // read the statement's operation and its table, absent for a transaction
            const operation = String(point.attributes["db.operation.name"]);
            const table = point.attributes["db.collection.name"];
            const target = typeof table === "string" ? `${operation}:${table}` : operation;
            for (const key of new Set([`rows{${target}}`, `rows{${operation}}`])) {
                rows[key] = (rows[key] ?? 0) + point.value;
            }
        }

        return rows;
    }

    /** Count the log reads, by reader and whether the read found changes. */
    reads(): Record<string, number> {
        const reads: Record<string, number> = {};
        for (const point of this.points.filter((each) => each.name === LOG_READS)) {
            // read the reader and the outcome
            const reader = String(point.attributes["destack.log.reader"]);
            const outcome = String(point.attributes["destack.log.read.outcome"]);
            const key = `reads{${reader}:${outcome}}`;
            reads[key] = (reads[key] ?? 0) + point.value;
        }

        return reads;
    }

    /** Count the keys changes woke, by the job woken and the changed table. */
    wakes(): Record<string, number> {
        const wakes: Record<string, number> = {};
        for (const point of this.points.filter((each) => each.name === KEY_WAKES)) {
            // read the woken job and the changed table
            const job = String(point.attributes["destack.controller.name"]);
            const table = String(point.attributes["db.collection.name"]);
            const key = `wakes{${job}:${table}}`;
            wakes[key] = (wakes[key] ?? 0) + point.value;
        }

        return wakes;
    }

    /** Count the listings of jobs' keys, by what listed them. */
    listings(): Record<string, number> {
        const listings: Record<string, number> = {};
        for (const point of this.points.filter((each) => each.name === LISTINGS)) {
            const key = `listings{${String(point.attributes["destack.wake.kind"])}}`;
            listings[key] = (listings[key] ?? 0) + point.value;
        }

        return listings;
    }

    /** Total the statements under each outermost span, by its name or a pass's label. */
    outermost(): Record<string, number> {
        // total the statements of the spans that started with no other span running
        const statements: Record<string, number> = {};
        for (const span of this.spans.filter((each) => each.attributes[OUTERMOST] === true)) {
            const counted = span.attributes[STATEMENTS];
            const count = typeof counted === "number" ? counted : 0;
            const label = MEASURED_SPANS.has(span.name)
                ? SpanPath.label(span.name, span.attributes)
                : span.name;
            const key = `statements{outermost[${label.replaceAll(/\s/gu, "_")}]}`;
            statements[key] = (statements[key] ?? 0) + count;
        }

        return statements;
    }

    /** Count each controller pass and object call with its statements and keys, by label. */
    work(): Record<string, number> {
        // take the spans that are a job's pass or a call
        const statements: Record<string, number> = {};
        const keys = new Map<string, Set<string>>();
        const measured = this.spans.filter((span) => MEASURED_SPANS.has(span.name));
        for (const span of measured) {
            // count the span and the statements run under it, none for a span that ran none
            const counted = span.attributes[STATEMENTS];
            const count = typeof counted === "number" ? counted : 0;
            const label = SpanPath.label(span.name, span.attributes).replaceAll(/\s/gu, "_");
            const key = `statements{${label}}`;
            statements[key] = (statements[key] ?? 0) + count;
            statements[`spans{${label}}`] = (statements[`spans{${label}}`] ?? 0) + 1;

            // count a pass or a call that changed no row
            if ((span.attributes[WRITES] ?? 0) === 0) {
                statements[`idle{${label}}`] = (statements[`idle{${label}}`] ?? 0) + 1;
            }

            // collect the key a pass ran over
            const over = span.attributes[PASS_KEY];
            if (typeof over === "string") {
                keys.set(label, (keys.get(label) ?? new Set()).add(over));
            }
        }

        // count the distinct keys of each label that ran over keys
        for (const [label, passed] of keys) {
            statements[`keys{${label}}`] = passed.size;
        }

        return statements;
    }
}

/** Report whether a signal matches a selector by name, host and attributes. */
function matches(
    selector: Selector,
    name: string | undefined,
    host: string | undefined,
    attributes: Attributes,
): boolean {
    // match each part the selector names, any value for a part it leaves out
    const isNamed = selector.name === undefined || selector.name === name;
    const isHosted =
        selector.host === undefined || (host !== undefined && glob(selector.host).test(host));
    const isAttributed = Object.entries(selector.attributes ?? {}).every(
        ([key, value]) => attributes[key] === value,
    );

    return isNamed && isHosted && isAttributed;
}

/** Compile a host pattern whose `*` matches any run of characters. */
function glob(pattern: string): RegExp {
    const escaped = pattern
        .split("*")
        .map((part) => part.replaceAll(/[.+?^${}()|[\]\\]/gu, "\\$&"));

    return new RegExp(`^${escaped.join(".*")}$`, "u");
}

/** Read the host a resource names. */
function hostOf(attributes: Attributes): string | undefined {
    const host = attributes[HOST_ATTRIBUTE];

    return typeof host === "string" ? host : undefined;
}

/** Read a histogram point's records. */
function countOf(value: { readonly count?: number }): number {
    return present(value.count, "a histogram point's count");
}

/** Key a data point by its metric, host and attributes. */
function keyOf(point: Point): string {
    return JSON.stringify([point.name, point.host, point.attributes]);
}

/** Name a data point by its host and attribute values, such as `home main.db:INSERT:note`. */
function nameOf(point: Point): string {
    const attributes = Object.values(point.attributes).join(":");

    return attributes === "" ? (point.host ?? "world") : `${point.host ?? "world"} ${attributes}`;
}

/** Write a selector as a check's failure names it. */
function describe(selector: object): string {
    return JSON.stringify(selector);
}
