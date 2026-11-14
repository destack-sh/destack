import { present } from "@destack/schema";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Check, Selector } from "@destack/package/declare";

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
            .map((record) => String(record.body));
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
