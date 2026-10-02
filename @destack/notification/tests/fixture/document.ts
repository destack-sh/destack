import { principal, relation, union } from "@destack/access";
import { Subject } from "@destack/sync";
import { defineObject, field, method } from "@destack/object";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { defineNotification } from "../../src/declare/index.ts";
import { nameOf } from "./actor.ts";
import { announcement, notification, Subscription, subscription } from "../../src/index.ts";

/** The package release declaring the fixture's notifications. */
export const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** Someone mentions the recipient in a remark to answer from the notification. */
export const mention = defineNotification(
    {
        name: "mention",
        title: "Mentions",
        description: "Someone mentions you in a remark.",
        payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: (payload) => ({ title: `${payload.author} mentioned you`, body: payload.excerpt }),
        summary: (count) => (count === 1 ? "1 mention" : `${count} mentions`),
        actions: {
            reply: {
                title: "Reply",
                text: { placeholder: "Reply", button: "Send" },
                effect: ({ source }, call, text) =>
                    call.invoke(document, "remark", {
                        spaceId: source.scope,
                        id: source.id,
                        text: text!,
                        mentions: [],
                    }),
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
        summary: (count) => `${count} approvals requested`,
        actions: {
            approve: {
                title: "Approve",
                effect: ({ source }, call) =>
                    call.invoke(document, "approve", { spaceId: source.scope, id: source.id }),
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
        content: (payload) => ({ title: "Document changed", body: payload.summary }),
        summary: (count) => (count === 1 ? "1 change" : `${count} changes`),
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
        summary: (count) => `${count} publishing updates`,
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

/** Documents their owner shares with editors and viewers, taking notifications, announcements and subscriptions. */
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
        notification.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
    methods: {
        get: method.get("read"),
        create: method.create("manage", { fields: ["title"] }),
        remark: method({ permission: "read", input: Remark }),
        publish: method({
            permission: "edit",
            input: schema.object({ state: schema.enum(["publishing", "published", "failed"]) }),
        }),
        request: method({ permission: "edit", input: schema.object({ reviewer: Subject }) }),
        approve: method({ permission: "edit" }),
        edit: method({ permission: "edit", input: schema.object({ summary: schema.string() }) }),
        broadcast: method({
            permission: "manage",
            input: schema.object({
                audience: schema.enum(["subscribers", "members", "editors"]),
                excluded: schema.array(Subject).optional(),
            }),
        }),
    },
}).handle({
    remark: async (call) => {
        // subscribe and notify the principals a remark mentions
        const { text, mentions } = call.input as schema.Infer<typeof Remark>;
        for (const mentioned of mentions) {
            await Subscription.add(call, call.reference(), mentioned, "mention");
        }
        await mention.notify(call, {
            source: call.reference(),
            recipients: mentions,
            reason: "mention",
            payload: { author: nameOf(call.caller!), excerpt: text },
        });

        return call.target;
    },
    publish: async (call) => {
        // tell the owner the latest publishing state
        const { state } = call.input as { readonly state: "publishing" | "published" | "failed" };
        await status.notify(call, {
            source: call.reference(),
            recipients: [Subject.read(call.target!.owner as string)],
            reason: "author",
            payload: { state },
            key: "publishing",
        });

        return call.target;
    },
    request: async (call) => {
        // ask a reviewer for approval
        const { reviewer } = call.input as { readonly reviewer: Subject };
        await review.notify(call, {
            source: call.reference(),
            recipients: [reviewer],
            reason: "assigned",
            payload: { title: String(call.target!.title) },
            key: "review",
        });

        return call.target;
    },
    approve: async (call) => call.update({ approvedBy: Subject.key(call.caller!) }),
    edit: async (call) => {
        // tell the document's subscribers about the change
        const { summary } = call.input as { readonly summary: string };
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
        const { audience, excluded } = call.input as {
            readonly audience: "subscribers" | "members" | "editors";
            readonly excluded?: readonly Subject[];
        };
        await change.announce(call, {
            source: call.reference(),
            audience:
                audience === "editors"
                    ? { kind: "permission", permission: "edit", reason: "mention" }
                    : audience === "members"
                      ? { kind: "members", reason: "mention" }
                      : { kind: "subscribers" },
            payload: { summary: `${String(call.target!.title)} is out` },
            ...(excluded === undefined ? {} : { excluded }),
        });

        return call.target;
    },
});
