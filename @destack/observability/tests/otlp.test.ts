import { refusal, expect, test } from "@destack/test";
import { OtlpExport } from "../src/otlp/index.ts";
import { ids, START } from "./fixture/space.ts";
import { logsExport, NOTES_MANIFEST, tracesExport } from "./fixture/notes.ts";

/** The microseconds in a millisecond. */
const MICROSECONDS = 1000;

/** The trace the scenarios record in, 32 hexadecimal digits. */
const TRACE = "a".repeat(32);

/** The span the scenarios record in, 16 hexadecimal digits. */
const SPAN = "b".repeat(16);

/** A metrics export of a counter increment and a one-value histogram point. */
function metricsExport(time: number) {
    const nanoseconds = String(BigInt(time) * 1_000_000n);

    return {
        resourceMetrics: [
            {
                resource: { attributes: [] },
                scopeMetrics: [
                    {
                        scope: { name: "@example/notes", version: "2026.9.0" },
                        metrics: [
                            {
                                name: "note.saves",
                                sum: {
                                    aggregationTemporality: 1,
                                    dataPoints: [{ timeUnixNano: nanoseconds, asInt: "2" }],
                                },
                            },
                            {
                                name: "note.render.cumulative",
                                sum: {
                                    aggregationTemporality: 2,
                                    dataPoints: [{ timeUnixNano: nanoseconds, asInt: "2" }],
                                },
                            },
                        ],
                    },
                ],
            },
        ],
    };
}

test("read a resource's spans and records under the emitter its forwarder names, keying their session and sealing personal attributes only under a person", async () => {
    // read a span for dana in her session and two records, one of them for nobody, in the notes instance's resource
    const [traced] = await new OtlpExport(
        "traces",
        tracesExport([
            {
                trace: TRACE,
                span: SPAN,
                name: "note.rename",
                start: START,
                end: START + 2,
                status: 2,
                attributes: {
                    "enduser.id": "dana",
                    "session.id": "session-1",
                    "sensitive.email": "dana@example.com",
                },
            },
        ]),
    ).decode(ids.space);
    const [logged] = await new OtlpExport(
        "logs",
        logsExport([
            { time: START, name: "note.saved", severity: 9, trace: TRACE, span: SPAN },
            {
                time: START + 1,
                name: "note.conflict",
                severity: 13,
                attributes: { attempts: 2, "sensitive.token": "secret" },
            },
        ]),
    ).decode(ids.space);

    // stamp each with the forwarder's installation, instance and build, keep dana's email apart and drop the unclaimed token
    const stamp = { installationId: ids.notes, build: NOTES_MANIFEST };
    const instrumentation = { name: "@example/notes", version: "2026.9.0" };
    expect({
        emitter: traced?.emitter,
        spans: traced?.events.span.map(({ id: _id, ...span }) => span),
        logs: logged?.events.log.map(({ id: _id, ...record }) => record),
    }).toEqual({
        emitter: {
            scope: ids.space,
            installation: ids.notes,
            instance: ids.instance,
            build: NOTES_MANIFEST,
        },
        spans: [
            {
                time: START * MICROSECONDS,
                ...stamp,
                name: "note.rename",
                trace: TRACE,
                parent: null,
                status: "error",
                duration: 2 * MICROSECONDS,
                issue: null,
                session: "session-1",
                person: "dana",
                attributes: {},
                data: {
                    span: SPAN,
                    annotations: [],
                    links: [],
                    instance: ids.instance,
                    instrumentation,
                    personal: { email: "dana@example.com" },
                },
            },
        ],
        logs: [
            {
                time: START * MICROSECONDS,
                ...stamp,
                name: "note.saved",
                severity: 9,
                trace: TRACE,
                issue: null,
                session: null,
                person: null,
                attributes: {},
                data: { span: SPAN, instance: ids.instance, instrumentation },
            },
            {
                time: (START + 1) * MICROSECONDS,
                ...stamp,
                name: "note.conflict",
                severity: 13,
                trace: null,
                issue: null,
                session: null,
                person: null,
                attributes: { attempts: 2 },
                data: { instance: ids.instance, instrumentation },
            },
        ],
    });
});

test("identify the same records alike in one scope and apart across scopes, and refuse a cumulative metric", async () => {
    // read one export twice in the space and once in another scope
    const exported = logsExport([{ time: START, name: "note.saved", severity: 9 }]);
    const identities = async (scope: string) =>
        (await new OtlpExport("logs", exported).decode(scope)).flatMap((each) =>
            each.events.log.map((record) => record.id),
        );
    const [first, again, elsewhere] = [
        await identities(ids.space),
        await identities(ids.space),
        await identities("space-01996ab0-0000-7000-8000-0000000000ff"),
    ];

    // keep a retried export's identities, give another scope's its own, and refuse summing a cumulative counter
    expect({
        isRetrySame: JSON.stringify(first) === JSON.stringify(again),
        isScoped: first?.[0] !== elsewhere?.[0],
        cumulative: await refusal(
            new OtlpExport("metrics", metricsExport(START)).decode(ids.space),
        ),
    }).toEqual({
        isRetrySame: true,
        isScoped: true,
        cumulative: ["BAD_REQUEST", "metric note.render.cumulative is cumulative: export deltas"],
    });
});

test("stamp each resource of an export with its forwarder's emitter, refusing an export claiming one", async () => {
    // stamp a resource with its own attributes, and one without any
    const own = { key: "service.name", value: { stringValue: "notes" } };
    const stamped = new OtlpExport("logs", {
        resourceLogs: [{ resource: { attributes: [own] }, scopeLogs: [] }, { scopeLogs: [] }],
    }).stamp({ installation: ids.notes, build: NOTES_MANIFEST }).body;

    // refuse a resource claiming its build and another installation
    const forged = [
        { key: "destack.build.manifest", value: { stringValue: NOTES_MANIFEST } },
        { key: "destack.installation.id", value: { stringValue: "installation-forged" } },
    ];
    const claim = () =>
        new OtlpExport("logs", {
            resourceLogs: [{ resource: { attributes: [own, ...forged] }, scopeLogs: [] }],
        }).stamp({ installation: ids.notes });

    // set the known emitter on each resource after its other attributes, and refuse the claim
    const emitter = [
        { key: "destack.installation.id", value: { stringValue: ids.notes } },
        { key: "destack.build.manifest", value: { stringValue: NOTES_MANIFEST } },
    ];
    expect(stamped).toEqual({
        resourceLogs: [
            { resource: { attributes: [own, ...emitter] }, scopeLogs: [] },
            { resource: { attributes: emitter }, scopeLogs: [] },
        ],
    });
    expect(await refusal(Promise.resolve().then(claim))).toEqual([
        "FORBIDDEN",
        "only the receiver records an export's emitter, found destack.build.manifest, destack.installation.id",
    ]);
});
