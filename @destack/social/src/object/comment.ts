import { space } from "@destack/account/object";
import { intersection, permission, relation, through, union } from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { ObjectReference, Subject } from "@destack/sync";
import { type Select } from "@destack/db";
import { activity, announcement, Subscription, subscription } from "@destack/notification/object";
import { defineNotification } from "@destack/notification/declare";
import {
    MethodContext,
    Containment,
    defineObject,
    field,
    type ObjectOf,
    RICH_TEXT,
    TextSelection,
    TextSpan,
} from "@destack/object";
import { present, schema } from "@destack/schema";
import { SocialError } from "../error/index.ts";
import { reaction } from "./reaction.ts";
import { defineAction, nested, selected } from "@destack/object";

/** The longest excerpt a notification carries: at most 840 bytes, within a payload's 2048. */
const EXCERPT_LENGTH = 280;

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
        title: t`${plural(argument("count"), { one: "Comment", other: "Comments" })}`,
        scope: space,
        fields: {
            /** The author. */
            author: field.subject().caller(),
            /** The text, with its marks and mentions. */
            body: field.text(RICH_TEXT),
            /** The text the thread comments on, on its first comment only. */
            selection: field.json(TextSelection).optional(),
            /** The thread's first comment, absent on the first comment itself. */
            thread: field
                .reference((): ObjectOf<{ identity: "comment" }> => comment, { delete: "cascade" })
                .optional(),
            /** When the thread was resolved, absent while open. */
            resolvedAt: field.time().optional(),
            /** Who resolved the thread, absent while open. */
            resolvedBy: field.subject().optional(),
            /** The number of reactions. */
            reactionCount: field.count(),
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
        access: {
            permissions: {
                read: through("parent", "read"),
                comment: through("parent", "comment"),
                write: intersection(relation("author"), through("parent", "comment")),
                resolve: union(permission("write"), through("parent", "write")),
                delete: union(permission("write"), through("parent", "manage")),
            },
        },
        traits: [nested({ in: "any", add: "comment" })],
    },
    (method) => ({
        create: method.create("comment", { fields: ["body", "selection"] }),
        resolve: method.mutation({ title: t.context("action")`Resolve`, permission: "resolve" }),
        reopen: method.mutation({ title: t.context("action")`Reopen`, permission: "resolve" }),
        reply: method.mutation({
            title: t.context("action")`Reply`,
            permission: "comment",
            input: schema.object({
                /** The reply's text. */
                body: schema.array(TextSpan).min(1),
            }),
        }),
        // edit the body through the text's edit method, which the update's permission allows
        update: method.update("write", { fields: [] }),
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

    /** Read where a comment's notices go and which thread they join, from its row. */
    static notice(row: Comment["row"]): {
        readonly source: ObjectReference;
        readonly thread: string;
    } {
        return { source: new Comment(row).source, thread: row.parentId };
    }

    /** Build the comment's excerpt from its body, without splitting a surrogate pair. */
    excerpt(body: readonly TextSpan[]): Excerpt {
        const text = TextSpan.plain(body).slice(0, EXCERPT_LENGTH);
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
                "thread's first comment is required: only it takes replies and resolves the thread",
            );
        }

        return this;
    }

    /** Build the calls subscribing its author and announcing a new comment, from its body. */
    announce(body: readonly TextSpan[]) {
        // read whether it replies and its author, leaving its mentions to the mention notices
        const isReply = this.row.threadId !== null;
        const author = ObjectReference.at("author", this.row);

        return [
            Subscription.subscribe(this.thread, author, isReply ? "participating" : "author"),
            (isReply ? reply : thread).call({
                source: this.source,
                audience: { kind: "subscribers" },
                thread: this.row.parentId,
                payload: this.excerpt(body),
            }),
        ];
    }

    /** Require a selection within a text field of the comment's host. */
    static async requireSelection(context: MethodContext, selection: TextSelection): Promise<void> {
        // find the host's type and the selected elements
        const host = context.requireParent();
        const type = present(
            context.objects.find((object) => object.policy.is(host)),
            `the type of ${host.type} ${host.id}`,
        );
        const elements = [selection.anchor, selection.head].flatMap((edge) =>
            "element" in edge ? [edge.element] : [],
        );

        // refuse unknown fields
        if (!type.text.includes(selection.field)) {
            throw SocialError.of(
                "TEXT_FIELD_INVALID",
                `text field '${selection.field}' is invalid: ${host.type} has no such text field`,
            );
        }

        // skip a selection of the text's edges only
        if (elements.length === 0) {
            return;
        }

        // refuse elements the host's text lacks
        const asked = type.call("contains", {
            ...type.target(host),
            field: selection.field,
            elements,
        });
        const { isContained } = Containment.parse(await context.query(context.caller, asked));
        if (!isContained) {
            throw SocialError.of(
                "SELECTION_INVALID",
                `selection in '${selection.field}' is invalid: the host text has no such selection`,
            );
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

/** Reply in a comment's thread. */
export const answer = defineAction({
    name: "comment.reply",
    title: t.context("action")`Reply`,
    input: { comments: selected(comment), text: schema.string().min(1) },
    call: ({ comments, text }) => comments.prepare("reply", { body: [{ text }] }),
});

/** Reply in the notified comment's thread from a notification, typing the reply. */
const answerInThread = {
    action: answer,
    target: "thread",
    text: { placeholder: "Reply", button: "Send" },
} as const;

/** A comment starts a thread on a subscribed object. */
export const thread = defineNotification({
    name: "thread",
    title: t.context("title")`Threads`,
    description: t`Someone starts a comment thread on something you subscribe to.`,
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t.context("title")`New thread`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# thread", other: "# threads" })}`,
    actions: [answerInThread],
});

/** A comment replies in a subscribed thread. */
export const reply = defineNotification({
    name: "reply",
    title: t.context("title")`Replies`,
    description: t`Someone replies in a comment thread you subscribe to.`,
    payload: Excerpt,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t.context("title")`New reply`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# reply", other: "# replies" })}`,
    actions: [answerInThread],
});
