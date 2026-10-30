import { eq, asc } from "@destack/db";
import { announcement } from "@destack/notification";
import { installation } from "@destack/object";
import { present } from "@destack/schema";
import objectPackage from "@destack/object/package";
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
    fixture: TestObservability,
    title: string,
    condition: AlertCondition,
    actions: AlertAction[] = [{ kind: "notify" }],
) {
    return fixture.member.client(observabilityPackage).alertRule.create({
        ...alertRule.in(ids.space),
        title,
        condition,
        actions,
        requestId: RequestId.create(fixture.environment),
    });
}

/** Read the space's alerts' statuses, the announcements made, and the responses started. */
async function outcome(fixture: TestObservability) {
    const { database } = fixture.served;
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
    await using fixture = await TestObservability.open();
    const now = fixture.clock.now();

    // log three errors and one note in the last minute, and record a 5 ms render
    await fixture.post(
        "logs",
        fixture.emitter.logs([
            ...errors(now),
            { time: now - 10_000, name: "note.saved", severity: 9 },
        ]),
    );
    await fixture.post(
        "metrics",
        fixture.emitter.metrics([
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
    const created = await rule(fixture, "Errors", ERROR_COUNT);
    await rule(fixture, "Error rate", { ...ERROR_COUNT, per: {}, threshold: 0.5 });
    await rule(fixture, "Slow renders", {
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
    const client = fixture.member.client(observabilityPackage);
    const alerts = async () =>
        (await client.alert.list({ ...alert.in(ids.space) })).items
            .map(({ title, value, labels, status }) => ({ title, value, labels, status }))
            .toSorted((left, right) => left.title.localeCompare(right.title));
    const statuses = async () => (await alerts()).map((each) => each.status);
    await fixture.simulation.until(
        async () => (await statuses()).join() === "firing,firing,firing",
    );
    const fired = await alerts();
    await client.alertRule.update({
        ...alertRule.target(created),
        condition: { ...ERROR_COUNT, threshold: 10 },
        requestId: RequestId.create(fixture.environment),
    });
    await fixture.simulation.until(async () => (await statuses()).includes("resolved"));

    // fire once per rule for the notes installation's group, or the space's, and settle the errors' alert
    const labels = { installationId: fixture.notes };
    expect({ fired, settled: await alerts() }).toEqual({
        fired: [
            {
                title: `Error rate installationId=${fixture.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `Errors installationId=${fixture.notes}`,
                value: 3,
                labels,
                status: "firing",
            },
            { title: "Slow renders", value: 5, labels: {}, status: "firing" },
        ],
        settled: [
            {
                title: `Error rate installationId=${fixture.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `Errors installationId=${fixture.notes}`,
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
    await using fixture = await TestObservability.open();
    await fixture.allow([alertRule.permission("read"), alertRule.permission("write")]);

    // refuse the member's rule rolling back the installation an error count names
    const calling = rule(
        fixture,
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
    await using fixture = await TestObservability.open();

    // let the owner detach observability's installation from the space's stack and withdraw the capability, then serve the space again
    const owner = fixture.space.person(fixture.served, fixture.space.owner).client(objectPackage);
    const [installed] = await fixture.served.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.packageId, observabilityPackage.id));
    const target = { scope: ids.space, id: present(installed, "observability's installation").id };
    await owner.installation.detach({
        ...target,
        requestId: RequestId.create(fixture.environment),
    });
    await owner.installation.update({
        ...target,
        capabilities: [],
        requestId: RequestId.create(fixture.environment),
    });
    await fixture.space.simulation.idle();
    await fixture.restart();

    // refuse the member's rule rolling the installation back
    expect(
        await refusal(
            rule(fixture, "Roll back", ERROR_COUNT, [{ kind: "call", method: "rollBack" }]),
        ),
    ).toEqual([
        "PRECONDITION_FAILED",
        "RESPOND_CAPABILITY_MISSING",
        `a call action needs observability's ${RESPOND_CAPABILITY} capability`,
    ]);
});

test("make the manager who last sets a rule's condition or actions its author, whom its calls act as", async () => {
    // let the member create a rule notifying of an error count
    await using fixture = await TestObservability.open();
    const now = fixture.clock.now();
    const created = await rule(fixture, "Roll back", ERROR_COUNT);
    const author = async () => {
        const [row] = await fixture.served.database
            .select()
            .from(alertRule.table)
            .where(eq(alertRule.table.id, created.id));

        return ObjectReference.at("author", present(row, "the rule"));
    };
    const first = await author();

    // let the space's owner make it roll the installation back, then let it fire on errors
    const owner = fixture.space
        .person(fixture.served, fixture.space.owner)
        .client(observabilityPackage);
    const update = (fields: Partial<Parameters<typeof owner.alertRule.update>[0]>) => ({
        ...alertRule.target(created),
        requestId: RequestId.create(fixture.environment),
        ...fields,
    });
    await owner.alertRule.update(update({ actions: [{ kind: "call", method: "rollBack" }] }));
    const acted = await author();
    await fixture.post("logs", fixture.emitter.logs(errors(now)));
    await fixture.simulation.until(async () => (await outcome(fixture)).responses.length === 1);
    const { responses } = await outcome(fixture);

    // let the owner change only its title, then the member only its condition
    await owner.alertRule.update(update({ title: "Errors" }));
    const titled = await author();
    const member = fixture.member.client(observabilityPackage);
    await member.alertRule.update(update({ condition: { ...ERROR_COUNT, threshold: 10 } }));
    const conditioned = await author();
    await fixture.simulation.until(
        async () => (await outcome(fixture)).statuses.join() === "resolved",
    );

    // past the repeat interval, let the member restore the condition and fire again on new errors
    await fixture.space.simulation.jump(now + REPEAT_MILLISECONDS);
    await member.alertRule.update(update({ condition: ERROR_COUNT }));
    await fixture.post("logs", fixture.emitter.logs(errors(fixture.clock.now())));
    await fixture.simulation.until(async () => (await outcome(fixture)).responses.length === 2);
    const responders = (await outcome(fixture)).responses.map((response) => response.actor);

    // move the author with the actions and the condition, keep it for the title, and respond as it
    expect({
        authors: [first, acted, titled, conditioned],
        first: responses.length,
        responders,
    }).toEqual({
        authors: [
            fixture.member.subject,
            fixture.space.owner.subject,
            fixture.space.owner.subject,
            fixture.member.subject,
        ],
        first: 1,
        responders: [fixture.space.owner.subject, fixture.member.subject],
    });
});

test("declare alert rules as code through the space's stack, checking each rule as it applies", async () => {
    await using fixture = await TestObservability.open();

    // declare an error count rule through the stack
    await fixture.apply([alertRule], {
        alertRules: {
            errors: { title: "Errors", condition: ERROR_COUNT, actions: [{ kind: "notify" }] },
        },
    });
    const rules = await fixture.served.database.select().from(alertRule.table);

    // refuse a declared rule over a type the space serves none of
    const unserved = { ...ERROR_COUNT, event: { packageId: NOTES.id, type: "page-view" } };
    const refused = await fixture.submit([alertRule], {
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
    await using fixture = await TestObservability.open();
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
        rule(fixture, "Calling", condition, [{ kind: "call", method: "rollBack" }]),
    );
    const unserved = await refusal(
        rule(fixture, "Unserved", {
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
    await using fixture = await TestObservability.open();
    const now = fixture.clock.now();
    await fixture.post("logs", fixture.emitter.logs(errors(now)));

    // watch the errors, notifying followers and rolling back the installation after two minutes
    const client = fixture.member.client(observabilityPackage);
    const created = await client.alertRule.create({
        ...alertRule.in(ids.space),
        title: "Errors",
        condition: ERROR_COUNT,
        actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        pending: { minutes: 2 },
        requestId: RequestId.create(fixture.environment),
    });

    // evaluate now, each minute until the pending period ends, and once the threshold rises above the count
    const statuses = async () => (await outcome(fixture)).statuses.join();
    await fixture.simulation.until(async () => (await statuses()) === "pending");
    const pending = (await outcome(fixture)).statuses;
    await fixture.simulation.clock.sleep(now + 2 * 60_000 - fixture.simulation.clock.now());
    await fixture.simulation.idle();
    const fired = await outcome(fixture);
    await client.alertRule.update({
        ...alertRule.target(created),
        condition: { ...ERROR_COUNT, threshold: 10 },
        requestId: RequestId.create(fixture.environment),
    });
    await fixture.simulation.until(async () => (await statuses()) === "resolved");
    const resolved = await outcome(fixture);

    // expect pending, then firing with one response as the member, then resolved with both announcements
    expect({
        statuses: [pending, fired.statuses, resolved.statuses],
        responses: fired.responses,
        announced: resolved.announced,
    }).toEqual({
        statuses: [["pending"], ["firing"], ["resolved"]],
        responses: [{ key: fired.alerts[0]?.id, actor: fixture.member.subject }],
        announced: ["firingAlert", "resolvedAlert"],
    });
});

test("fire an issue rule once per opened issue and once per regression, and not again once its alerts expired", async () => {
    await using fixture = await TestObservability.open();
    const now = fixture.clock.now();
    const client = fixture.member.client(observabilityPackage);
    const fired = async (title: string) => {
        const rules = await fixture.served.database.select().from(alertRule.table);
        const ruleId = present(
            rules.find((row) => row.title === title),
            title,
        ).id;
        const alerts = await fixture.served.database.select().from(alert.table);

        return alerts.filter((row) => row.ruleId === ruleId).map((row) => row.status);
    };

    // watch issues opening and regressing, then fail once, opening an issue
    await rule(fixture, "Opened", { kind: "issue", on: "open" });
    await rule(fixture, "Regressed", { kind: "issue", on: "regress" });
    await fixture.post("logs", fixture.emitter.exceptions([fixture.renameFailure(now, "dana")]));
    await fixture.simulation.until(async () => (await fired("Opened")).length === 1);
    const opened = [await fired("Opened"), await fired("Regressed")];

    // resolve the issue, then fail again, regressing it
    const [row] = (await client.issue.list({ ...issue.in(ids.space) })).items;
    await client.issue.resolve({
        ...issue.target(present(row, "the issue")),
        requestId: RequestId.create(fixture.environment),
        until: { kind: "event" },
    });
    await fixture.post(
        "logs",
        fixture.emitter.exceptions([fixture.renameFailure(now + 10, "dana")]),
    );
    await fixture.simulation.until(async () => (await fired("Regressed")).length === 1);
    const regressed = [await fired("Opened"), await fired("Regressed")];

    // let the resolved alerts expire, evaluating the rules again after
    await fixture.space.simulation.jump(now + 121 * 3_600_000);
    await fixture.simulation.clock.sleep(2 * 60_000);
    await fixture.simulation.idle();
    const expired = [await fired("Opened"), await fired("Regressed")];

    // expect each change answered by one alert, settled at once, and nothing raised again
    expect({ opened, regressed, expired }).toEqual({
        opened: [["resolved"], []],
        regressed: [["resolved"], ["resolved"]],
        expired: [[], []],
    });
});

test("fire a silenced subject's alert without announcing it or responding, and announce it once the silence ends", async () => {
    await using fixture = await TestObservability.open();
    const now = fixture.clock.now();
    await fixture.post("logs", fixture.emitter.logs(errors(now)));

    // silence the notes installation's alerts for two minutes, and watch its errors
    await fixture.member.client(observabilityPackage).silence.create({
        ...silence.in(ids.space),
        matchers: [{ label: "installationId", operator: "=", value: fixture.notes }],
        startsAt: now,
        endsAt: now + 120_000,
        comment: "Deploying notes",
        requestId: RequestId.create(fixture.environment),
    });
    await rule(fixture, "Errors", ERROR_COUNT, [
        { kind: "notify" },
        { kind: "call", method: "rollBack" },
    ]);
    await fixture.simulation.until(
        async () => (await outcome(fixture)).statuses.join() === "firing",
    );
    const silenced = await outcome(fixture);

    // announce it and start its response at the first evaluation after the silence ends
    await fixture.simulation.until(async () => (await outcome(fixture)).announced.length > 0);
    const lapsed = await outcome(fixture);

    expect({
        silenced: [silenced.statuses, silenced.announced.length, silenced.responses.length],
        lapsed: [lapsed.statuses, lapsed.announced.length, lapsed.responses.length],
    }).toEqual({
        silenced: [["firing"], 0, 0],
        lapsed: [["firing"], 1, 1],
    });
});

test("fire an events condition with an empty window when its rule says no data alerts, and stay clear otherwise", async () => {
    await using fixture = await TestObservability.open();

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
    const client = fixture.member.client(observabilityPackage);
    await client.alertRule.create({
        ...alertRule.in(ids.space),
        title: "Missing",
        condition: missing,
        actions: [{ kind: "notify" }],
        noData: "alerting",
        requestId: RequestId.create(fixture.environment),
    });
    await rule(fixture, "Quiet", missing);
    await fixture.simulation.until(async () =>
        (await outcome(fixture)).alerts.some((each) => each.status === "firing"),
    );

    // expect one no-data alert, of the alerting rule
    const { alerts } = await outcome(fixture);
    expect(alerts.map(({ title, labels, status }) => ({ title, labels, status }))).toEqual([
        { title: "Missing no data", labels: {}, status: "firing" },
    ]);
});
