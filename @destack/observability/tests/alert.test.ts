import { AccessContext, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import * as accountObject from "@destack/account/object";
import { Scope } from "@destack/sync";
import { EventTime } from "@destack/event";
import { expect, refusal, test } from "@destack/test";
import { log, metric } from "../src/object/index.ts";
import {
    alert,
    alertRule,
    type AlertRuleDefinition,
    respond,
    silence,
} from "../src/object/index.ts";
import { announcement } from "@destack/notification";
import { ObjectReference } from "@destack/sync";
import { NOTES_MANIFEST } from "./fixture/notes.ts";
import {
    actors,
    ids,
    mutate,
    type ObservedSpace,
    openSpace,
    reconcile,
    START,
} from "./fixture/space.ts";

/** A condition over events, as a rule declares it. */
type EventsCondition = Extract<AlertRuleDefinition["condition"], { readonly kind: "events" }>;

/** The library the notes workload records with. */
const INSTRUMENTATION = { name: "@example/notes", version: "2026.9.0" };

/** A log record of the notes installation, a number of seconds before the start. */
function record(id: string, seconds: number, name: string, severity: number) {
    return {
        scope: ids.space,
        id,
        time: EventTime.of(START - seconds * 1000),
        keys: {
            installationId: ids.notes,
            build: NOTES_MANIFEST,
            name,
            severity,
            trace: null,
            issue: null,
            session: null,
            person: null,
            attributes: {},
        },
        data: { instrumentation: INSTRUMENTATION },
    };
}

/** Create an alert rule in the space as alice, notifying its followers. */
function rule(
    space: ObservedSpace,
    title: string,
    condition: AlertRuleDefinition["condition"],
    actions: AlertRuleDefinition["actions"] = [{ kind: "notify" }],
) {
    return mutate(space.store, actors.alice, [
        alertRule.call("create", { ...alertRule.in(ids.space), title, condition, actions }),
    ]);
}

test("fire an events condition per crossing group, as a count, a ratio and a percentile over the window, and settle its alert once it clears", async () => {
    const space = await openSpace();

    // log three errors and one note in the last minute, and record a 5 ms render
    await space.events.append(log.eventKind(), [
        record("failed-1", 40, "note.failed", 17),
        record("failed-2", 30, "note.failed", 17),
        record("failed-3", 20, "note.failed", 17),
        record("saved-1", 10, "note.saved", 9),
    ]);
    await space.events.append(metric.eventKind(), [
        {
            scope: ids.space,
            id: "render-1",
            time: EventTime.of(START - 10_000),
            keys: {
                installationId: ids.notes,
                build: NOTES_MANIFEST,
                name: "note.render.duration",
                instrument: "histogram",
                value: 5,
                count: 1,
                attributes: {},
            },
            data: {
                start: EventTime.of(START - 10_000),
                unit: "ms",
                histogram: {
                    count: 1,
                    sum: 5,
                    min: 5,
                    max: 5,
                    scale: 0,
                    zeroCount: 0,
                    positive: { offset: 2, counts: [1] },
                    negative: { offset: 0, counts: [] },
                },
                instrumentation: INSTRUMENTATION,
            },
        },
    ]);

    // watch the errors per installation, their share of every record, and the slowest renders
    const window = { minutes: 5 };
    const errors: EventsCondition = {
        kind: "events",
        event: log.typeReference,
        where: "severity >= 17",
        fold: "count",
        group: ["installationId"],
        window,
        comparison: "above",
        threshold: 2,
    };
    const [created] = await rule(space, "Errors", errors);
    await rule(space, "Error rate", { ...errors, per: {}, threshold: 0.5 });
    await rule(space, "Slow renders", {
        kind: "events",
        event: metric.typeReference,
        where: 'name = "note.render.duration"',
        fold: "p99",
        group: [],
        window,
        comparison: "above",
        threshold: 4,
    });

    // evaluate the rules, then raise the errors' threshold above the count and evaluate again
    const alerts = async () =>
        (
            await space.store.query(
                AccessContext.of(Caller.of(actors.alice), Date.now()),
                alert.call("list", { ...alert.in(ids.space) }),
            )
        ).items
            .map(({ title, value, labels, status }) => ({ title, value, labels, status }))
            .toSorted((left, right) => left.title.localeCompare(right.title));
    await reconcile(space, "alert-rule");
    const fired = await alerts();
    await mutate(space.store, actors.alice, [
        alertRule.call("update", {
            ...alertRule.target(created),
            condition: { ...errors, threshold: 10 },
        }),
    ]);
    await reconcile(space, "alert-rule");

    // fire once per rule for the notes installation's group, or the space's, and settle the errors' alert
    const labels = { installationId: ids.notes };
    expect({ fired, settled: await alerts(), failures: space.failures }).toEqual({
        fired: [
            {
                title: `log count 0.75 installationId=${ids.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `log count 3 installationId=${ids.notes}`,
                value: 3,
                labels,
                status: "firing",
            },
            { title: "metric p99 5", value: 5, labels: {}, status: "firing" },
        ],
        settled: [
            {
                title: `log count 0.75 installationId=${ids.notes}`,
                value: 0.75,
                labels,
                status: "firing",
            },
            {
                title: `log count 3 installationId=${ids.notes}`,
                value: 3,
                labels,
                status: "resolved",
            },
            { title: "metric p99 5", value: 5, labels: {}, status: "firing" },
        ],
        failures: [],
    });
});

test("refuse a rule calling an installation's method from an editor who does not manage alert rules", async () => {
    const observed = await openSpace();

    // let bob edit alert rules without managing them
    const bob = principal.user.reference(Scope.universe.id, "bob");
    await new AccessFixture(observed.store.database).replicateRole(
        accountObject.space.reference(ids.account, ids.space),
        {
            name: "editor",
            description: "Edits alert rules",
            permissions: [alertRule.permission("read"), alertRule.permission("write")],
        },
        bob,
        0,
    );

    // refuse his rule rolling back the installation an error rate names
    const calling = mutate(observed.store, bob, [
        alertRule.call("create", {
            ...alertRule.in(ids.space),
            title: "Roll back",
            condition: {
                kind: "events",
                event: log.typeReference,
                fold: "count",
                group: ["installationId"],
                window: { minutes: 5 },
                comparison: "above",
                threshold: 1,
            },
            actions: [{ kind: "call", method: "rollBack" }],
        }),
    ]);
    expect(await refusal(calling)).toEqual(["FORBIDDEN", "permission denied: manage"]);
});

test("refuse an events rule calling an installation its group names none of, and one over a type the space serves none of", async () => {
    const space = await openSpace();
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
        rule(space, "Calling", condition, [{ kind: "call", method: "rollBack" }]),
    );
    const unserved = await refusal(
        rule(space, "Unserved", {
            ...condition,
            event: { packageId: ids.package, type: "page-view" },
        }),
    );

    // refuse both, naming what is missing
    expect({ calling, unserved }).toEqual({
        calling: ["BAD_REQUEST", "a call action needs a group by installation or issue"],
        unserved: ["BAD_REQUEST", "the space serves no events type page-view"],
    });
});

/** Three error records of the notes installation within the last minute. */
const ERRORS = [
    record("failed-1", 40, "note.failed", 17),
    record("failed-2", 30, "note.failed", 17),
    record("failed-3", 20, "note.failed", 17),
];

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

/** Read the space's alerts' statuses, the announcements made, and the responses started. */
async function outcome(space: ObservedSpace) {
    const alerts = await space.store.database.select().from(alert.table);
    const announced = await space.store.database.select().from(announcement.table);
    const responses = await space.store.database.select().from(respond.run.table);

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

test("hold an alert pending for its rule's pending period, then fire, announce and respond as the rule's author, and announce its resolution once clear", async () => {
    const space = await openSpace();
    await space.events.append(log.eventKind(), ERRORS);

    // watch the errors, notifying followers and rolling back the installation after two minutes
    const [created] = await mutate(space.store, actors.alice, [
        alertRule.call("create", {
            ...alertRule.in(ids.space),
            title: "Errors",
            condition: ERROR_COUNT,
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
            pending: { minutes: 2 },
        }),
    ]);

    // evaluate now, after the pending period, and once the threshold rises above the count
    await reconcile(space, "alert-rule");
    const pending = (await outcome(space)).statuses;
    space.clock.advance(2 * 60_000);
    await reconcile(space, "alert-rule");
    const fired = await outcome(space);
    await mutate(space.store, actors.alice, [
        alertRule.call("update", {
            ...alertRule.target(created),
            condition: { ...ERROR_COUNT, threshold: 10 },
        }),
    ]);
    await reconcile(space, "alert-rule");
    const resolved = await outcome(space);

    // expect pending, then firing with one response as alice, then resolved with both announcements
    expect({
        statuses: [pending, fired.statuses, resolved.statuses],
        responses: fired.responses,
        announced: resolved.announced,
    }).toEqual({
        statuses: [["pending"], ["firing"], ["resolved"]],
        responses: [{ key: fired.alerts[0]?.id, actor: actors.alice }],
        announced: ["firingAlert", "resolvedAlert"],
    });
});

test("fire a silenced subject's alert without announcing it or responding", async () => {
    const space = await openSpace();
    await space.events.append(log.eventKind(), ERRORS);

    // silence the notes installation's alerts for an hour, and watch its errors
    await mutate(space.store, actors.alice, [
        silence.call("create", {
            ...silence.in(ids.space),
            matchers: [{ label: "installationId", operator: "=", value: ids.notes }],
            startsAt: START,
            endsAt: START + 3_600_000,
            comment: "Deploying notes",
        }),
    ]);
    await rule(space, "Errors", ERROR_COUNT, [
        { kind: "notify" },
        { kind: "call", method: "rollBack" },
    ]);
    await reconcile(space, "alert-rule");

    // expect the alert firing, and nothing announced or started
    const { statuses, announced, responses } = await outcome(space);
    expect({ statuses, announced, responses }).toEqual({
        statuses: ["firing"],
        announced: [],
        responses: [],
    });
});

test("fire an events condition with an empty window when its rule says no data alerts, and stay clear otherwise", async () => {
    const space = await openSpace();

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
    await mutate(space.store, actors.alice, [
        alertRule.call("create", {
            ...alertRule.in(ids.space),
            title: "Missing",
            condition: missing,
            actions: [{ kind: "notify" }],
            noData: "alerting",
        }),
        alertRule.call("create", {
            ...alertRule.in(ids.space),
            title: "Quiet",
            condition: missing,
            actions: [{ kind: "notify" }],
        }),
    ]);
    await reconcile(space, "alert-rule");

    // expect one no-data alert, of the alerting rule
    const { alerts } = await outcome(space);
    expect(alerts.map(({ title, labels, status }) => ({ title, labels, status }))).toEqual([
        { title: "metric no data", labels: {}, status: "firing" },
    ]);
});
