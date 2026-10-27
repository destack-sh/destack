import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { activity, announcement, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { none, through } from "@destack/access";
import {
    type MethodContext,
    type ObjectOf,
    defineObject,
    field,
    installation,
} from "@destack/object";
import { ObservabilityError } from "../error/error.ts";
import { graph } from "@destack/package";
import { Digest, Instant, schema, type Uuid } from "@destack/schema";
import { comment } from "@destack/social";
import { installationRevision } from "@destack/stack/object";

/** The longest issue title: 500 characters, an exception's type and its message's first line. */
export const ISSUE_TITLE_LENGTH = 500;

/** The namespace of issue identifiers, derived from their space, installation and fingerprint. */
export const ISSUE_NAMESPACE: Uuid = "6f1d2c8a-3b7e-4a59-9c4d-2e8b5a7f1c30";

/** A person exceptions affected, with when they last saw one. */
export const AffectedPerson = schema.object({
    /** The person. */
    person: schema.string().min(1),
    /** When they last saw one, in Unix milliseconds. */
    lastSeenAt: schema.number().int().nonnegative(),
});

/** An exception an issue records: the person it happened to, and when. */
export const RecordedOccurrence = schema.object({
    /** The person it happened to, absent when unknown. */
    person: schema.string().min(1).exactOptional(),
    /** When it happened, in Unix milliseconds. */
    at: schema.number().int().nonnegative(),
});
/** An exception an issue records. */
export type RecordedOccurrence = schema.Infer<typeof RecordedOccurrence>;

/** The levels of an issue, from its most severe exception. */
export const ISSUE_LEVELS = ["fatal", "error", "warning", "info"] as const;

/** The fields opening an issue at its first exceptions. */
export const IssueOpening = schema.object({
    /** The installation that ran them. */
    installationId: schema.identifier("installation"),
    /** The digest grouping them. */
    fingerprint: Digest,
    /** The error type. */
    errorType: schema.string(),
    /** The error type and the first line of the first exception's message. */
    title: schema.string().min(1).max(ISSUE_TITLE_LENGTH),
    /** The innermost in-app symbol the first exception ran, absent without one. */
    culprit: graph.Moniker.exactOptional(),
    /** The declaration the culprit is or belongs to, absent without one. */
    declaration: graph.Moniker.exactOptional(),
    /** The most severe level of the exceptions. */
    level: schema.enum(ISSUE_LEVELS),
    /** When the first exception happened, in Unix milliseconds. */
    firstSeenAt: schema.number().int().nonnegative(),
    /** When the latest exception happened, in Unix milliseconds. */
    lastSeenAt: schema.number().int().nonnegative(),
    /** The revision the first exception ran, absent when unknown. */
    firstRevisionId: schema.identifier("installation-revision").exactOptional(),
    /** The revision the latest exception ran, absent when unknown. */
    lastRevisionId: schema.identifier("installation-revision").exactOptional(),
});
/** The fields opening an issue at its first exceptions. */
export type IssueOpening = schema.Infer<typeof IssueOpening>;

/** The issue fields an alert rule's filter may name. */
export const ISSUE_FILTER_FIELDS: ReadonlySet<string> = new Set([
    "installationId",
    "errorType",
    "title",
    "culprit",
    "declaration",
    "level",
    "status",
    "count",
    "people",
]);

/** An installation revision's identifier. */
const RevisionId = schema.identifier("installation-revision");

/** An issue as its notifications carry it. */
export const IssueExcerpt = schema.object({
    /** The issue. */
    issue: schema.identifier("issue"),
    /** The issue's title. */
    title: schema.string().max(ISSUE_TITLE_LENGTH),
});
/** An issue as its notifications carry it. */
export type IssueExcerpt = schema.Infer<typeof IssueExcerpt>;

/** A resolved issue fails again. */
export const regressedIssue = defineNotification({
    name: "regressedIssue",
    title: "Regressions",
    description: "An issue you follow fails again after its resolution.",
    payload: IssueExcerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`Regressed`, body: t`${argument("title")}` },
    summary: t`${plural(argument("count"), { one: "# issue", other: "# issues" })} regressed`,
});

/** Someone resolves an issue. */
export const resolvedIssue = defineNotification({
    name: "resolvedIssue",
    title: "Resolutions",
    description: "Someone resolves an issue you follow.",
    payload: IssueExcerpt,
    interruption: "passive",
    preference: { channels: ["desktop"], delivery: "immediate" },
    content: { title: t`Resolved`, body: t`${argument("title")}` },
    summary: t`${plural(argument("count"), { one: "# issue", other: "# issues" })} resolved`,
});

/** What reopens a resolved or ignored issue. */
export const Until = schema.discriminatedUnion("kind", [
    schema.object({
        /** Any further exception. */
        kind: schema.literal("event"),
    }),
    schema.object({
        /** An exception of a revision of the installation after the given one. */
        kind: schema.literal("revision"),
        after: RevisionId,
    }),
    schema.object({
        /** Any exception after a time. */
        kind: schema.literal("time"),
        at: Instant,
    }),
    schema.object({
        /** The issue's exceptions reaching a total count. */
        kind: schema.literal("count"),
        count: schema.number().int().min(1),
    }),
    schema.object({
        /** Nothing. */
        kind: schema.literal("never"),
    }),
]);
/** What reopens a resolved or ignored issue. */
export type Until = schema.Infer<typeof Until>;

/** Exceptions of one installation grouped by their fingerprint. */
export const issue = defineObject(
    {
        name: "issue",
        plural: "issues",
        scope: space,
        fields: {
            /** The installation whose exceptions it groups. */
            installation: field.reference(
                (): ObjectOf<{ identity: "installation" }> => installation,
                {
                    delete: "cascade",
                },
            ),
            /** The digest of the error type and in-app symbols grouping its exceptions. */
            fingerprint: field.string(Digest),
            /** The error type: a call error code, an error's own code or its name. */
            errorType: field.string(),
            /** The error type and the first line of the first exception's message. */
            title: field.string(schema.string().min(1).max(ISSUE_TITLE_LENGTH)),
            /** The innermost in-app symbol the first exception ran. */
            culprit: field.string(graph.Moniker).optional(),
            /** The declaration the culprit is or belongs to. */
            declaration: field.string(graph.Moniker).optional(),
            /** The most severe level of its exceptions. */
            level: field.enum(ISSUE_LEVELS),

            // lifecycle
            /** Where the issue stands: open, resolved, failing again or ignored. */
            status: field.state({
                initial: "unresolved",
                transitions: {
                    resolve: {
                        from: ["unresolved", "regressed", "ignored"],
                        to: "resolved",
                        permission: "write",
                        fields: ["until"],
                    },
                    ignore: {
                        from: ["unresolved", "regressed"],
                        to: "ignored",
                        permission: "write",
                        fields: ["until"],
                    },
                    reopen: {
                        from: ["resolved", "ignored"],
                        to: "unresolved",
                        permission: "write",
                    },
                    regress: { from: ["resolved"], to: "regressed", permission: "detect" },
                    escalate: { from: ["ignored"], to: "unresolved", permission: "detect" },
                },
            }),
            /** What reopens the issue once resolved or ignored. */
            until: field.json(Until),
            /** When the issue last regressed or escalated, in Unix milliseconds. */
            reopenedAt: field.time().optional(),

            // occurrence
            /** When the first exception happened, in Unix milliseconds. */
            firstSeenAt: field.time(),
            /** When the latest exception happened, in Unix milliseconds. */
            lastSeenAt: field.time(),
            /** The revision the first exception ran. */
            firstRevision: field.reference(installationRevision).optional(),
            /** The revision the latest exception ran. */
            lastRevision: field.reference(installationRevision).optional(),
            /** The exceptions grouped. */
            count: field.integer().default(0),
            /** The people affected. */
            people: field.integer().default(0),
            /** The person or team working on it, as Sentry's assignee. */
            assignee: field.subject().optional(),
            /** The comments on the issue. */
            commentCount: field.count(),
        },
        permissions: {
            read: none(),
            write: none(),
            manage: none(),
            // let the machine serving the space record what it forwards, in the transaction appending it
            detect: through("scope", "serve"),
        },
        attachments: [
            comment.attach({ by: "read" }),
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
            subscription.attach({ by: "read" }),
        ],
        constraints: {
            fingerprint: { kind: "unique", on: ["scope", "installationId", "fingerprint"] },
            lastSeen: { kind: "index", on: ["scope", "lastSeenAt"] },
            declaration: { kind: "index", on: ["scope", "declaration"] },
        },
    },
    (method) => ({
        create: false,
        delete: false,
        /** Open an issue for its first exceptions. */
        open: method.create("detect", {
            visibility: "package",
            fields: [
                "installationId",
                "fingerprint",
                "errorType",
                "title",
                "culprit",
                "declaration",
                "level",
                "firstSeenAt",
                "lastSeenAt",
                "firstRevisionId",
                "lastRevisionId",
                "until",
            ],
        }),
        /** Count further exceptions and the people they affected into an issue. */
        occur: method.mutation({
            permission: "detect",
            visibility: "package",
            input: schema.object({
                /** The severest level of the exceptions. */
                level: schema.enum(ISSUE_LEVELS),
                /** When the latest of them happened. */
                lastSeenAt: schema.number().int().nonnegative(),
                /** The revision the latest of them ran, absent when unknown. */
                lastRevisionId: schema.identifier("installation-revision").exactOptional(),
                /** How many exceptions to add. */
                count: schema.int().min(1),
                /** The people the exceptions affected, with when each last saw one. */
                people: schema.array(AffectedPerson),
            }),
        }),
        /** Record exceptions into their issue, opening or reopening it, as its reader does. */
        record: method.mutation({
            permission: "detect",
            visibility: "package",
            target: "scope",
            output: schema.object({}),
            input: schema.object({
                /** The issue, its identifier derived from its installation and fingerprint. */
                issueId: schema.identifier("issue"),
                /** The fields opening the issue at its first exceptions. */
                opening: IssueOpening,
                /** The exceptions, each appended once by its identity. */
                occurrences: schema.array(RecordedOccurrence).min(1),
                /** When the latest revision the exceptions ran was made, absent when unknown. */
                revisedAt: schema.number().int().nonnegative().exactOptional(),
            }),
        }),
        /** Count a claimed page of a space's exceptions into their issues, as their reader does. */
        count: method.mutation({
            permission: "detect",
            visibility: "package",
            target: "scope",
            output: schema.object({}),
            input: schema.object({
                /** The log sequence the space's exceptions stand counted up to. */
                after: schema.number().int().nonnegative(),
                /** The log sequence the claimed page of exceptions ends at. */
                through: schema.number().int().nonnegative(),
            }),
        }),
        /** Set whom the issue is assigned to. */
        update: method.update("write", {
            fields: ["assigneePackageId", "assigneeType", "assigneeScope", "assigneeId"],
        }),
    }),
);
/** An issue as its table stores it. */
export type Issue = Select<typeof issue.table>;

/** The issues of a space's installations. */
export const Issue = {
    /** Read the more severe of two levels. */
    severest(left: Issue["level"], right: Issue["level"]): Issue["level"] {
        return ISSUE_LEVELS.indexOf(left) <= ISSUE_LEVELS.indexOf(right) ? left : right;
    },

    /** Build the announcement of an issue to its subscribers. */
    notice(row: Issue) {
        return {
            source: issue.reference(row.scope, row.id),
            audience: { kind: "subscribers" },
            payload: { issue: row.id, title: row.title },
        } as const;
    },

    /** Refuse a reopening condition already met, or naming a revision of another installation. */
    async requireUntil(
        call: {
            readonly target: Issue;
            readonly now: number;
            readonly read: MethodContext["read"];
        },
        until: Until,
    ): Promise<void> {
        // refuse a count the issue reached or a time that passed, which would reopen it at its next event
        const { target } = call;
        if (until.kind === "count" && until.count <= target.count) {
            throw ObservabilityError.of(
                "UNTIL_PASSED",
                `the issue counted ${target.count} events, at or past ${until.count}`,
            );
        } else if (until.kind === "time" && until.at <= call.now) {
            throw ObservabilityError.of("UNTIL_PASSED", `the time ${until.at} has passed`);
        }

        // refuse a revision of another installation or none
        if (until.kind === "revision") {
            const found = await call.read(installationRevision).findFirst({
                where: { id: until.after, installationId: target.installationId },
            });
            if (found === undefined) {
                throw ObservabilityError.of(
                    "REVISION_UNKNOWN",
                    `revision ${until.after} is not a revision of installation ${target.installationId}`,
                );
            }
        }
    },

    /** Report whether recorded exceptions reopen a resolved or ignored issue, by its condition. */
    reopens(
        row: Pick<Issue, "status" | "until" | "count">,
        recorded: {
            /** When the latest exception happened, in Unix milliseconds. */
            readonly lastSeenAt: number;
            /** When the latest revision the exceptions ran was made, absent when unknown. */
            readonly revisedAt: number | undefined;
            /** When the revision the condition names was made, absent for another condition. */
            readonly afterAt: number | undefined;
        },
    ): boolean {
        // reopen only resolved and ignored issues
        const { until } = row;
        if (row.status !== "resolved" && row.status !== "ignored") {
            return false;
        }

        // reopen by the condition: any exception, a later revision, a time, or a count
        switch (until.kind) {
            case "event":
                return true;
            case "revision":
                return (
                    recorded.revisedAt !== undefined &&
                    recorded.afterAt !== undefined &&
                    recorded.revisedAt > recorded.afterAt
                );
            case "time":
                return recorded.lastSeenAt >= until.at;
            case "count":
                return row.count >= until.count;
            case "never":
                return false;
        }
    },
};
