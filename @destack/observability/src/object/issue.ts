import { space } from "@destack/account/object";
import { index, unique, type Select } from "@destack/db";
import { plural, t } from "@destack/locale";
import { activity, announcement, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field, type ObjectType } from "@destack/object";
import { graph } from "@destack/package";
import { defineSchema, Digest, Instant, schema } from "@destack/schema";
import { comment } from "@destack/social";
import { installation, installationRevision } from "@destack/space/object";

/** The levels of an issue, from its most severe exception. */
export const ISSUE_LEVELS = ["fatal", "error", "warning", "info"] as const;

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
    title: schema.string().max(500),
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
    content: (payload) => ({ title: t`Regressed`, body: payload.title }),
    summary: (count) => t`${plural(count, { one: "# issue", other: "# issues" })} regressed`,
});

/** Someone resolves an issue. */
export const resolvedIssue = defineNotification({
    name: "resolvedIssue",
    title: "Resolutions",
    description: "Someone resolves an issue you follow.",
    payload: IssueExcerpt,
    interruption: "passive",
    preference: { channels: ["desktop"], delivery: "immediate" },
    content: (payload) => ({ title: t`Resolved`, body: payload.title }),
    summary: (count) => t`${plural(count, { one: "# issue", other: "# issues" })} resolved`,
});

/** What reopens a resolved or ignored issue. */
export const Until = defineSchema(
    schema.discriminatedUnion("kind", [
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
    ]),
);
/** What reopens a resolved or ignored issue. */
export type Until = schema.Infer<typeof Until>;

/** Exceptions of one installation grouped by their fingerprint. */
export const issue = defineObject({
    name: "issue",
    plural: "issues",
    scope: space,
    fields: {
        /** The installation whose exceptions it groups. */
        installation: field.reference("installation", (): ObjectType => installation, {
            delete: "cascade",
        }),
        /** The digest of the error type and in-app symbols grouping its exceptions. */
        fingerprint: field.string(Digest),
        /** The error type: a service error code, an error's own code or its name. */
        errorType: field.string(),
        /** The error type and the first line of the first exception's message. */
        title: field.string(schema.string().min(1).max(500)),
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
                    permission: "edit",
                },
                ignore: { from: ["unresolved", "regressed"], to: "ignored", permission: "edit" },
                reopen: { from: ["resolved", "ignored"], to: "unresolved", permission: "edit" },
                regress: { from: ["resolved"], to: "regressed", permission: "detect" },
                escalate: { from: ["ignored"], to: "unresolved", permission: "detect" },
            },
        }),
        /** What reopens the issue once resolved or ignored. */
        until: field.json(Until).default({ kind: "event" }),
        /** When the issue last regressed or escalated, in Unix milliseconds. */
        reopenedAt: field.time().optional(),

        // occurrence
        /** When the first exception happened, in Unix milliseconds. */
        firstSeenAt: field.time(),
        /** When the latest exception happened, in Unix milliseconds. */
        lastSeenAt: field.time(),
        /** The revision the first exception ran. */
        firstRevision: field
            .reference("installation-revision", (): ObjectType => installationRevision)
            .optional(),
        /** The revision the latest exception ran. */
        lastRevision: field
            .reference("installation-revision", (): ObjectType => installationRevision)
            .optional(),
        /** The exceptions grouped. */
        count: field.integer().default(0),
        /** The people affected. */
        people: field.integer().default(0),
        /** The comments on the issue. */
        commentCount: field.count(),
    },
    constraints: (entry) => [
        unique("issue_fingerprint").on(entry.scope, entry.installationId, entry.fingerprint),
        index("issue_last_seen").on(entry.scope, entry.lastSeenAt),
        index("issue_declaration").on(entry.scope, entry.declaration),
    ],
    permissions: ["read", "edit", "manage", "detect"],
    attachments: [
        comment.attach({ by: "read" }),
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Set what reopens the issue once resolved or ignored. */
        update: method.update("edit", { fields: ["until"] }),
        /** Open an issue for its first exceptions. */
        open: method.create("detect", {
            isInternal: true,
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
            ],
        }),
        /** Count further exceptions into an issue. */
        occur: method.update("detect", {
            isInternal: true,
            fields: ["level", "lastSeenAt", "lastRevisionId", "count", "people"],
        }),
    }),
});
/** An issue as its table stores it. */
export type Issue = Select<typeof issue.table>;
