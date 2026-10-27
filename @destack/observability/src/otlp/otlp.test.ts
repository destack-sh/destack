import { testHost } from "@destack/test/host";
import { Digest, schema } from "@destack/schema";
import { ids } from "@destack/stack/test";
import { refusal, expect, test } from "@destack/test";
import {
    ACTION_EVENT,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import { type TestLog, TestEmitter } from "../test/emitter.ts";
import { NOTES } from "../test/notes.ts";
import { EMITTER_ATTRIBUTES, OtlpExport } from "./otlp.ts";

/** The time the exports record at: 2027-01-15, in Unix milliseconds. */
const START = Date.UTC(2027, 0, 15, 12);

/** The notes installation that emits the exports. */
const NOTES_INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000005");

/** The notes workload instance that emits the exports. */
const NOTES_INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-000000000006");

/** The digest of the notes build's manifest. */
const NOTES_MANIFEST = Digest.parse(
    "8bdf50d6d97eea3aeea1819f64e41b24dfc909ad0d1e5e6edfd7c7ed48b24bd1",
);

/** The notes workload, its exports carrying the emitter its machine forwards them for. */
const emitter = new TestEmitter(
    {
        "service.name": NOTES.name,
        [EMITTER_ATTRIBUTES.installation]: NOTES_INSTALLATION,
        [EMITTER_ATTRIBUTES.instance]: NOTES_INSTANCE,
        [EMITTER_ATTRIBUTES.build]: NOTES_MANIFEST,
    },
    { name: NOTES.name, version: NOTES.version },
);

/** The microseconds in a millisecond. */
const MICROSECONDS = 1000;

/** The trace the scenarios record in, 32 hexadecimal digits. */
const TRACE = "a".repeat(32);

/** The span the scenarios record in, 16 hexadecimal digits. */
const SPAN = "b".repeat(16);

/** A metrics export of a delta counter increment and a cumulative one. */
function metricsExport(time: number) {
    const dataPoints = [{ timeUnixNano: TestEmitter.nanoseconds(time), asInt: "2" }];

    return emitter.metrics([
        { name: "note.saves", sum: { aggregationTemporality: 1, dataPoints } },
        { name: "note.render.cumulative", sum: { aggregationTemporality: 2, dataPoints } },
    ]);
}

/** Decode an export in the space, answering its refusal. */
function decode(signal: "logs" | "metrics", body: unknown) {
    return refusal(new OtlpExport(signal, body).decode(ids.space, testHost().crypto));
}

test("read a resource's spans and records under the emitter its forwarder names, keying their session and sealing personal attributes only under a person", async () => {
    // read a span for dana in her session and two records, one of them for nobody, in the notes instance's resource
    const [traced] = await new OtlpExport(
        "traces",
        emitter.traces([
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
    ).decode(ids.space, testHost().crypto);
    const [logged] = await new OtlpExport(
        "logs",
        emitter.logs([
            { time: START, name: "note.saved", severity: 9, trace: TRACE, span: SPAN },
            {
                time: START + 1,
                name: "note.conflict",
                severity: 13,
                attributes: { attempts: 2, "sensitive.token": "secret" },
            },
        ]),
    ).decode(ids.space, testHost().crypto);

    // stamp each with the forwarder's installation, instance and build, keep dana's email apart and drop the unclaimed token
    const stamp = { installationId: NOTES_INSTALLATION, build: NOTES_MANIFEST };
    const instrumentation = { name: "@example/notes", version: "2026.9.0" };
    expect({
        emitter: traced?.emitter,
        spans: traced?.events.span.map(({ id: _id, ...span }) => span),
        logs: logged?.events.log.map(({ id: _id, ...record }) => record),
    }).toEqual({
        emitter: {
            scope: ids.space,
            installation: NOTES_INSTALLATION,
            instance: NOTES_INSTANCE,
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
                    instance: NOTES_INSTANCE,
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
                data: { span: SPAN, instance: NOTES_INSTANCE, instrumentation },
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
                data: { instance: NOTES_INSTANCE, instrumentation },
            },
        ],
    });
});

test("identify the same records alike in one scope and apart across scopes, and refuse a cumulative metric", async () => {
    // read one export twice in the space and once in another scope
    const exported = emitter.logs([{ time: START, name: "note.saved", severity: 9 }]);
    const identities = async (scope: string) =>
        (await new OtlpExport("logs", exported).decode(scope, testHost().crypto)).flatMap((each) =>
            each.events.log.map((record) => record.id),
        );
    const [first, again, elsewhere] = [
        await identities(ids.space),
        await identities(ids.space),
        await identities("space-01996ab0-0000-7000-8000-0000000000ff"),
    ];

    // keep a retried export's identities, give another scope's its own, and refuse summing a cumulative counter
    expect({
        first,
        again,
        elsewhere,
        cumulative: await refusal(
            new OtlpExport("metrics", metricsExport(START)).decode(ids.space, testHost().crypto),
        ),
    }).toEqual({
        first: ["9a742f480682e247f897575afdb51f21"],
        again: ["9a742f480682e247f897575afdb51f21"],
        elsewhere: ["402cf82284e642ffec1b7b98cdfaf2b8"],
        cumulative: [
            "BAD_REQUEST",
            "TEMPORALITY_UNSUPPORTED",
            "metric note.render.cumulative is cumulative: export deltas",
        ],
    });
});

test("stamp each resource of an export with its forwarder's emitter, refusing an export claiming one", async () => {
    // stamp a resource with its own attributes, and one without any
    const own = { key: "service.name", value: { stringValue: "notes" } };
    const stamped = new OtlpExport("logs", {
        resourceLogs: [{ resource: { attributes: [own] }, scopeLogs: [] }, { scopeLogs: [] }],
    }).stamp({ installation: NOTES_INSTALLATION, build: NOTES_MANIFEST }).body;

    // refuse a resource claiming its build and another installation
    const forged = [
        { key: "destack.build.manifest", value: { stringValue: NOTES_MANIFEST } },
        { key: "destack.installation.id", value: { stringValue: "installation-forged" } },
    ];
    const claim = () =>
        new OtlpExport("logs", {
            resourceLogs: [{ resource: { attributes: [own, ...forged] }, scopeLogs: [] }],
        }).stamp({ installation: NOTES_INSTALLATION });

    // set the known emitter on each resource after its other attributes, and refuse the claim
    const known = [
        { key: "destack.installation.id", value: { stringValue: NOTES_INSTALLATION } },
        { key: "destack.build.manifest", value: { stringValue: NOTES_MANIFEST } },
    ];
    expect(stamped).toEqual({
        resourceLogs: [
            { resource: { attributes: [own, ...known] }, scopeLogs: [] },
            { resource: { attributes: known }, scopeLogs: [] },
        ],
    });
    expect(await refusal(Promise.resolve().then(claim))).toEqual([
        "FORBIDDEN",
        "EMITTER_ATTRIBUTE_RESERVED",
        "only the receiver can record an export's emitter, found destack.build.manifest, destack.installation.id",
    ]);
});

test("refuse records and points missing what their kind requires", async () => {
    // read a logs export of one record, then of the same record without its time
    const logs = (records: readonly TestLog[]) => decode("logs", emitter.logs(records));
    const timed = emitter.logs([{ time: START, name: "note.saved", severity: 9 }]);
    const untimed = {
        resourceLogs: timed.resourceLogs.map((group) => ({
            ...group,
            scopeLogs: group.scopeLogs.map((scoped) => ({
                ...scoped,
                logRecords: scoped.logRecords.map(({ timeUnixNano: _time, ...record }) => record),
            })),
        })),
    };

    // write a delta sum point without a value, and an explicit-bucket histogram
    const points = [{ timeUnixNano: TestEmitter.nanoseconds(START) }];
    const unvalued = emitter.metrics([
        { name: "note.saves", sum: { aggregationTemporality: 1, dataPoints: points } },
    ]);
    const bucketed = emitter.metrics([
        { name: "note.render", histogram: { aggregationTemporality: 1, dataPoints: [] } },
    ]);

    // refuse each with its own reason
    expect({
        unnamed: await logs([{ time: START, name: "" }]),
        untimed: await decode("logs", untimed),
        unrouted: await logs([{ time: START, name: VISIT_EVENT }]),
        nameless: await logs([{ time: START, name: ACTION_EVENT }]),
        unended: await logs([{ time: START, name: SESSION_EVENTS.end }]),
        unknown: await logs([
            {
                time: START,
                name: SESSION_EVENTS.end,
                attributes: { [SESSION_STATUS_ATTRIBUTE]: "lost" },
            },
        ]),
        unvalued: await decode("metrics", unvalued),
        bucketed: await decode("metrics", bucketed),
    }).toEqual({
        unnamed: ["BAD_REQUEST", "LOG_EVENT_MISSING", "the log record has no event name"],
        untimed: ["BAD_REQUEST", "LOG_TIME_MISSING", "log record note.saved has no time"],
        unrouted: ["BAD_REQUEST", "VISIT_ROUTE_MISSING", "the visit has no route"],
        nameless: ["BAD_REQUEST", "ACTION_NAME_MISSING", "the action has no name"],
        unended: ["BAD_REQUEST", "SESSION_STATUS_MISSING", "the session end has no status"],
        unknown: ["BAD_REQUEST", "SESSION_STATUS_INVALID", 'unknown session status "lost"'],
        unvalued: [
            "BAD_REQUEST",
            "POINT_VALUE_MISSING",
            "metric note.saves has a point with no value",
        ],
        bucketed: [
            "BAD_REQUEST",
            "METRIC_UNSUPPORTED",
            "metric note.render is not a delta sum, gauge or exponential histogram",
        ],
    });
});

test("take a page's person from its machine alone, refusing a page record naming another, and a workload's from its records", async () => {
    // export as a page, whose machine stamps no instance, verified as a person or anonymous
    const verifiedPerson = "user-01996ab0-0000-7000-8000-0000000000aa";
    const page = (person?: string) =>
        new TestEmitter(
            {
                [EMITTER_ATTRIBUTES.installation]: NOTES_INSTALLATION,
                [EMITTER_ATTRIBUTES.build]: NOTES_MANIFEST,
                ...(person === undefined ? {} : { [EMITTER_ATTRIBUTES.person]: person }),
            },
            { name: NOTES.name, version: NOTES.version },
        );
    const claimed = [{ time: START, name: "note.saved", attributes: { "enduser.id": "dana" } }];

    // keep the verified person and the person a workload's record names, and refuse a page's claim of dana
    const refusedClaim = [
        "FORBIDDEN",
        "PERSON_UNVERIFIED",
        "a page record names person dana, whom its machine did not verify",
    ];
    expect({
        verified: await personsOf(page(verifiedPerson).logs([{ time: START, name: "note.saved" }])),
        workload: await personsOf(emitter.logs(claimed)),
        forged: await decode("logs", page(verifiedPerson).logs(claimed)),
        anonymous: await decode("logs", page().logs(claimed)),
    }).toEqual({
        verified: [verifiedPerson],
        workload: ["dana"],
        forged: refusedClaim,
        anonymous: refusedClaim,
    });
});

/** Decode a logs export in the space, reading the person of each log record. */
async function personsOf(exported: unknown): Promise<(string | null)[]> {
    const resources = await new OtlpExport("logs", exported).decode(ids.space, testHost().crypto);

    return resources.flatMap((resource) => resource.events.log.map((event) => event.person));
}
