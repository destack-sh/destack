import { expect, refusal, test } from "@destack/test";
import { OtlpExport } from "./request.ts";

/** The installation a forwarder knows an export comes from. */
const INSTALLATION = "installation-01996ab0-0000-7000-8000-000000000001";

/** The build manifest a forwarder knows an export's installation serves. */
const MANIFEST = "5f8d1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7";

test("stamp each resource of an export with its forwarder's emitter, refusing an export claiming one", async () => {
    // stamp a resource with its own attributes, and one without any
    const own = { key: "service.name", value: { stringValue: "notes" } };
    const stamped = new OtlpExport("logs", {
        resourceLogs: [{ resource: { attributes: [own] }, scopeLogs: [] }, { scopeLogs: [] }],
    }).stamp({ installation: INSTALLATION, build: MANIFEST }).body;

    // refuse a resource claiming its build and another installation
    const forged = [
        { key: "destack.build.manifest", value: { stringValue: MANIFEST } },
        { key: "destack.installation.id", value: { stringValue: "installation-forged" } },
    ];
    const claim = () =>
        new OtlpExport("logs", {
            resourceLogs: [{ resource: { attributes: [own, ...forged] }, scopeLogs: [] }],
        }).stamp({ installation: INSTALLATION });

    // set the known emitter on each resource after its other attributes, and refuse the claim
    const known = [
        { key: "destack.installation.id", value: { stringValue: INSTALLATION } },
        { key: "destack.build.manifest", value: { stringValue: MANIFEST } },
    ];
    expect(stamped).toEqual({
        resourceLogs: [
            { resource: { attributes: [own, ...known] }, scopeLogs: [] },
            { resource: { attributes: known }, scopeLogs: [] },
        ],
    });
    expect(await refusal(Promise.resolve().then(claim))).toEqual([
        "FORBIDDEN",
        "EMITTER_ATTRIBUTE_DENIED",
        "export emitter 'destack.build.manifest, destack.installation.id' is forbidden: only the receiver can record an export's emitter",
    ]);
});

test("read an empty export as one of no resources, refusing a malformed one", async () => {
    // stamp an export omitting its empty resources, as OTLP/JSON omits empty lists
    const empty = new OtlpExport("traces", {}).stamp({ installation: INSTALLATION }).body;

    // refuse an export whose resources are no list
    const malformed = () =>
        new OtlpExport("traces", { resourceSpans: "spans" }).stamp({ installation: INSTALLATION });

    // keep the empty export, and refuse the malformed one as a bad request
    expect(empty).toEqual({ resourceSpans: [] });
    expect(await refusal(Promise.resolve().then(malformed))).toEqual([
        "BAD_REQUEST",
        "EXPORT_INVALID",
        "the 'traces' export is invalid: 'resourceSpans: Invalid input: expected array, received string'",
    ]);
});

test("read a logs export's records with their times, severity, body, attributes and span", () => {
    // read one record emitted in a span, and one with only its observed time
    const records = new OtlpExport("logs", {
        resourceLogs: [
            {
                resource: { attributes: [{ key: "service.name", value: { stringValue: "site" } }] },
                scopeLogs: [
                    {
                        scope: { name: "page", version: "1" },
                        logRecords: [
                            {
                                timeUnixNano: "1700000000000000001",
                                observedTimeUnixNano: "1700000000000000002",
                                severityNumber: 17,
                                eventName: "exception",
                                body: { stringValue: "pricing link failed" },
                                attributes: [
                                    { key: "exception.type", value: { stringValue: "TypeError" } },
                                ],
                                traceId: "0af7651916cd43dd8448eb211c80319c",
                                spanId: "b7ad6b7169203331",
                            },
                            { observedTimeUnixNano: 5, attributes: [] },
                        ],
                    },
                ],
            },
        ],
    }).records();

    // read each record's fields under their shared scope, the observed time for a missing one
    expect(
        records.map((record) => ({
            hrTime: record.hrTime,
            hrTimeObserved: record.hrTimeObserved,
            spanContext: record.spanContext,
            severityNumber: record.severityNumber,
            eventName: record.eventName,
            body: record.body,
            resource: record.resource.attributes,
            instrumentationScope: record.instrumentationScope,
            attributes: record.attributes,
        })),
    ).toEqual([
        {
            hrTime: [1_700_000_000, 1],
            hrTimeObserved: [1_700_000_000, 2],
            spanContext: {
                traceId: "0af7651916cd43dd8448eb211c80319c",
                spanId: "b7ad6b7169203331",
                traceFlags: 1,
            },
            severityNumber: 17,
            eventName: "exception",
            body: "pricing link failed",
            resource: { "service.name": "site" },
            instrumentationScope: { name: "page", version: "1" },
            attributes: { "exception.type": "TypeError" },
        },
        {
            hrTime: [0, 5],
            hrTimeObserved: [0, 5],
            spanContext: undefined,
            severityNumber: undefined,
            eventName: undefined,
            body: undefined,
            resource: { "service.name": "site" },
            instrumentationScope: { name: "page", version: "1" },
            attributes: {},
        },
    ]);
});
