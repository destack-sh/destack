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
import { type Select } from "@destack/db";
import {
    activity,
    announcement,
    NOTIFY_RECIPIENTS,
    Subscription,
    subscription,
} from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import {
    MethodContext,
    Containment,
    defineObject,
    field,
    type ObjectOf,
    Selection,
} from "@destack/object";
import { present, schema } from "@destack/schema";
import { SocialError } from "../error/index.ts";
import { reaction } from "./reaction.ts";

/** The longest comment text in UTF-16 code units, about four pages. */
const TEXT_LENGTH = 10_000;

/** The longest excerpt a notification carries: at most 840 bytes, within a payload's 2048. */
const EXCERPT_LENGTH = 280;

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
    mentions: schema.array(Mention).max(NOTIFY_RECIPIENTS),
});
/** A comment's text and mentions. */
export type Body = schema.Infer<typeof Body>;

/** A thread's resolution: when and by whom. */
export const Resolution = schema.object({
    /** When the thread was resolved. */
    at: schema.number(),
    /** Who resolved it. */
    by: Subject,
});
/** A thread's resolution. */
export type Resolution = schema.Infer<typeof Resolution>;

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
            // resolve a thread at a time and by someone together
            resolution: {
                kind: "check",
                condition: {
                    OR: [
                        { resolvedAt: { isNull: true }, resolvedById: { isNull: true } },
                        { resolvedAt: { isNotNull: true }, resolvedById: { isNotNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("comment", { fields: ["body", "selection"] }),
        resolve: method.mutation({ permission: "resolve" }),
        reopen: method.mutation({ permission: "resolve" }),
        reply: method.mutation({
            permission: "comment",
            input: Body.extend({ mentions: Body.shape.mentions.default([]) }),
        }),
        update: method.update("write", { fields: ["body"] }),
        delete: method.delete("delete"),
    }),
);

/** A comment: its thread, where its notifications go, its excerpt and its mentions. */
export class Comment {
    /** The comment's row. */
    readonly row: Select<typeof comment.table>;

    /** Read a comment from its row. */
    constructor(row: Comment["row"]) {
        this.row = row;
    }

    /** Reference the comment's thread: its first comment. */
    get thread(): ObjectReference {
        return comment.reference(this.row.scope, this.row.threadId ?? this.row.id);
    }

    /** Read the thread's resolution, absent while open. */
    get resolution(): Resolution | undefined {
        const { resolvedAt } = this.row;

        return resolvedAt === null
            ? undefined
            : { at: resolvedAt, by: ObjectReference.at("resolvedBy", this.row) };
    }

    /** Reference where the comment's notifications go: its thread for a reply, else its host. */
    get source(): ObjectReference {
        return this.row.threadId === null ? ObjectReference.parent(this.row) : this.thread;
    }

    /** Build the comment's excerpt without splitting a surrogate pair. */
    excerpt(): Excerpt {
        const text = this.row.body.text.slice(0, EXCERPT_LENGTH);
        const isSplit = /[\uD800-\uDBFF]$/u.test(text);

        return {
            author: ObjectReference.at("author", this.row),
            comment: this.row.id,
            thread: comment.identifier(this.thread.id),
            text: isSplit ? text.slice(0, -1) : text,
        };
    }

    /** Require the comment to be the first of its thread. */
    requireFirst(): this {
        if (this.row.threadId !== null) {
            throw SocialError.of(
                "FIRST_COMMENT_REQUIRED",
                "only a thread's first comment takes replies and resolves it",
            );
        }

        return this;
    }

    /** List the distinct principals the comment mentions. */
    mentioned(): Subject[] {
        const mentioned = this.row.body.mentions.map((mention) => mention.principal);

        return [
            ...new Map(mentioned.map((principal) => [Subject.key(principal), principal])).values(),
        ];
    }

    /** Build the call notifying the principals the comment mentions, none without any. */
    mention(mentioned: readonly Subject[]) {
        return mentioned.length === 0
            ? []
            : [
                  mention.call({
                      source: this.source,
                      recipients: mentioned,
                      reason: "mention",
                      thread: this.row.parentId,
                      payload: this.excerpt(),
                  }),
              ];
    }

    /** Build the calls subscribing its author, notifying and announcing a new comment. */
    announce() {
        // read whether it replies, its author and the principals it mentions
        const isReply = this.row.threadId !== null;
        const author = ObjectReference.at("author", this.row);
        const mentioned = this.mentioned();

        return [
            Subscription.subscribe(this.thread, author, isReply ? "participating" : "author"),
            ...this.mention(mentioned),
            (isReply ? reply : thread).call({
                source: this.source,
                audience: { kind: "subscribers" },
                thread: this.row.parentId,
                payload: this.excerpt(),
                excluded: mentioned,
            }),
        ];
    }

    /** Require ordered, disjoint mention spans within a body's text. */
    static requireSpans(body: Body): void {
        let end = 0;
        for (const mentioned of body.mentions) {
            if (mentioned.offset < end || mentioned.offset + mentioned.length > body.text.length) {
                throw SocialError.of(
                    "MENTIONS_INVALID",
                    "mentions must be ordered, disjoint spans within the text",
                );
            }
            end = mentioned.offset + mentioned.length;
        }
    }

    /** Require a selection within a text field of the comment's host. */
    static async requireSelection(context: MethodContext, selection: Selection): Promise<void> {
        // find the host's type and the selected elements
        const host = context.requireParent();
        const type = present(
            context.objects.find((object) => object.policy.is(host)),
            `the type of ${host.type} ${host.id}`,
        );
        const elements = [selection.anchor.element, selection.head.element];

        // refuse unknown fields
        if (!type.text.includes(selection.field)) {
            throw SocialError.of(
                "TEXT_FIELD_UNKNOWN",
                `${host.type} has no text field ${selection.field}`,
            );
        }

        // refuse elements the host's text lacks, as the caller reads it
        const asked = type.call("contains", {
            ...type.target(host),
            field: selection.field,
            elements,
        });
        const { isContained } = Containment.parse(await context.query(context.caller, asked));
        if (!isContained) {
            throw SocialError.of("SELECTION_UNKNOWN", `${selection.field} has no such selection`);
        }
    }
}

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
