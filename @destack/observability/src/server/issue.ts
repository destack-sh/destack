import type { EventStore } from "@destack/event";
import { subscription } from "@destack/notification";
import { serveActivities } from "@destack/notification/server";
import * as social from "@destack/social";
import { reaction } from "@destack/social";
import * as socialServer from "@destack/social/server";
import { ObjectWatch } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { Subject } from "@destack/sync";
import * as base from "../object/index.ts";
import { AlertRuleCheck, evaluateRules } from "./alert.ts";

/** The activities and announcements of issues, alert rules and their comments. */
const activities = serveActivities({
    notifications: [
        base.regressedIssue,
        base.resolvedIssue,
        base.firingAlert,
        social.mention,
        social.thread,
        social.reply,
    ],
});

/** What serving issues and alert rules needs: the space's events. */
export interface IssueOptions {
    /** The event store events conditions fold. */
    readonly events: EventStore;
}

/** Serve issues, which announce resolutions and regressions. */
export function serveIssues(options: IssueOptions) {
    const alertRule = alertRuleOf(options);

    return [
        issue,
        alertRule.implement({
            controller: {
                pending: {},
                key: (row) => ({ scope: row.scope }),
                watches: [ObjectWatch.of(base.issue.table, (row) => [{ scope: row.scope }])],
                reconcile: (reconciliation) => evaluateRules(reconciliation, options),
            },
        }),
        alert,
        socialServer.comment,
        reaction,
        ...activities,
    ];
}

/** Issues announcing their resolutions and regressions, clearing their condition once reopened. */
const issue = base.issue.implement({
    methods: {
        resolve: async (call, next) => {
            const row = await next();
            await call.mutate(call.caller, [base.resolvedIssue.call(noticeOf(row))]);

            return row;
        },
        regress: async (call) => {
            const row = await call.update({
                status: "regressed",
                reopenedAt: call.now,
                until: { kind: "event" },
            });
            await call.mutate(call.caller, [base.regressedIssue.call(noticeOf(row))]);

            return row;
        },
        escalate: (call) =>
            call.update({ status: "unresolved", reopenedAt: call.now, until: { kind: "event" } }),
    },
});

/** Alert rules that refuse conditions they cannot evaluate or their author may not read. */
function alertRuleOf(options: IssueOptions) {
    return base.alertRule.implement({
        methods: {
            create: async (call, next) => {
                // refuse an unreadable rule, then follow the rule as its author
                const row = requireRule(await next(), options);
                const host = base.alertRule.reference(row.scope, row.id);
                await call.mutate(call.caller, [
                    subscription.call("add", {
                        parent: { packageId: host.packageId, type: host.type, id: host.id },
                        owner: Subject.key(call.requireCaller()),
                        reason: "author",
                    }),
                ]);

                return row;
            },
            update: async (_call, next) => requireRule(await next(), options),
            apply: async (_call, next) => {
                // refuse a declared rule the space cannot evaluate, a retired one aside
                const applied = await next();

                return "condition" in applied ? requireRule(applied, options) : applied;
            },
        },
    });
}

/** Alerts announced to the subscribers of their rule. */
const alert = base.alert.implement({
    methods: {
        fire: async (call, next) => {
            // announce the alert on its rule when the rule notifies
            const row = await next();
            const rule =
                row.ruleId === null
                    ? null
                    : await call.read(base.alertRule).row({ id: row.ruleId });
            if (rule?.actions.some((action) => action.kind === "notify") === true) {
                await call.mutate(call.caller, [
                    base.firingAlert.call({
                        source: base.alertRule.reference(rule.scope, rule.id),
                        audience: { kind: "subscribers" },
                        payload: { alert: row.id, rule: rule.title, title: row.title },
                    }),
                ]);
            }

            return row;
        },
    },
});

/** The announcement of an issue to its subscribers. */
function noticeOf(row: base.Issue) {
    return {
        source: base.issue.reference(row.scope, row.id),
        audience: { kind: "subscribers" },
        payload: { issue: row.id, title: row.title },
    } as const;
}

/** Refuse a written rule whose condition the space cannot evaluate, and roll its write back. */
function requireRule(row: base.AlertRule, options: IssueOptions): base.AlertRule {
    try {
        AlertRuleCheck.require(row, options.events);
    } catch (error) {
        throw new ServiceError("BAD_REQUEST", {
            message: error instanceof Error ? error.message : String(error),
        });
    }

    return row;
}
