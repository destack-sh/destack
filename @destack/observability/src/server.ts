import { Subscription } from "@destack/notification";
import { ObjectWatch } from "@destack/object";
import { Snapshot } from "@destack/db";
import { EventTime } from "@destack/event";
import { implementPackage } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import { installationRevision } from "@destack/stack/object";
import { ObjectReference, RequestId } from "@destack/sync";
import {
    alert,
    alertRule,
    AlertRule,
    NO_PENDING,
    EVALUATION_MILLISECONDS,
    Issue,
    issue,
    exception,
    issuePerson,
    regressedIssue,
    resolvedIssue,
    respond,
    silence,
    Until,
} from "./object/index.ts";
import { OtlpIntake } from "./otlp/intake.ts";
import observabilityPackage from "./package.ts";

/** The fields deciding what an alert rule's calls do and when, which their editor authors. */
const AUTHORED_FIELDS = ["condition", "actions", "interval", "pending", "noData"] as const;

/** The responses to fired alerts: their rules' calls, each waiting on any approval it needs. */
const responses = respond.implement(async (step, input, run) => {
    // read the alert and its rule, and make the rule's calls
    const fired = await step.query(
        "alert",
        alert.call("get", alert.target({ scope: run.scope, id: input.alert })),
    );
    const rule = await step.query(
        "rule",
        alertRule.call("get", alertRule.target({ scope: run.scope, id: fired.ruleId })),
    );
    await new AlertRule(rule).act(step, fired);
});

/** The observability package's server: issues, alert rules, alerts, and the OTLP intake. */
export default implementPackage(observabilityPackage, {
    objects: [
        issue.implement({
            methods: {
                record: async (call) => {
                    // find the issue of the fingerprint, or open it at its first exceptions
                    const { issueId: id, opening, occurrences, revisedAt } = call.input;
                    const scope = schema.identifier("space").parse(call.scope);
                    const found = await call.read(issue).findFirst({ where: { scope, id } });
                    const [opened] =
                        found === undefined
                            ? await call.mutate(call.caller, [
                                  issue.call("open", {
                                      ...issue.in(scope),
                                      id,
                                      ...opening,
                                      until: { kind: "event" },
                                  }),
                              ])
                            : [found];

                    // gather the people the exceptions affected, with when each last saw one
                    const people = new Map<string, number>();
                    for (const { person, at } of occurrences) {
                        if (person !== undefined) {
                            people.set(person, Math.max(people.get(person) ?? 0, at));
                        }
                    }

                    // count them and the people they affected into the issue, seen last at the latest of them
                    const { level, lastRevisionId } = opening;
                    const lastSeenAt = Math.max(...occurrences.map((each) => each.at));
                    const count = occurrences.length;
                    const affected = [...people].map(([person, seenAt]) => ({
                        person,
                        lastSeenAt: seenAt,
                    }));
                    const [counted] = await call.mutate(call.caller, [
                        issue.call("occur", {
                            ...issue.target(present(opened, `issue ${id}`)),
                            level,
                            lastSeenAt,
                            ...(lastRevisionId === undefined ? {} : { lastRevisionId }),
                            count,
                            people: affected,
                        }),
                    ]);

                    // regress a resolved issue and escalate an ignored one its condition reopens
                    const { until } = counted;
                    const after =
                        until.kind === "revision"
                            ? await call
                                  .read(installationRevision)
                                  .findFirst({ where: { id: until.after } })
                            : undefined;
                    const isReopened = Issue.reopens(counted, {
                        lastSeenAt,
                        revisedAt,
                        afterAt: after?.createdAt,
                    });
                    if (isReopened) {
                        const transition = counted.status === "resolved" ? "regress" : "escalate";
                        await call.mutate(call.caller, [
                            issue.call(transition, issue.target(counted)),
                        ]);
                    }

                    return {};
                },
                count: async (call) => {
                    // read the claimed page of the space's exceptions, grouped by issue
                    const { after, through } = call.input;
                    const kind = exception.eventKind();
                    const scope = schema.identifier("space").parse(call.scope);
                    const page = await call.events(kind).range({ scopes: [scope] }, after, through);
                    const read = page.map((event) => ({
                        at: EventTime.milliseconds(event.time),
                        keys: kind.parseKeys(event.keys),
                        data: kind.parseData(event.data),
                    }));

                    // record each issue's exceptions
                    const calls = [...Map.groupBy(read, (each) => each.keys.issue)].map(
                        ([id, recorded]) => {
                            // open it at the earliest exception, the severest level and the latest revision
                            const first = present(recorded[0], `an exception of issue ${id}`);
                            const revisions = recorded.flatMap((each) => each.data.revisedAt ?? []);
                            const opening = {
                                ...first.data.opening,
                                level: recorded
                                    .map((each) => each.data.opening.level)
                                    .reduce((left, right) => Issue.severest(left, right)),
                                firstSeenAt: Math.min(...recorded.map((each) => each.at)),
                                lastSeenAt: Math.max(...recorded.map((each) => each.at)),
                            };

                            return issue.call("record", {
                                ...issue.in(scope),
                                issueId: issue.identifier(id),
                                opening,
                                occurrences: recorded.map((each) => ({
                                    ...(each.keys.person === null
                                        ? {}
                                        : { person: each.keys.person }),
                                    at: each.at,
                                })),
                                ...(revisions.length === 0
                                    ? {}
                                    : { revisedAt: Math.max(...revisions) }),
                            });
                        },
                    );
                    if (calls.length > 0) {
                        await call.mutate(call.caller, calls);
                    }

                    return {};
                },
                occur: async (call) => {
                    // read the people the issue affected before
                    const { input, target } = call;
                    const rows = await call.read(issuePerson).findMany({
                        where: {
                            parentId: target.id,
                            person: { in: input.people.map(({ person }) => person) },
                        },
                    });
                    const known = new Map(rows.map((row) => [row.person, row]));
                    const added = input.people.filter(({ person }) => !known.has(person));

                    // count the occurrences, taking their revision only when they are the latest
                    const isLatest = input.lastSeenAt >= target.lastSeenAt;
                    const revision =
                        input.lastRevisionId === undefined || !isLatest
                            ? {}
                            : { lastRevisionId: input.lastRevisionId };
                    const row = await call.update({
                        level: Issue.severest(target.level, input.level),
                        lastSeenAt: Math.max(target.lastSeenAt, input.lastSeenAt),
                        ...revision,
                        count: target.count + input.count,
                        people: target.people + added.length,
                    });

                    // record the new people and the later times of known ones
                    const later = input.people.flatMap(({ person, lastSeenAt }) => {
                        const found = known.get(person);

                        return found === undefined || found.lastSeenAt >= lastSeenAt
                            ? []
                            : [
                                  issuePerson.call("update", {
                                      ...issuePerson.target(found),
                                      lastSeenAt,
                                  }),
                              ];
                    });
                    const created = added.map(({ person, lastSeenAt }) =>
                        issuePerson.call("create", {
                            ...issuePerson.in(target.scope),
                            parentId: target.id,
                            person,
                            lastSeenAt,
                        }),
                    );
                    if (created.length + later.length > 0) {
                        await call.mutate(call.caller, [...created, ...later]);
                    }

                    return row;
                },
                resolve: async (call, next) => {
                    // resolve until the condition the caller gives, then announce it
                    await Issue.requireUntil(call, Until.parse(call.input["until"]));
                    const row = await next();
                    await call.mutate(call.caller, [resolvedIssue.call(Issue.notice(row))]);

                    return row;
                },
                ignore: async (call, next) => {
                    // ignore until the condition the caller gives
                    await Issue.requireUntil(call, Until.parse(call.input["until"]));

                    return next();
                },
                reopen: (call) => call.update({ status: "unresolved", until: { kind: "event" } }),
                regress: async (call) => {
                    // reopen the issue until its next event, and announce the regression
                    const row = await call.update({
                        status: "regressed",
                        reopenedAt: call.now,
                        until: { kind: "event" },
                    });
                    await call.mutate(call.self, [regressedIssue.call(Issue.notice(row))]);

                    return row;
                },
                escalate: (call) =>
                    call.update({
                        status: "unresolved",
                        reopenedAt: call.now,
                        until: { kind: "event" },
                    }),
            },
        }),
        alertRule.implement({
            // let stacks declare rules as code, applied through create and update, which check each rule
            declaration: {},
            controller: {
                pending: {},
                watches: [
                    // evaluate the space's issue rules again once one of its issues changes
                    ObjectWatch.of(issue.table, async (row, database) => {
                        const rules = await Snapshot.live(database)
                            .read(alertRule.table)
                            .findMany({ where: { scope: row.scope } });

                        return rules
                            .filter((rule) => rule.condition.kind === "issue")
                            .map((rule) => ({ id: rule.id }));
                    }),
                ],
                reconcile: async (reconciliation) => {
                    // evaluate the rule, again every minute
                    for (const row of reconciliation.rows) {
                        await new AlertRule(row).evaluate(reconciliation);
                    }

                    return EVALUATION_MILLISECONDS;
                },
            },
            methods: {
                create: async (call, next) => {
                    // refuse an unreadable rule before writing it
                    const { condition, actions, pending = NO_PENDING } = call.input;
                    const scope = present(call.scope, "the scope of a new alert rule");
                    AlertRule.require({ scope, condition, actions, pending }, call.objects);

                    // refuse a rule calling installations from a writer who does not manage alert rules
                    const row = await next();
                    await AlertRule.requireCalls(row, call);
                    const host = alertRule.reference(row.scope, row.id);

                    // follow the rule as its author
                    await call.mutate(call.caller, [
                        Subscription.subscribe(host, call.caller.subject, "author"),
                    ]);

                    return row;
                },
                update: async (call, next) => {
                    // refuse an unreadable rule before writing it, as create does
                    const { target, input } = call;
                    const proposed = {
                        scope: target.scope,
                        id: target.id,
                        condition: input.condition ?? target.condition,
                        actions: input.actions ?? target.actions,
                        pending: input.pending ?? target.pending,
                    };
                    AlertRule.require(proposed, call.objects);

                    // make the editor of what the rule does or when the author, whom the calls act as
                    const isAuthoring = AUTHORED_FIELDS.some((name) => input[name] !== undefined);
                    if (isAuthoring) {
                        await AlertRule.requireCalls(proposed, call);
                    }
                    const author = ObjectReference.columns("author", call.caller.subject);
                    const row = await next(
                        isAuthoring ? call.with({ input: { ...input, ...author } }) : call,
                    );
                    if (!isAuthoring) {
                        return row;
                    }

                    // follow the rule as its new author
                    const host = alertRule.reference(row.scope, row.id);
                    await call.mutate(call.caller, [
                        Subscription.subscribe(host, call.caller.subject, "author"),
                    ]);

                    return row;
                },
            },
        }),
        alert.implement({
            methods: {
                fire: async (call) => {
                    // fire the alert at its time, then announce it unless a silence holds it back
                    const row = await call.update({ status: "firing", firedAt: call.now });
                    const [announced] = await call.mutate(call.self, [
                        alert.call("announce", alert.target(row)),
                    ]);

                    return announced;
                },
                announce: async (call) => {
                    // leave an alert no longer firing, one announced, and one a silence covers
                    const { target } = call;
                    const rule = await AlertRule.read(call, target);
                    const silences = await rule.silences(call);
                    const isHeld =
                        target.status !== "firing" ||
                        target.announcedAt !== null ||
                        silences.some((each) => each.covers(target, call.now));
                    if (isHeld) {
                        return target;
                    }

                    // announce it, start its response, and record when
                    await rule.announce(call, target, "fired", silences);
                    await rule.respond(call, target, silences);

                    return call.update({ announcedAt: call.now });
                },
                resolve: async (call) => {
                    // resolve the alert at its time, by a person or once its condition cleared, then announce it
                    const row = await call.update({ status: "resolved", resolvedAt: call.now });
                    const rule = await AlertRule.read(call, row);
                    await rule.announce(call, row, "resolved", await rule.silences(call));

                    return row;
                },
            },
        }),
        silence,
        ...responses.objects,
    ],
    // run the responses to fired alerts
    controllers: [
        ...responses.controllers,
        // count each space's exceptions into its issues a minute at a time, moving the reader in the same commit
        exception.consume("issues", async (page, { store, transaction }) => {
            const { key: scope, after, through } = page;
            await store.mutate(
                store.as(issue),
                {
                    id: RequestId.create(store.environment),
                    calls: [issue.call("count", { ...issue.in(scope), after, through })],
                },
                { transaction, scope },
            );
        }),
    ],
    // take OTLP exports, grouping a space's exceptions into its issues
    adapters: {
        otlp: (request, _caller, context, store) => new OtlpIntake(store).fetch(request, context),
    },
});
