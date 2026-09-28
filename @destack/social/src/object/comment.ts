import {
    ACCESS_PACKAGE_ID,
    intersection,
    permission,
    relation,
    Subject,
    through,
    union,
} from "@destack/access";
import { index, type Select } from "@destack/db";
import { type Action, announcement, notification, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { Call, defineObject, field, method, type ObjectType, Selection } from "@destack/object";
import { identifier, schema } from "@destack/schema";
import { space } from "@destack/space/object";
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
    /** The mentioned user, host or installation. */
    principal: Subject.omit({ relation: true }).extend({
        packageId: schema.literal(ACCESS_PACKAGE_ID),
        type: schema.enum(["user", "host", "installation"]),
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
export const comment = defineObject({
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
            .reference<"comment">((): ObjectType => comment, { delete: "cascade" })
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
    // find a thread's replies
    constraints: (comment) => [index("comment_thread").on(comment.thread)],
    permissions: {
        read: through("parent", "read"),
        comment: through("parent", "comment"),
        edit: intersection(relation("author"), through("parent", "comment")),
        resolve: union(permission("edit"), through("parent", "edit")),
        delete: union(permission("edit"), through("parent", "manage")),
    },
    aggregates: { commentCount: { function: "count" } },
    attachments: [
        reaction.attach({ by: "comment" }),
        // hold a thread's subscriptions and notifications
        subscription.attach({ by: "read" }),
        notification.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
    ],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("comment", { fields: ["body", "selection", "thread"] }),
        update: method.update("edit", { fields: ["body"] }),
        delete: method.delete("delete"),
        resolve: method({ permission: "resolve" }),
        reopen: method({ permission: "resolve" }),
    },
});

/** A comment as its table holds it. */
export type CommentRow = Select<typeof comment.table>;

/** A comment as its notifications carry it. */
export const Excerpt = schema.object({
    /** The author. */
    author: Subject,
    /** The comment. */
    comment: identifier("comment"),
    /** The thread's first comment. */
    thread: identifier("comment"),
    /** The start of the text. */
    text: schema.string().max(EXCERPT_LENGTH),
});
/** A comment as its notifications carry it. */
export type Excerpt = schema.Infer<typeof Excerpt>;

/** Reply in the notified comment's thread. */
const answer: Action<Excerpt> = {
    title: "Reply",
    text: { placeholder: "Reply", button: "Send" },
    call: ({ source, payload }, text) =>
        Call.record(comment, "create", {
            [comment.route.field!]: source.scope,
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            body: { text: text!, mentions: [] },
            thread: payload.thread,
        }),
};

/** A comment mentions the recipient. */
export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: "New mention", body: payload.text }),
    summary: (count) => (count === 1 ? "1 mention" : `${count} mentions`),
    actions: { reply: answer },
});

/** A comment starts a thread on a subscribed object. */
export const thread = defineNotification({
    name: "thread",
    title: "Threads",
    description: "Someone starts a comment thread on something you subscribe to.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: "New thread", body: payload.text }),
    summary: (count) => (count === 1 ? "1 thread" : `${count} threads`),
    actions: { reply: answer },
});

/** A comment replies in a subscribed thread. */
export const reply = defineNotification({
    name: "reply",
    title: "Replies",
    description: "Someone replies in a comment thread you subscribe to.",
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: "New reply", body: payload.text }),
    summary: (count) => (count === 1 ? "1 reply" : `${count} replies`),
    actions: { reply: answer },
});
