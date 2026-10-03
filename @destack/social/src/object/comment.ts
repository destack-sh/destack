import {
    ACCESS_PACKAGE_ID,
    intersection,
    permission,
    relation,
    through,
    union,
} from "@destack/access";
import { Subject } from "@destack/sync";
import { index, type Select } from "@destack/db";
import { type Action, activity, announcement, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field, type ObjectType, Selection } from "@destack/object";
import { schema } from "@destack/schema";
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
            .reference("comment", (): ObjectType => comment, { delete: "cascade" })
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
    constraints: (entry) => [index("comment_thread").on(entry.threadId)],
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
        // keep a thread's subscriptions and activities
        subscription.attach({ by: "read" }),
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
    ],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("comment", { fields: ["body", "selection", "threadId"] }),
        update: method.update("edit", { fields: ["body"] }),
        delete: method.delete("delete"),
        resolve: method.mutation({ permission: "resolve" }),
        reopen: method.mutation({ permission: "resolve" }),
    }),
});

/** A comment as its table stores it. */
export type Comment = Select<typeof comment.table>;

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
const answer: Action<Excerpt> = {
    title: "Reply",
    text: { placeholder: "Reply", button: "Send" },
    effect: async ({ source, payload }, call, text) => {
        // require the reply's text
        if (text === undefined) {
            throw new TypeError("reply action needs its text");
        }

        // find the object the thread comments on, through its first comment when notified there
        const first =
            source.type === comment.name
                ? await call.invoke(comment).get({ id: comment.identifier(source.id) })
                : undefined;
        const parent =
            first === undefined
                ? { packageId: source.packageId, type: source.type, id: source.id }
                : { packageId: first.parentPackageId, type: first.parentType, id: first.parentId };

        return call.invoke(comment).create({
            parent,
            body: { text, mentions: [] },
            threadId: payload.thread,
        });
    },
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
