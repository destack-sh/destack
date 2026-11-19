import { testHost } from "@destack/test/host";
import { Digest, schema } from "@destack/schema";
import { ids } from "@destack/machine/test";
import { refusal, expect, test } from "@destack/test";
import {
    ACTION_EVENT,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import { type TestLog, TestEmitter } from "../test/emitter.ts";
import { NOTES } from "../test/notes.ts";
import { EMITTER_ATTRIBUTES, OtlpExport } from "@destack/telemetry/otlp";
import { OtlpResource } from "./otlp.ts";

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

/** A second notes workload instance, logging the same lines at the same times. */
const OTHER_INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-000000000007");

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

/** Decode a logs export in a scope, answering its records' identities. */
async function identities(scope: string, exported: unknown) {
    const decoded = await OtlpResource.decode(
        new OtlpExport("logs", exported),
        scope,
        testHost().crypto,
    );

    return decoded.flatMap((each) => each.events.log.map((record) => record.id));
}

/** Decode an export in the space, answering its refusal. */
function decode(signal: "logs" | "metrics", body: unknown) {
    return refusal(OtlpResource.decode(new OtlpExport(signal, body), ids.space, testHost().crypto));
}

test("decode spans and records under the forwarder's emitter, keeping personal attributes only under a person", async () => {
    // read a span for dana in her session and two records, one of them for nobody, in the notes instance's resource
    const [traced] = await OtlpResource.decode(
        new OtlpExport(
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
        ),
        ids.space,
        testHost().crypto,
    );
    const [logged] = await OtlpResource.decode(
        new OtlpExport(
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
        ),
        ids.space,
        testHost().crypto,
    );

    // stamp each with the forwarder's installation, instance and build, keep dana's email apart and drop the unclaimed token
    const stamp = { installationId: NOTES_INSTALLATION, build: NOTES_MANIFEST };
    const instrumentation = { name: NOTES.name, version: NOTES.version };
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
                span: SPAN,
                parent: null,
                status: "error",
                duration: 2 * MICROSECONDS,
                issue: null,
                session: "session-1",
                person: "dana",
                attributes: {},
                data: {
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
                span: SPAN,
                issue: null,
                session: null,
                person: null,
                attributes: {},
                data: { instance: NOTES_INSTANCE, instrumentation },
            },
            {
                time: (START + 1) * MICROSECONDS,
                ...stamp,
                name: "note.conflict",
                severity: 13,
                trace: null,
                span: null,
                issue: null,
                session: null,
                person: null,
                attributes: { attempts: 2 },
                data: { instance: NOTES_INSTANCE, instrumentation },
            },
        ],
    });
});

test("identify a log record alike on a retry and apart in another scope or instance", async () => {
    // read one export twice in the space, once in another scope, and the same line from another instance
    const line = { time: START, name: "note.saved", severity: 9 };
    const exported = emitter.logs([line]);
    const otherEmitter = emitter.with({ [EMITTER_ATTRIBUTES.instance]: OTHER_INSTANCE });
    const [first, retried, otherScope, otherInstance] = [
        await identities(ids.space, exported),
        await identities(ids.space, exported),
        await identities("space-01996ab0-0000-7000-8000-0000000000ff", exported),
        await identities(ids.space, otherEmitter.logs([line])),
    ];

    // keep a retried export's identity, and give another scope and another instance their own
    expect({ first, retried, otherScope, otherInstance }).toEqual({
        first: ["log-648f9048-5c54-86bd-afd5-bd0fc31f047f"],
        retried: ["log-648f9048-5c54-86bd-afd5-bd0fc31f047f"],
        otherScope: ["log-aea79e82-21f6-8a67-84e0-e1b0ba2fa996"],
        otherInstance: ["log-f43b89a8-fc9d-88d0-ab71-fc229838e460"],
    });
});

test("refuse a cumulative metric", async () => {
    expect(await decode("metrics", metricsExport(START))).toEqual([
        "BAD_REQUEST",
        "TEMPORALITY_INVALID",
        "metric 'note.render.cumulative' is invalid: it is cumulative and must export deltas",
    ]);
});

test("refuse log records and metric points missing what their kind requires, and an explicit-bucket histogram", async () => {
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
        unnamed: ["BAD_REQUEST", "LOG_EVENT_REQUIRED", "log record event name is required"],
        untimed: [
            "BAD_REQUEST",
            "LOG_TIME_REQUIRED",
            "time of log record 'note.saved' is required",
        ],
        unrouted: ["BAD_REQUEST", "VISIT_ROUTE_REQUIRED", "visit route is required"],
        nameless: ["BAD_REQUEST", "ACTION_NAME_REQUIRED", "action name is required"],
        unended: ["BAD_REQUEST", "SESSION_STATUS_REQUIRED", "session end status is required"],
        unknown: [
            "BAD_REQUEST",
            "SESSION_STATUS_INVALID",
            `session status "lost" is invalid: it must be a known status`,
        ],
        unvalued: [
            "BAD_REQUEST",
            "POINT_VALUE_REQUIRED",
            "point value of metric 'note.saves' is required",
        ],
        bucketed: [
            "BAD_REQUEST",
            "METRIC_INVALID",
            "metric 'note.render' is invalid: it must be a delta sum, gauge or exponential histogram",
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
        "PERSON_DENIED",
        "person 'dana' is forbidden: a page record names it, and its machine did not verify it",
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
    const resources = await OtlpResource.decode(
        new OtlpExport("logs", exported),
        ids.space,
        testHost().crypto,
    );

    return resources.flatMap((resource) => resource.events.log.map((event) => event.person));
}
