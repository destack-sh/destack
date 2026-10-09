import { expect, refusal, test } from "@destack/test";
import { eq } from "@destack/db";
import { present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { installation } from "@destack/space/object";
import { alertRule, type AlertRuleDefinition } from "../src/object/index.ts";
import { client, ids, serveObservability } from "./fixture/observability.ts";
import {
    exportLogs,
    logsExport,
    NOTES_EMITTER,
    openNotesBuild,
    recordNotesRevision,
    renameFailure,
} from "./fixture/notes.ts";

/** A metrics export of one histogram point recording 5 ms, at a time in Unix milliseconds. */
function renderDuration(time: number) {
    const nanoseconds = String(BigInt(time) * 1_000_000n);

    return {
        resourceMetrics: [
            {
                scopeMetrics: [
                    {
                        scope: { name: "@example/notes", version: "2026.9.0" },
                        metrics: [
                            {
                                name: "note.render.duration",
                                unit: "ms",
                                exponentialHistogram: {
                                    aggregationTemporality: 1,
                                    dataPoints: [
                                        {
                                            timeUnixNano: nanoseconds,
                                            count: "1",
                                            sum: 5,
                                            min: 5,
                                            max: 5,
                                            scale: 0,
                                            positive: { offset: 2, bucketCounts: ["1"] },
                                        },
                                    ],
                                },
                            },
                        ],
                    },
                ],
            },
        ],
    };
}

test("fire an events condition per crossing group, as a count, a ratio and a percentile over the window, and settle its alert once it clears", async () => {
    const { server, receive, database, failures } = await serveObservability();
    const alice = client(server, "alice");
    const now = Date.now();

    // log three errors and one note, and record a 5 ms render
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([
            { time: now, name: "note.failed", severity: 17 },
            { time: now + 1, name: "note.failed", severity: 17 },
            { time: now + 2, name: "note.failed", severity: 17 },
            { time: now + 3, name: "note.saved", severity: 9 },
        ]),
    );
    await receive(NOTES_EMITTER, "metrics", renderDuration(now));

    // watch the errors per installation, their share of every record, and the slowest renders
    const rule = (name: string, condition: AlertRuleDefinition["condition"]) =>
        alice.alertRule.create({
            spaceId: ids.space,
            requestId: RequestId.create(),
            name,
            condition,
            actions: [{ kind: "notify" }],
        });
    const window = { minutes: 5 };
    const errors = await rule("Errors", {
        kind: "events",
        event: "log",
        where: "severity >= 17",
        fold: "count",
        group: ["installation"],
        window,
        comparison: "above",
        threshold: 2,
    });
    await rule("Error rate", {
        kind: "events",
        event: "log",
        where: "severity >= 17",
        fold: "count",
        group: ["installation"],
        per: {},
        window,
        comparison: "above",
        threshold: 0.5,
    });
    await rule("Slow renders", {
        kind: "events",
        event: "metric",
        where: 'name = "note.render.duration"',
        fold: "p99",
        group: [],
        window,
        comparison: "above",
        threshold: 4,
    });

    // fire once per rule for the notes installation's group, or the space's
    const alerts = async () =>
        (await alice.alert.list({ spaceId: ids.space })).items
            .map(({ title, value, group, status }) => ({ title, value, group, status }))
            .toSorted((left, right) => left.title.localeCompare(right.title));
    await expect.poll(async () => (await alerts()).length).toBe(3);
    const fired = await alerts();

    // settle the errors' alert once its threshold rises above the count
    await alice.alertRule.update({
        spaceId: ids.space,
        requestId: RequestId.create(),
        id: errors.id,
        condition: {
            kind: "events",
            event: "log",
            where: "severity >= 17",
            fold: "count",
            group: ["installation"],
            window,
            comparison: "above",
            threshold: 10,
        },
    });
    await expect
        .poll(async () =>
            (await alerts()).filter((each) => each.status === "resolved").map((each) => each.value),
        )
        .toEqual([3]);
    expect({
        fired,
        failures,
        rules: (await database.select().from(alertRule.table)).length,
    }).toEqual({
        fired: [
            {
                title: `log count 0.75 installation=${ids.notes}`,
                value: 0.75,
                group: { installation: ids.notes },
                status: "firing",
            },
            {
                title: `log count 3 installation=${ids.notes}`,
                value: 3,
                group: { installation: ids.notes },
                status: "firing",
            },
            { title: "metric p99 5", value: 5, group: {}, status: "firing" },
        ],
        failures: [],
        rules: 3,
    });
});

test("refuse an events rule calling an installation its group names none of", async () => {
    const { server } = await serveObservability();

    // call an installation from a rule grouping by nothing
    const refused = await refusal(
        client(server, "alice").alertRule.create({
            spaceId: ids.space,
            requestId: RequestId.create(),
            name: "Refused",
            condition: {
                kind: "events",
                event: "log",
                fold: "count",
                group: [],
                window: { minutes: 5 },
                comparison: "above",
                threshold: 1,
            },
            actions: [{ kind: "call", method: "rollBack" }],
        }),
    );

    // refuse it, naming what a call needs
    expect(refused).toEqual([
        "BAD_REQUEST",
        "a call action needs a group naming an installation or an issue",
    ]);
});

test("roll an installation back through installation.rollBack when a rule watching new issues fires", async () => {
    // serve observability with notes applied at its second revision
    const notesBuild = await openNotesBuild();
    const { server, receive, database, failures } = await serveObservability({
        openBuild: async () => notesBuild,
    });
    const now = Date.now();
    const previous = await recordNotesRevision(database, now - 60_000);
    const current = await recordNotesRevision(database, now);
    await database
        .update(installation.table)
        .set({ revisionId: current, appliedRevisionId: current })
        .where(eq(installation.table.id, ids.notes));
    const alice = client(server, "alice");

    // roll back on every new issue
    await alice.alertRule.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        name: "Roll back new issues",
        condition: { kind: "issue", on: "open" },
        actions: [{ kind: "call", method: "rollBack" }],
    });
    await exportLogs({ receive }, [renameFailure(now, "dana")]);

    // follow the previous revision in the space
    await expect
        .poll(
            async () =>
                (
                    await database
                        .select({ revisionId: installation.table.revisionId })
                        .from(installation.table)
                        .where(eq(installation.table.id, ids.notes))
                ).map(({ revisionId }) => revisionId),
            { timeout: 10_000 },
        )
        .toEqual([previous]);
    expect(failures).toEqual([]);
});

test("keep the alert rules an installation's applied revision declares, and retire one a later revision drops", async () => {
    // serve observability with a notes revision declaring a rule, and a later one declaring none
    const rollback: AlertRuleDefinition = {
        name: "Roll back new fatal issues",
        condition: { kind: "issue", on: "open", filter: "level = fatal" },
        actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
    };
    const builds = new Map([
        ["c".repeat(64), await openNotesBuild({ rollback })],
        ["d".repeat(64), await openNotesBuild()],
    ]);
    const { database } = await serveObservability({
        openBuild: async (_packageId, build) =>
            present(builds.get(build.manifest ?? ""), "the notes build of the revision"),
    });
    const revision = async (manifest: string, createdAt: number) => {
        const id = await recordNotesRevision(database, createdAt, manifest);
        await database
            .update(installation.table)
            .set({ revisionId: id, appliedRevisionId: id })
            .where(eq(installation.table.id, ids.notes));
    };

    // keep the declared rule, managed by the notes installation
    const kept = async () =>
        (await database.select().from(alertRule.table)).map((row) => ({
            name: row.name,
            manager: row.managerInstallationId,
            declaration: row.managerName,
            isRetiring: row.deletionRequestedAt !== null,
        }));
    await revision("c".repeat(64), Date.now() - 60_000);
    await expect.poll(kept, { timeout: 4000 }).toEqual([
        {
            name: "Roll back new fatal issues",
            manager: ids.notes,
            declaration: "rollback",
            isRetiring: false,
        },
    ]);

    // retire it once a later revision declares none
    await revision("d".repeat(64), Date.now());
    await expect.poll(async () => (await kept()).every((rule) => rule.isRetiring)).toBe(true);
});
