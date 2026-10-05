import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { ROOT_CONTEXT } from "@opentelemetry/api";
import { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
import { BatchSpanProcessor, SamplingDecision } from "@opentelemetry/sdk-trace";
import { startTelemetry } from "../host/index.ts";
import { OtlpExporter } from "../otlp/index.ts";
import { RatioSampler } from "./sampler.ts";
import { TailSampler } from "./tail.ts";

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

test("keep failed and slow unsampled traces with their records, warnings alone, and records outside traces", async () => {
    // export through a tail sampler behind a sampler keeping no trace by ratio
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
    const tail = new TailSampler(
        new BatchSpanProcessor({ exporter: exporter.traces }),
        new BatchLogRecordProcessor({ exporter: exporter.logs }),
        20,
    );
    const telemetry = await startTelemetry({
        name: source.name,
        version: source.version,
        traces: { sampler: new RatioSampler(0), spanProcessors: [tail.spans] },
        logs: { processors: [tail.logs] },
        metrics: {},
        report: (error) => {
            throw error;
        },
    });

    // run a quiet trace, a failed one, a slow one and a quiet one with a warning, and log outside any trace
    try {
        const { log, span } = telemetry.scope(source);
        await span("quiet", {}, () => log.info("quiet.saved"));
        await span("failed", {}, () => {
            log.info("failed.started");
            throw new Error("broken");
        }).catch(() => {});
        await span("slow", {}, async () => {
            log.debug("slow.waiting");
            await new Promise((resolve) => {
                setTimeout(resolve, 30);
            });
        });
        await span("warned", {}, () => log.warn("warned.conflict"));
        log.info("outside.started");
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

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
