import { eq, asc } from "@destack/db";
import { announcement } from "@destack/notification";
import { installation } from "@destack/object";
import { present } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { ids } from "@destack/stack/test";
import { ObjectReference, RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import observabilityPackage from "../package.ts";
import { NOTES, TestEmitter, type TestLog, TestObservability } from "../test/index.ts";
import {
    alert,
    type AlertAction,
    type AlertCondition,
    alertRule,
    issue,
    log,
    metric,
    respond,
    RESPOND_CAPABILITY,
    silence,
} from "./index.ts";

/** The time past a rule's repeat interval of half an hour, after which a subject alerts again. */
const REPEAT_MILLISECONDS = 31 * 60_000;

/** A condition over events, as a rule declares it. */
type EventsCondition = Extract<AlertCondition, { readonly kind: "events" }>;

/** More than two error records per installation over five minutes. */
const ERROR_COUNT: EventsCondition = {
    kind: "events",
    event: log.typeReference,
    where: "severity >= 17",
    fold: "count",
    group: ["installationId"],
    window: { minutes: 5 },
    comparison: "above",
    threshold: 2,
};

/** Write three error records of the notes installation within the minute before a time. */
function errors(now: number): TestLog[] {
    return [40, 30, 20].map((seconds) => ({
        time: now - seconds * 1000,
        name: "note.failed",
        severity: 17,
    }));
}

/** Create an alert rule in the space as the member, notifying its followers. */
function rule(
    kit: TestObservability,
    title: string,
    condition: AlertCondition,
    actions: AlertAction[] = [{ kind: "notify" }],
) {
    return kit.member.client(observabilityPackage).alertRule.create({
        ...alertRule.in(ids.space),
        title,
        condition,
        actions,
        requestId: RequestId.create(kit.environment),
    });
}

/** Read the space's alerts' statuses, the announcements made, and the responses started. */
async function outcome(kit: TestObservability) {
    const { database } = kit.served;
    const alerts = await database.select().from(alert.table);
    const announced = await database.select().from(announcement.table);
    const responses = await database
        .select()
        .from(respond.run.table)
        .orderBy(asc(respond.run.table.createdAt), asc(respond.run.table.id));

    return {
        statuses: alerts.map((row) => row.status),
        announced: announced.map((row) => row.name).toSorted(),
        responses: responses.map((row) => ({
            key: row["key"],
            actor: ObjectReference.at("actor", row),
        })),
        alerts,
    };
}

test("fire an events condition per crossing group, as a count, a ratio and a percentile over the window, and resolve its alert once it clears", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();

    // log three errors and one note in the last minute, and record a 5 ms render
    await kit.post(
        "logs",
        kit.emitter.logs([...errors(now), { time: now - 10_000, name: "note.saved", severity: 9 }]),
    );
    await kit.post(
        "metrics",
        kit.emitter.metrics([
            {
                name: "note.render.duration",
                unit: "ms",
                exponentialHistogram: {
                    aggregationTemporality: 1,
                    dataPoints: [
                        {
                            attributes: [],
                            startTimeUnixNano: TestEmitter.nanoseconds(now - 10_000),
                            timeUnixNano: TestEmitter.nanoseconds(now - 10_000),
                            count: "1",
                            sum: 5,
                            min: 5,
                            max: 5,
                            scale: 0,
                            zeroCount: "0",
                            positive: { offset: 2, bucketCounts: ["1"] },
                        },
                    ],
                },
            },
        ]),
    );

    // watch the errors per installation, their share of every record, and the slowest renders
    const created = await rule(kit, "Errors", ERROR_COUNT);
    await rule(kit, "Error rate", { ...ERROR_COUNT, per: {}, threshold: 0.5 });
    await rule(kit, "Slow renders", {
        kind: "events",
        event: metric.typeReference,
        where: 'name = "note.render.duration"',
        fold: "p99",
        group: [],
        window: { minutes: 5 },
        comparison: "above",
        threshold: 4,
    });

    // evaluate the rules, then raise the errors' threshold above the count and evaluate again
    const client = kit.member.client(observabilityPackage);
    const alerts = async () =>
        (await client.alert.list({ ...alert.in(ids.space) })).items
            .map(({ title, value, labels, status }) => ({ title, value, labels, status }))
            .toSorted((left, right) => left.title.localeCompare(right.title));
    const statuses = async () => (await alerts()).map((each) => each.status);
    await kit.simulation.run({
        until: async () => (await statuses()).join() === "firing,firing,firing",
    });
    const fired = await alerts();
    await client.alertRule.update({
        ...alertRule.target(created),
        condition: { ...ERROR_COUNT, threshold: 10 },
        requestId: RequestId.create(kit.environment),
    });
    await kit.simulation.run({ until: async () => (await statuses()).includes("resolved") });

    // fire once per rule for the notes installation's group, or the space's, and settle the errors' alert
    const labels = { installationId: kit.notes };
    expect({ fired, settled: await alerts() }).toEqual({
        fired: [
            {
                title: `Error rate installationId=${kit.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `Errors installationId=${kit.notes}`,
                value: 3,
                labels,
                status: "firing",
            },
            { title: "Slow renders", value: 5, labels: {}, status: "firing" },
        ],
        settled: [
            {
                title: `Error rate installationId=${kit.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `Errors installationId=${kit.notes}`,
                value: 3,
                labels,
                status: "resolved",
            },
            { title: "Slow renders", value: 5, labels: {}, status: "firing" },
        ],
    });
});

test("refuse a rule calling an installation's method from an editor who does not manage alert rules", async () => {
    // let the member edit alert rules without managing them
    await using kit = await TestObservability.open();
    await kit.allow([alertRule.permission("read"), alertRule.permission("write")]);

    // refuse the member's rule rolling back the installation an error count names
    const calling = rule(
        kit,
        "Roll back",
        {
            kind: "events",
            event: log.typeReference,
            fold: "count",
            group: ["installationId"],
            window: { minutes: 5 },
            comparison: "above",
            threshold: 1,
        },
        [{ kind: "call", method: "rollBack" }],
    );
    expect(await refusal(calling)).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission denied: manage",
    ]);
});

test("refuse a rule calling an installation once the space's owner withdraws observability's respond capability", async () => {
    await using kit = await TestObservability.open();

    // let the owner withdraw the capability through the space's stack, and serve the space again
    const owner = kit.space.person(kit.served, kit.space.owner).client(stackPackage);
    const [installed] = await kit.served.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.packageId, observabilityPackage.id));
    await owner.rollout.update({
        spaceId: ids.space,
        id: present(installed, "observability's installation").id,
        capabilities: [],
        requestId: RequestId.create(kit.environment),
    });
    await kit.space.settle(kit.served);
    await kit.restart();

    // refuse the member's rule rolling the installation back
    expect(
        await refusal(rule(kit, "Roll back", ERROR_COUNT, [{ kind: "call", method: "rollBack" }])),
    ).toEqual([
        "PRECONDITION_FAILED",
        "RESPOND_CAPABILITY_MISSING",
        `a call action needs observability's ${RESPOND_CAPABILITY} capability`,
    ]);
});

test("make the manager who last sets a rule's condition or actions its author, whom its calls act as", async () => {
    // let the member create a rule notifying of an error count
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    const created = await rule(kit, "Roll back", ERROR_COUNT);
    const author = async () => {
        const [row] = await kit.served.database
            .select()
            .from(alertRule.table)
            .where(eq(alertRule.table.id, created.id));

        return ObjectReference.at("author", present(row, "the rule"));
    };
    const first = await author();

    // let the space's owner make it roll the installation back, then let it fire on errors
    const owner = kit.space.person(kit.served, kit.space.owner).client(observabilityPackage);
    const update = (fields: Partial<Parameters<typeof owner.alertRule.update>[0]>) => ({
        ...alertRule.target(created),
        requestId: RequestId.create(kit.environment),
        ...fields,
    });
    await owner.alertRule.update(update({ actions: [{ kind: "call", method: "rollBack" }] }));
    const acted = await author();
    await kit.post("logs", kit.emitter.logs(errors(now)));
    await kit.simulation.run({
        until: async () => (await outcome(kit)).responses.length === 1,
    });
    const { responses } = await outcome(kit);

    // let the owner change only its title, then the member only its condition
    await owner.alertRule.update(update({ title: "Errors" }));
    const titled = await author();
    const member = kit.member.client(observabilityPackage);
    await member.alertRule.update(update({ condition: { ...ERROR_COUNT, threshold: 10 } }));
    const conditioned = await author();
    await kit.simulation.run({
        until: async () => (await outcome(kit)).statuses.join() === "resolved",
    });

    // past the repeat interval, let the member restore the condition and fire again on new errors
    await kit.space.jump(kit.served, now + REPEAT_MILLISECONDS);
    await member.alertRule.update(update({ condition: ERROR_COUNT }));
    await kit.post("logs", kit.emitter.logs(errors(kit.clock.now())));
    await kit.simulation.run({
        until: async () => (await outcome(kit)).responses.length === 2,
    });
    const responders = (await outcome(kit)).responses.map((response) => response.actor);

    // move the author with the actions and the condition, keep it for the title, and respond as it
    expect({
        authors: [first, acted, titled, conditioned],
        first: responses.length,
        responders,
    }).toEqual({
        authors: [
            kit.member.subject,
            kit.space.owner.subject,
            kit.space.owner.subject,
            kit.member.subject,
        ],
        first: 1,
        responders: [kit.space.owner.subject, kit.member.subject],
    });
});

test("declare alert rules as code through the space's stack, checking each rule as it applies", async () => {
    await using kit = await TestObservability.open();

    // declare an error count rule through the stack
    await kit.apply([alertRule], {
        alertRules: {
            errors: { title: "Errors", condition: ERROR_COUNT, actions: [{ kind: "notify" }] },
        },
    });
    const rules = await kit.served.database.select().from(alertRule.table);

    // refuse a declared rule over a type the space serves none of
    const unserved = { ...ERROR_COUNT, event: { packageId: NOTES.id, type: "page-view" } };
    const refused = await kit.submit([alertRule], {
        alertRules: {
            pages: { title: "Pages", condition: unserved, actions: [{ kind: "notify" }] },
        },
    });

    // write the declared rule with its defaults, and keep the stack from applying the unserved one
    expect(
        rules.map(({ title, condition, actions, interval, pending, noData }) => ({
            title,
            condition,
            actions,
            interval,
            pending,
            noData,
        })),
    ).toEqual([
        {
            title: "Errors",
            condition: ERROR_COUNT,
            actions: [{ kind: "notify" }],
            interval: { minutes: 30 },
            pending: { seconds: 0 },
            noData: "ok",
        },
    ]);
    expect(refused).toEqual(["ApplyFailed", "the space has no events type page-view"]);
});

test("refuse an events rule calling an installation its group names none of, and one over a type the space serves none of", async () => {
    await using kit = await TestObservability.open();
    const condition: EventsCondition = {
        kind: "events",
        event: log.typeReference,
        fold: "count",
        group: [],
        window: { minutes: 5 },
        comparison: "above",
        threshold: 1,
    };

    // call an installation from a rule grouping by nothing, and watch a type no package serves here
    const calling = await refusal(
        rule(kit, "Calling", condition, [{ kind: "call", method: "rollBack" }]),
    );
    const unserved = await refusal(
        rule(kit, "Unserved", {
            ...condition,
            event: { packageId: NOTES.id, type: "page-view" },
        }),
    );

    // refuse both, naming what is missing
    expect({ calling, unserved }).toEqual({
        calling: [
            "BAD_REQUEST",
            "CALL_GROUP_MISSING",
            "a call action needs a group by installation or issue",
        ],
        unserved: ["BAD_REQUEST", "EVENTS_TYPE_UNKNOWN", "the space has no events type page-view"],
    });
});

test("keep an alert pending for its rule's pending period, then fire, announce and respond as the rule's author, and announce its resolution once clear", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    await kit.post("logs", kit.emitter.logs(errors(now)));

    // watch the errors, notifying followers and rolling back the installation after two minutes
    const client = kit.member.client(observabilityPackage);
    const created = await client.alertRule.create({
        ...alertRule.in(ids.space),
        title: "Errors",
        condition: ERROR_COUNT,
        actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        pending: { minutes: 2 },
        requestId: RequestId.create(kit.environment),
    });

    // evaluate now, each minute until the pending period ends, and once the threshold rises above the count
    const statuses = async () => (await outcome(kit)).statuses.join();
    await kit.simulation.run({ until: async () => (await statuses()) === "pending" });
    const pending = (await outcome(kit)).statuses;
    await kit.simulation.run({ to: now + 2 * 60_000 });
    const fired = await outcome(kit);
    await client.alertRule.update({
        ...alertRule.target(created),
        condition: { ...ERROR_COUNT, threshold: 10 },
        requestId: RequestId.create(kit.environment),
    });
    await kit.simulation.run({ until: async () => (await statuses()) === "resolved" });
    const resolved = await outcome(kit);

    // expect pending, then firing with one response as the member, then resolved with both announcements
    expect({
        statuses: [pending, fired.statuses, resolved.statuses],
        responses: fired.responses,
        announced: resolved.announced,
    }).toEqual({
        statuses: [["pending"], ["firing"], ["resolved"]],
        responses: [{ key: fired.alerts[0]?.id, actor: kit.member.subject }],
        announced: ["firingAlert", "resolvedAlert"],
    });
});

test("fire an issue rule once per opened issue and once per regression, and not again once its alerts expired", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    const client = kit.member.client(observabilityPackage);
    const fired = async (title: string) => {
        const rules = await kit.served.database.select().from(alertRule.table);
        const ruleId = present(
            rules.find((row) => row.title === title),
            title,
        ).id;
        const alerts = await kit.served.database.select().from(alert.table);

        return alerts.filter((row) => row.ruleId === ruleId).map((row) => row.status);
    };

    // watch issues opening and regressing, then fail once, opening an issue
    await rule(kit, "Opened", { kind: "issue", on: "open" });
    await rule(kit, "Regressed", { kind: "issue", on: "regress" });
    await kit.post("logs", kit.emitter.exceptions([kit.renameFailure(now, "dana")]));
    await kit.simulation.run({ until: async () => (await fired("Opened")).length === 1 });
    const opened = [await fired("Opened"), await fired("Regressed")];

    // resolve the issue, then fail again, regressing it
    const [row] = (await client.issue.list({ ...issue.in(ids.space) })).items;
    await client.issue.resolve({
        ...issue.target(present(row, "the issue")),
        requestId: RequestId.create(kit.environment),
        until: { kind: "event" },
    });
    await kit.post("logs", kit.emitter.exceptions([kit.renameFailure(now + 10, "dana")]));
    await kit.simulation.run({ until: async () => (await fired("Regressed")).length === 1 });
    const regressed = [await fired("Opened"), await fired("Regressed")];

    // let the resolved alerts expire, evaluating the rules again after
    await kit.space.jump(kit.served, now + 121 * 3_600_000);
    await kit.simulation.run({ to: kit.clock.now() + 2 * 60_000 });
    const expired = [await fired("Opened"), await fired("Regressed")];

    // expect each change answered by one alert, settled at once, and nothing raised again
    expect({ opened, regressed, expired }).toEqual({
        opened: [["resolved"], []],
        regressed: [["resolved"], ["resolved"]],
        expired: [[], []],
    });
});

test("fire a silenced subject's alert without announcing it or responding, and announce it once the silence ends", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    await kit.post("logs", kit.emitter.logs(errors(now)));

    // silence the notes installation's alerts for two minutes, and watch its errors
    await kit.member.client(observabilityPackage).silence.create({
        ...silence.in(ids.space),
        matchers: [{ label: "installationId", operator: "=", value: kit.notes }],
        startsAt: now,
        endsAt: now + 120_000,
        comment: "Deploying notes",
        requestId: RequestId.create(kit.environment),
    });
    await rule(kit, "Errors", ERROR_COUNT, [
        { kind: "notify" },
        { kind: "call", method: "rollBack" },
    ]);
    await kit.simulation.run({
        until: async () => (await outcome(kit)).statuses.join() === "firing",
    });
    const silenced = await outcome(kit);

    // announce it and start its response at the first evaluation after the silence ends
    await kit.simulation.run({ until: async () => (await outcome(kit)).announced.length > 0 });
    const lapsed = await outcome(kit);

    expect({
        silenced: [silenced.statuses, silenced.announced.length, silenced.responses.length],
        lapsed: [lapsed.statuses, lapsed.announced.length, lapsed.responses.length],
    }).toEqual({
        silenced: [["firing"], 0, 0],
        lapsed: [["firing"], 1, 1],
    });
});

test("fire an events condition with an empty window when its rule says no data alerts, and stay clear otherwise", async () => {
    await using kit = await TestObservability.open();

    // watch a metric nothing records, alerting on no data, and the same counting no data as clear
    const missing: EventsCondition = {
        kind: "events",
        event: metric.typeReference,
        where: 'name = "note.missing"',
        fold: "count",
        group: [],
        window: { minutes: 5 },
        comparison: "above",
        threshold: 0,
    };
    const client = kit.member.client(observabilityPackage);
    await client.alertRule.create({
        ...alertRule.in(ids.space),
        title: "Missing",
        condition: missing,
        actions: [{ kind: "notify" }],
        noData: "alerting",
        requestId: RequestId.create(kit.environment),
    });
    await rule(kit, "Quiet", missing);
    await kit.simulation.run({ until: async () => (await outcome(kit)).alerts.length > 0 });

    // expect one no-data alert, of the alerting rule
    const { alerts } = await outcome(kit);
    expect(alerts.map(({ title, labels, status }) => ({ title, labels, status }))).toEqual([
        { title: "Missing no data", labels: {}, status: "firing" },
    ]);
});
