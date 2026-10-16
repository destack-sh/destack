import { space } from "@destack/account/object";
import {
    ACCESS_PACKAGE_ID,
    intersection,
    permission,
    relation,
    through,
    union,
} from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { ObjectReference, Subject } from "@destack/sync";
import { CallError } from "@destack/runtime/error";
import { type Select } from "@destack/db";
import { activity, announcement, type Reason, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import {
    MethodContext,
    Containment,
    defineObject,
    field,
    type ObjectOf,
    Selection,
} from "@destack/object";
import { schema } from "@destack/schema";

import { reaction } from "./reaction.ts";

/** The longest comment text in UTF-16 code units, about four pages. */
const TEXT_LENGTH = 10_000;

/** The most principals one comment mentions, as many as one notify call names. */
const MENTION_COUNT = 100;

/** The longest excerpt a notification carries: at most 840 bytes, within a payload's 2048. */
export const EXCERPT_LENGTH = 280;

/** A principal a comment mentions, at a span of its text. */
export const Mention = schema.object({
    /** The span's first UTF-16 code unit. */
    offset: schema.int().min(0),
    /** The span's length in UTF-16 code units. */
    length: schema.int().min(1),
    /** The mentioned user, machine or installation. */
    principal: Subject.omit({ relation: true }).extend({
        packageId: schema.literal(ACCESS_PACKAGE_ID),
        type: schema.enum(["user", "machine", "installation"]),
    }),
});
/** A principal a comment mentions. */
export type Mention = schema.Infer<typeof Mention>;

/** A comment's text and mentions. */
export const Body = schema.object({
    /** The plain text. */
    text: schema.string().min(1).max(TEXT_LENGTH),
    /** The mentions, in order, never overlapping. */
    mentions: schema.array(Mention).max(MENTION_COUNT),
});
/** A comment's text and mentions. */
export type Body = schema.Infer<typeof Body>;

/** A comment that starts or replies to a thread on any object. */
export const comment = defineObject(
    {
        name: "comment",
        plural: "comments",
        scope: space,
        nested: { in: "any", receive: "comment" },
        fields: {
            /** The author. */
            author: field.subject().caller(),
            /** The text and its mentions. */
            body: field.json(Body),
            /** The text the thread comments on, on its first comment only. */
            selection: field.json(Selection).optional(),
            /** The thread's first comment, absent on the first comment itself. */
            thread: field
                .reference((): ObjectOf<{ identity: "comment" }> => comment, { delete: "cascade" })
                .optional(),
            /** When the author last edited the body. */
            editedAt: field.time().optional(),
            /** When the thread was resolved, absent while open. */
            resolvedAt: field.time().optional(),
            /** Who resolved the thread, absent while open. */
            resolvedBy: field.subject().optional(),
            /** The number of reactions. */
            reactionCount: field.count(),
        },
        permissions: {
            read: through("parent", "read"),
            comment: through("parent", "comment"),
            write: intersection(relation("author"), through("parent", "comment")),
            resolve: union(permission("write"), through("parent", "write")),
            delete: union(permission("write"), through("parent", "manage")),
        },
        aggregates: { commentCount: { function: "count" } },
        attachments: [
            reaction.attach({ by: "comment" }),
            // attach a thread's subscriptions and activities
            subscription.attach({ by: "read" }),
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
        ],
        constraints: {
            // find a thread's replies
            thread: { kind: "index", on: ["threadId"] },
        },
    },
    (method) => ({
        create: method.create("comment", { fields: ["body", "selection", "threadId"] }),
        resolve: method.mutation({ permission: "resolve" }),
        reopen: method.mutation({ permission: "resolve" }),
        reply: method.mutation({
            permission: "comment",
            input: schema.object({
                /** The reply's plain text. */
                text: schema.string().min(1).max(TEXT_LENGTH),
            }),
        }),
        update: method.update("write", { fields: ["body"] }),
        delete: method.delete("delete"),
    }),
);

/** A comment as its table stores it. */
export type Comment = Select<typeof comment.table>;

/** A comment's thread, where its notifications go, and its excerpt. */
export const Comment = {
    /** Reference a comment's thread: its first comment. */
    thread(row: Comment): ObjectReference {
        return comment.reference(row.scope, row.threadId ?? row.id);
    },

    /** Reference where a comment's notifications go: its thread for a reply, else its host. */
    source(row: Comment): ObjectReference {
        return row.threadId === null ? ObjectReference.parent(row) : Comment.thread(row);
    },

    /** Build a comment's excerpt without splitting a surrogate pair. */
    excerpt(row: Comment): Excerpt {
        const text = row.body.text.slice(0, EXCERPT_LENGTH);
        const isSplit = /[\uD800-\uDBFF]$/u.test(text);

        return {
            author: ObjectReference.at("author", row),
            comment: row.id,
            thread: row.threadId ?? row.id,
            text: isSplit ? text.slice(0, -1) : text,
        };
    },

    /** Require ordered, disjoint mention spans within a body's text. */
    requireSpans(body: Body): void {
        let end = 0;
        for (const mention of body.mentions) {
            if (mention.offset < end || mention.offset + mention.length > body.text.length) {
                throw new CallError("BAD_REQUEST", {
                    message: "mentions must be ordered, disjoint spans within the text",
                });
            }
            end = mention.offset + mention.length;
        }
    },

    /** Require a comment to be the first of its thread. */
    requireFirst(row: Comment): Comment {
        if (row.threadId !== null) {
            throw new CallError("BAD_REQUEST", {
                message: "a thread resolves through its first comment",
            });
        }

        return row;
    },

    /** Require a reply to answer its thread's first comment on its host, without a selection. */
    async requireThread(
        context: MethodContext,
        thread: Comment["id"],
        selection: Selection | null | undefined,
    ): Promise<void> {
        // read the thread's first comment on the same host
        const host = context.requireParent();
        const [first] = await context.read(comment).findMany({
            where: {
                id: thread,
                parentPackageId: host.packageId,
                parentType: host.type,
                parentId: host.id,
            },
        });

        // refuse a missing thread, a nested reply and a reply's own selection
        if (first === undefined) {
            throw new CallError("NOT_FOUND", { message: "thread not found on the host" });
        } else if (first.threadId !== null) {
            throw new CallError("BAD_REQUEST", {
                message: "a reply answers the first comment of its thread",
            });
        } else if (selection !== null && selection !== undefined) {
            throw new CallError("BAD_REQUEST", { message: "a reply takes its thread's selection" });
        }
    },

    /** Require a selection within a text field of the comment's host. */
    async requireSelection(context: MethodContext, selection: Selection): Promise<void> {
        // find the host's type and the selected elements
        const host = context.requireParent();
        const type = context.objects.find((object) => object.policy.is(host));
        const elements = [selection.anchor.element, selection.head.element];

        // refuse unknown fields
        if (type === undefined || !type.text.includes(selection.field)) {
            throw new CallError("BAD_REQUEST", {
                message: `${host.type} has no text field ${selection.field}`,
            });
        }

        // refuse elements the host's text lacks, as the caller reads it
        const asked = type.call("contains", {
            ...type.target(host),
            field: selection.field,
            elements,
        });
        const { isContained } = Containment.parse(await context.query(context.caller, asked));
        if (!isContained) {
            throw new CallError("BAD_REQUEST", {
                message: `${selection.field} has no such selection`,
            });
        }
    },

    /** List the distinct principals a comment mentions. */
    mentioned(row: Comment): Subject[] {
        const mentioned = row.body.mentions.map((mention) => mention.principal);

        return [
            ...new Map(mentioned.map((principal) => [Subject.key(principal), principal])).values(),
        ];
    },

    /** Build the call subscribing a principal to an object for a reason. */
    subscribe(host: ObjectReference, principal: Subject, reason: Reason) {
        return subscription.call("add", {
            parent: { packageId: host.packageId, type: host.type, id: host.id },
            ...ObjectReference.columns("owner", principal),
            reason,
        });
    },

    /** Build the calls subscribing and notifying the principals a comment mentions. */
    mention(row: Comment, mentioned: readonly Subject[]) {
        const source = Comment.source(row);

        return [
            ...mentioned.map((principal) => Comment.subscribe(source, principal, "mention")),
            mention.call({
                source,
                recipients: mentioned,
                reason: "mention",
                thread: row.parentId,
                payload: Comment.excerpt(row),
            }),
        ];
    },
};

/** A comment as its notifications carry it. */
export const Excerpt = schema.object({
    /** The author. */
    author: Subject,
    /** The comment. */
    comment: schema.identifier("comment"),
    /** The thread's first comment. */
    thread: schema.identifier("comment"),
    /** The start of the text. */
    text: schema.string().max(EXCERPT_LENGTH),
});
/** A comment as its notifications carry it. */
export type Excerpt = schema.Infer<typeof Excerpt>;

/** Reply in the notified comment's thread. */
const answer = {
    name: "reply",
    title: "Reply",
    text: { placeholder: "Reply", button: "Send" },
    type: comment,
    method: "reply",
    target: "thread",
} as const;

/** A comment mentions the recipient. */
export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`New mention`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
    actions: [answer],
});

/** A comment starts a thread on a subscribed object. */
export const thread = defineNotification({
    name: "thread",
    title: "Threads",
    description: "Someone starts a comment thread on something you subscribe to.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`New thread`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# thread", other: "# threads" })}`,
    actions: [answer],
});

/** A comment replies in a subscribed thread. */
export const reply = defineNotification({
    name: "reply",
    title: "Replies",
    description: "Someone replies in a comment thread you subscribe to.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`New reply`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# reply", other: "# replies" })}`,
    actions: [answer],
});
