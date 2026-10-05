import { principal, relation, union } from "@destack/access";
import { Subject } from "@destack/sync";
import { defineObject, field } from "@destack/object";
import { Message, plural } from "@destack/locale";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { defineNotification } from "../../src/declare/index.ts";
import { nameOf } from "./actor.ts";
import { activity, announcement, Subscription, subscription } from "../../src/index.ts";

/** The package release declaring the fixture's notifications. */
export const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** Write a message of the fixture's package, which its catalogs translate. */
const write = Message.context("", { package: notes });

/** The headline of a change. */
export const changed = write`Document changed`;

/** Summarize a thread's changes. */
export function summarizeChanges(count: number): Message {
    return write`${plural(count, { one: "# change", other: "# changes" })}`;
}

/** Someone mentions the recipient in a remark to answer from the activity. */
export const mention = defineNotification(
    {
        name: "mention",
        title: "Mentions",
        description: "Someone mentions you in a remark.",
        payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: (payload) => ({ title: `${payload.author} mentioned you`, body: payload.excerpt }),
        summary: (count) => write`${plural(count, { one: "# mention", other: "# mentions" })}`,
        actions: {
            reply: {
                title: "Reply",
                text: { placeholder: "Reply", button: "Send" },
                effect: async ({ source }, call, text) => {
                    // remark with the reply's text
                    if (text === undefined) {
                        throw new TypeError("reply action needs its text");
                    }

                    return call.invoke(document).remark({
                        id: source.id,
                        text,
                        mentions: [],
                    });
                },
            },
        },
    },
    { package: notes },
);

/** A document waits for the recipient's approval through a focus. */
export const review = defineNotification(
    {
        name: "review",
        title: "Review requests",
        description: "A document waits for your approval.",
        payload: schema.object({ title: schema.string() }),
        interruption: "timeSensitive",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: (payload) => ({ title: "Approval requested", body: payload.title }),
        summary: (count) =>
            write`${plural(count, { one: "# approval requested", other: "# approvals requested" })}`,
        actions: {
            approve: {
                title: "Approve",
                effect: ({ source }, call) => call.invoke(document).approve({ id: source.id }),
            },
        },
    },
    { package: notes },
);

/** A document changed, sent to recipients in a summary or at once. */
export const change = defineNotification(
    {
        name: "change",
        title: "Changes",
        description: "A document you follow changed.",
        payload: schema.object({ summary: schema.string() }),
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: (payload) => ({ title: changed, body: payload.summary }),
        summary: summarizeChanges,
    },
    { package: notes },
);

/** A document's publishing status, replaced by each later status. */
export const status = defineNotification(
    {
        name: "status",
        title: "Publishing status",
        description: "Where publishing a document stands.",
        payload: schema.object({ state: schema.enum(["publishing", "published", "failed"]) }),
        interruption: "passive",
        preference: { channels: ["desktop", "push"], delivery: "immediate" },
        content: (payload) => ({ title: "Publishing", body: payload.state }),
        summary: (count) =>
            write`${plural(count, { one: "# publishing update", other: "# publishing updates" })}`,
    },
    { package: notes },
);

/** Every notification the fixture declares. */
export const notifications = [mention, review, change, status];

/** A remark on a document: its text and the principals it mentions. */
const Remark = schema.object({
    /** The text. */
    text: schema.string().min(1),
    /** The principals mentioned. */
    mentions: schema.array(Subject),
});

/** Documents their owner shares with editors and viewers, taking activities, announcements and subscriptions. */
export const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: {
        /** The principal who created the document. */
        owner: field.subject().caller(),
        /** The title. */
        title: field.string(schema.string().min(1)),
        /** The principal who approved it, absent until approved. */
        approvedBy: field.subject().optional(),
    },
    relations: {
        editor: { subjects: [principal.user] },
        viewer: { subjects: [principal.user] },
    },
    permissions: {
        read: union(relation("owner"), relation("editor"), relation("viewer")),
        edit: union(relation("owner"), relation("editor")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    attachments: [
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("manage", { fields: ["title"] }),
        remark: method.mutation({ permission: "read", input: Remark }),
        publish: method.mutation({
            permission: "edit",
            input: schema.object({ state: schema.enum(["publishing", "published", "failed"]) }),
        }),
        request: method.mutation({
            permission: "edit",
            input: schema.object({ reviewer: Subject }),
        }),
        approve: method.mutation({ permission: "edit" }),
        edit: method.mutation({
            permission: "edit",
            input: schema.object({ summary: schema.string() }),
        }),
        broadcast: method.mutation({
            permission: "manage",
            input: schema.object({
                audience: schema.enum(["subscribers", "members", "editors"]),
                excluded: schema.array(Subject).exactOptional(),
            }),
        }),
    }),
}).handle({
    remark: async (call) => {
        // subscribe and notify the principals a remark mentions
        const { text, mentions } = call.input;
        const author = call.requireCaller();
        for (const mentioned of mentions) {
            await Subscription.add(call, call.reference(), mentioned, "mention");
        }
        await mention.notify(call, {
            source: call.reference(),
            recipients: mentions,
            reason: "mention",
            payload: { author: nameOf(author), excerpt: text },
        });

        return call.target;
    },
    publish: async (call) => {
        // tell the owner the latest publishing state
        const { state } = call.input;
        await status.notify(call, {
            source: call.reference(),
            recipients: [Subject.read(call.target.owner)],
            reason: "author",
            payload: { state },
            key: "publishing",
        });

        return call.target;
    },
    request: async (call) => {
        // ask a reviewer for approval
        const { reviewer } = call.input;
        await review.notify(call, {
            source: call.reference(),
            recipients: [reviewer],
            reason: "assigned",
            payload: { title: call.target.title },
            key: "review",
        });

        return call.target;
    },
    approve: async (call) => call.update({ approvedBy: Subject.key(call.requireCaller()) }),
    edit: async (call) => {
        // tell the document's subscribers about the change
        const { summary } = call.input;
        const subscribers = await Subscription.list(call, call.reference(), { limit: 100 });
        await change.notify(call, {
            source: call.reference(),
            recipients: subscribers.map((entry) => entry.owner),
            reason: "subscribed",
            payload: { summary },
        });

        return call.target;
    },
    broadcast: async (call) => {
        // announce the document to its subscribers, the space's members or its editors
        const { audience, excluded } = call.input;
        await change.announce(call, {
            source: call.reference(),
            audience:
                audience === "editors"
                    ? { kind: "permission", permission: "edit", reason: "mention" }
                    : audience === "members"
                      ? { kind: "members", reason: "mention" }
                      : { kind: "subscribers" },
            payload: { summary: `${call.target.title} is out` },
            ...(excluded === undefined ? {} : { excluded }),
        });

        return call.target;
    },
});
