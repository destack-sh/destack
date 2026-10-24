import type { Clock } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { TelemetryScope } from "../scope/index.ts";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { ROOT_CONTEXT } from "@opentelemetry/api";
import { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
import { BatchSpanProcessor, SamplingDecision } from "@opentelemetry/sdk-trace";
import { startTelemetry } from "../bun/index.ts";
import { OtlpExporter } from "../otlp/index.ts";
import { RatioSampler } from "./sampler.ts";
import { TailSampler } from "./tail.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** A named span or log record. */
const NamedItem = schema.looseObject({
    name: schema.string().exactOptional(),
    eventName: schema.string().exactOptional(),
});

/** The span and log record names an export request carries. */
const Named = schema.looseObject({
    resourceSpans: schema
        .array(
            schema.looseObject({
                scopeSpans: schema.array(schema.looseObject({ spans: schema.array(NamedItem) })),
            }),
        )
        .exactOptional(),
    resourceLogs: schema
        .array(
            schema.looseObject({
                scopeLogs: schema.array(
                    schema.looseObject({ logRecords: schema.array(NamedItem) }),
                ),
            }),
        )
        .exactOptional(),
});

/** Run work under a tail sampler behind a sampler that keeps no trace by ratio. */
async function exportThroughTail(
    work: (scope: TelemetryScope, clock: Clock) => Promise<void>,
): Promise<string[]> {
    // export through a tail sampler behind a sampler keeping no trace by ratio, in a world
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const exported: string[] = [];
    const exporter = new OtlpExporter(
        async (_, body) => {
            const request = Named.parse(JSON.parse(new TextDecoder().decode(body)));
            for (const group of request.resourceSpans ?? []) {
                exported.push(
                    ...group.scopeSpans.flatMap((scope) =>
                        scope.spans.map((span) => `span ${span.name}`),
                    ),
                );
            }
            for (const group of request.resourceLogs ?? []) {
                exported.push(
                    ...group.scopeLogs.flatMap((scope) =>
                        scope.logRecords.map((record) => `log ${record.eventName}`),
                    ),
                );
            }
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(
        {
            name: source.name,
            version: source.version,
            signals: (meters) => {
                const tail = new TailSampler(
                    new BatchSpanProcessor({ exporter: exporter.traces }),
                    new BatchLogRecordProcessor({ exporter: exporter.logs }),
                    meters,
                    20,
                );

                return {
                    traces: { sampler: new RatioSampler(0), spanProcessors: [tail.spans] },
                    logs: { processors: [tail.logs] },
                };
            },
            metrics: {},
            report: (error) => {
                throw error;
            },
        },
        environment,
    );

    // run the work and export what the sampler keeps
    try {
        await simulation.drive(work(TelemetryScope.of(source, telemetry), environment.clock));
        await simulation.drive(telemetry.flush());
    } finally {
        await telemetry.shutdown();
    }

    return exported;
}

test("keep failed and slow unsampled traces with their records, warnings alone, and records outside traces", async () => {
    // run a quiet trace, a failed one, a slow one and a quiet one with a warning, and log outside any trace
    const exported = await exportThroughTail(async ({ log, span }, clock) => {
        await span("quiet", {}, () => log.info("quiet.saved"));
        await span("failed", {}, () => {
            log.info("failed.started");
            throw new Error("broken");
        }).catch(() => {});
        await span("slow", {}, async () => {
            log.debug("slow.waiting");
            await clock.sleep(30);
        });
        await span("warned", {}, () => log.warn("warned.conflict"));
        log.info("outside.started");
    });

    // keep the failed and slow traces whole, the warning without its span, and the record outside traces
    expect(exported.toSorted()).toEqual(
        [
            "span failed",
            "log failed.started",
            "span slow",
            "log slow.waiting",
            "log warned.conflict",
            "log outside.started",
        ].toSorted(),
    );
});

test("keep a failed trace's root and its first spans and records up to its bound", async () => {
    // run a failed trace with more child spans and records than an undecided trace buffers
    const exported = await exportThroughTail(async ({ log, span }) => {
        await span("failed", {}, async () => {
            for (let index = 0; index < 1001; index += 1) {
                await span("child", {}, () => log.info("child.saved"));
            }
            throw new Error("broken");
        }).catch(() => {});
    });

    // keep the root beside the first thousand children and records
    const counts = Object.groupBy(exported, (name) => name);
    expect(
        Object.fromEntries(Object.entries(counts).map(([name, all]) => [name, all?.length])),
    ).toEqual({
        "span child": 1000,
        "log child.saved": 1000,
        "span failed": 1,
    });
});

test("sample a ratio of root traces by their random identifier bits and let children follow", () => {
    // decide roots by the identifier's last 8 hexadecimal digits against the ratio
    const sampler = new RatioSampler(0.5);
    const decide = (traceId: string) => sampler.shouldSample(ROOT_CONTEXT, traceId).decision;

    expect([
        decide("0192f3a4b5c60000000000007fffffff"),
        decide("0192f3a4b5c600000000000080000000"),
        new RatioSampler(1).shouldSample(ROOT_CONTEXT, "0192f3a4b5c6000000000000ffffffff").decision,
        new RatioSampler(0).shouldSample(ROOT_CONTEXT, "0192f3a4b5c600000000000000000000").decision,
    ]).toEqual([
        SamplingDecision.RECORD_AND_SAMPLED,
        SamplingDecision.RECORD,
        SamplingDecision.RECORD_AND_SAMPLED,
        SamplingDecision.RECORD,
    ]);
});

test("keep a trace once it outlives the slow threshold, passing every later span past the bound", async () => {
    // run a root that turns slow, then ends more children than a pending trace buffers
    const exported = await exportThroughTail(async ({ span }, clock) => {
        await span("stream", {}, async () => {
            await clock.sleep(30);
            for (let index = 0; index < 1500; index += 1) {
                await span("page", {}, () => undefined);
            }
        });
    });

    // keep the root and every child, as the trace was kept at its first slow child
    const counts = Object.groupBy(exported, (name) => name);
    expect(
        Object.fromEntries(Object.entries(counts).map(([name, all]) => [name, all?.length])),
    ).toEqual({ "span page": 1500, "span stream": 1 });
});
