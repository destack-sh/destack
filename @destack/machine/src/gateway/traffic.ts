import type { Meter, MeterAttributes } from "@destack/telemetry/meter";
import { copyRequest, Pipe } from "@destack/environment";
import { OtlpMetricsRequest } from "@destack/telemetry/otlp";
import { computeOperation, computeTime, networkTransfer } from "@destack/telemetry/meter";
import { Reading } from "../meter/index.ts";
import { Duration, type Identifier, schema } from "@destack/schema";
import { PROCESS_CPU_TIME } from "@destack/telemetry/metric";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** The OTLP aggregation temporality of deltas, the one a counted CPU time takes. */
const DELTA = 1;

/** An installation's or its space's traffic through a machine since the meters last took it. */
export interface TrafficCount {
    /** The space. */
    readonly scope: Identifier<"space">;
    /** The installation, absent for work the space does itself. */
    readonly installation?: Identifier<"installation">;
    /** The requests it served. */
    readonly requests: number;
    /** The bytes its requests brought into the machine. */
    readonly ingress: number;
    /** The bytes its responses sent out of the machine. */
    readonly egress: number;
    /** The CPU milliseconds its provider's traces or its process measured. */
    readonly cpuMilliseconds: number;
}

/** The work a provider's traces measured for an installation, or for its space alone. */
export const TraceCount = schema.object({
    /** The installation, absent for work the space does itself. */
    installation: schema.identifier("installation").exactOptional(),
    /** The CPU milliseconds the traces measured. */
    cpuMilliseconds: schema.number().min(0),
});
/** The work a provider's traces measured for an installation, or for its space alone. */
export type TraceCount = schema.Infer<typeof TraceCount>;

/** A number in a traffic count, such as its requests or its bytes. */
type TrafficMeasure = Exclude<keyof TrafficCount, "scope" | "installation">;

/** The meter and attributes each number of a traffic count reads on. */
const METERS: readonly (readonly [TrafficMeasure, Meter, MeterAttributes])[] = [
    ["requests", computeOperation, {}],
    ["cpuMilliseconds", computeTime, {}],
    ["ingress", networkTransfer, { direction: "ingress" }],
    ["egress", networkTransfer, { direction: "egress" }],
];

/** A count whose numbers grow in place. */
type Mutable<Counted> = { -readonly [Field in keyof Counted]: Counted[Field] };

/** The requests a machine's installations serve, the bytes they move and their CPU time. */
export class Traffic {
    /** The counts since the last take, by installation or by space, added to in place. */
    readonly #counts = new Map<string, Mutable<TrafficCount>>();
    /** The waits for the next count. */
    readonly #waiting = new Set<() => void>();

    /** Wait until traffic is counted, or until the signal aborts. */
    next(signal: AbortSignal): Promise<void> {
        // settle at the next count or the abort, whichever comes first
        const { promise, resolve } = Promise.withResolvers<void>();
        const settle = () => {
            this.#waiting.delete(settle);
            signal.removeEventListener("abort", settle);
            resolve();
        };
        this.#waiting.add(settle);
        signal.addEventListener("abort", settle, { once: true });
        if (signal.aborted) {
            settle();
        }

        return promise;
    }

    /** Count one request an installation serves. */
    countRequest(scope: Identifier<"space">, installation: Identifier<"installation">): void {
        this.#count(scope, installation).requests += 1;
    }

    /** Count the bytes of an installation's request as its body streams in. */
    countIngress(
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
        request: Request,
    ): Request {
        // pass a request without a body as it is, and count each chunk of one that has one
        if (request.body === null) {
            return request;
        }
        const counting = this.#counting(scope, installation, "ingress");

        return copyRequest(request, { body: Pipe.through(request.body, counting) });
    }

    /** Count the bytes of an installation's response as its body streams out. */
    countEgress(
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
        response: Response,
    ): Response {
        // pass a response without a body as it is, and count each chunk of one that has one
        if (response.body === null) {
            return response;
        }
        const counting = this.#counting(scope, installation, "egress");

        return new Response(Pipe.through(response.body, counting), {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
        });
    }

    /** Count the CPU time the provider's traces measured. */
    countTrace(scope: Identifier<"space">, traced: TraceCount): void {
        this.#count(scope, traced.installation).cpuMilliseconds += traced.cpuMilliseconds;
    }

    /** Count the process CPU time an instance's OTLP/JSON metrics export reports. */
    countMetrics(
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
        body: unknown,
    ): void {
        // find the process.cpu.time sums
        const metrics = OtlpMetricsRequest.parse(body).resourceMetrics.flatMap((group) =>
            group.scopeMetrics.flatMap((scoped) => scoped.metrics),
        );
        const sums = metrics.filter((metric) => metric.name === PROCESS_CPU_TIME);

        // count each delta sum's points
        for (const { sum } of sums) {
            // refuse a sum that is no delta
            if (sum?.aggregationTemporality !== DELTA) {
                throw MachineError.of(
                    "METRIC_INVALID",
                    `'${PROCESS_CPU_TIME}' is invalid: it must be a delta sum`,
                );
            }

            // add each CPU mode's point
            for (const point of sum.dataPoints) {
                // refuse a point without a value
                const seconds = point.asDouble ?? point.asInt;
                if (seconds === undefined) {
                    throw MachineError.of(
                        "POINT_WITHOUT_VALUE_INVALID",
                        `a point of '${PROCESS_CPU_TIME}' is invalid: it has no value`,
                    );
                }

                // add its seconds as milliseconds
                const milliseconds = Duration.milliseconds({ seconds: Number(seconds) });
                this.#count(scope, installation).cpuMilliseconds += milliseconds;
            }
        }
    }

    /** Take the traffic counted since the last take, starting every count again from zero. */
    take(): TrafficCount[] {
        const taken = [...this.#counts.values()];
        this.#counts.clear();

        return taken;
    }

    /** Read the taken counts on the request, CPU and transfer meters, leaving out zeros. */
    static readings(taken: readonly TrafficCount[]): Reading[] {
        return METERS.flatMap(([measure, meter, attributes]) =>
            Reading.counted(
                meter,
                attributes,
                taken
                    .filter((count) => count[measure] > 0)
                    .map((count) => ({
                        scope: count.scope,
                        ...(count.installation === undefined
                            ? {}
                            : { installation: count.installation }),
                        value: count[measure],
                    })),
            ),
        );
    }

    /** Build a stream adding each chunk's bytes to a direction of an installation's count. */
    #counting(
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
        direction: "ingress" | "egress",
    ): TransformStream<Uint8Array, Uint8Array> {
        return new TransformStream<Uint8Array, Uint8Array>({
            transform: (chunk, controller) => {
                this.#count(scope, installation)[direction] += chunk.byteLength;
                controller.enqueue(chunk);
            },
        });
    }

    /** Read an installation's or the space's count since the last take, starting it at zero. */
    #count(
        scope: Identifier<"space">,
        installation: Identifier<"installation"> | undefined,
    ): Mutable<TrafficCount> {
        // wake the waits for the next count
        for (const settle of this.#waiting) {
            settle();
        }

        // start the count at zero once per take
        const key = installation ?? scope;
        let counted = this.#counts.get(key);
        if (counted === undefined) {
            counted = {
                scope,
                ...(installation === undefined ? {} : { installation }),
                requests: 0,
                ingress: 0,
                egress: 0,
                cpuMilliseconds: 0,
            };
            this.#counts.set(key, counted);
        }

        return counted;
    }
}
