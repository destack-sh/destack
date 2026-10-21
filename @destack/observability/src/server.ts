import { subscription } from "@destack/notification";
import { ObjectWatch } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { ObjectReference } from "@destack/sync";
import {
    alert,
    alertRule,
    AlertRule,
    EVALUATION_MILLISECONDS,
    type Issue,
    issue,
    regressedIssue,
    resolvedIssue,
    respond,
    silence,
} from "./object/index.ts";
import { OtlpIntake } from "./otlp/intake.ts";
import observabilityPackage from "./package.ts";

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
                resolve: async (call, next) => {
                    // announce the resolution
                    const row = await next();
                    await call.mutate(call.caller, [resolvedIssue.call(notice(row))]);

                    return row;
                },
                regress: async (call) => {
                    // reopen the issue until its next event, and announce the regression
                    const row = await call.update({
                        status: "regressed",
                        reopenedAt: call.now,
                        until: { kind: "event" },
                    });
                    await call.mutate(call.caller, [regressedIssue.call(notice(row))]);

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
            controller: {
                pending: {},
                key: (row) => ({ scope: row.scope }),
                watches: [ObjectWatch.of(issue.table, (row) => [{ scope: row.scope }])],
                reconcile: async (reconciliation) => {
                    // evaluate each of the space's rules, again every minute
                    for (const row of reconciliation.rows) {
                        await new AlertRule(row).evaluate(reconciliation);
                    }

                    return EVALUATION_MILLISECONDS;
                },
            },
            methods: {
                create: async (call, next) => {
                    // refuse an unreadable rule, and one calling installations its author manages none of
                    const row = await next();
                    const rule = new AlertRule(row);
                    rule.require(call.objects);
                    await rule.requireCalls(call);
                    const host = alertRule.reference(row.scope, row.id);

                    // follow the rule as its author
                    await call.mutate(call.caller, [
                        subscription.call("add", {
                            parent: { packageId: host.packageId, type: host.type, id: host.id },
                            ...ObjectReference.columns("owner", call.caller.subject),
                            reason: "author",
                        }),
                    ]);

                    return row;
                },
                update: async (call, next) => {
                    // refuse an unreadable rule, and one calling installations its editor manages none of
                    const rule = new AlertRule(await next());
                    rule.require(call.objects);
                    await rule.requireCalls(call);

                    return rule.row;
                },
                apply: async (call, next) => {
                    // refuse a declared rule the space cannot evaluate, a retired one aside
                    const applied = await next();
                    if ("condition" in applied) {
                        new AlertRule(applied).require(call.objects);
                    }

                    return applied;
                },
            },
        }),
        alert.implement({
            methods: {
                fire: async (call, next) => {
                    // announce the fired alert and start its response
                    const row = await next();
                    const rule = await AlertRule.read(call, row);
                    await rule.announce(call, row, "fired");
                    await rule.respond(call, row);

                    return row;
                },
                settle: async (call, next) => {
                    // announce the alert's resolution once its condition cleared
                    const row = await next();
                    await (await AlertRule.read(call, row)).announce(call, row, "resolved");

                    return row;
                },
                resolve: async (call, next) => {
                    // announce the alert's resolution by a person
                    const row = await next();
                    await (await AlertRule.read(call, row)).announce(call, row, "resolved");

                    return row;
                },
            },
        }),
        silence,
        ...responses.objects,
    ],
    // take OTLP exports, grouping a space's exceptions into its issues
    controllers: responses.controllers,
    adapters: {
        otlp: (request, _caller, context, store) => new OtlpIntake(store).fetch(request, context),
    },
});

/** Build the announcement of an issue to its subscribers. */
function notice(row: Issue) {
    return {
        source: issue.reference(row.scope, row.id),
        audience: { kind: "subscribers" },
        payload: { issue: row.id, title: row.title },
    } as const;
}
