import { space, user } from "@destack/account/object";
import { principal, relation, union } from "@destack/access";
import { argument, type Catalog, Message, plural, t } from "@destack/locale";
import { defineObject, field, Package } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { testBuild } from "@destack/object/test";
import type { Crypto } from "@destack/environment";
import { PackageVersion } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { defineNotification } from "../declare/index.ts";
import { activity, announcement, subscription } from "../object/index.ts";
import notificationPackage from "../package.ts";
import { shareable } from "@destack/object";

/** The documents application, whose calls notify the people sharing its documents. */
export const NOTES = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** A remark on a document: its text and the principals it mentions. */
const Remark = schema.object({
    /** The text. */
    text: schema.string().min(1),
    /** The principals mentioned. */
    mentions: schema.array(Subject),
});

/** Documents their owner shares with editors and viewers, taking activities. */
export const document = defineObject(
    {
        name: "document",
        plural: "documents",
        title: t`${plural(argument("count"), { one: "Document", other: "Documents" })}`,
        scope: space,
        fields: {
            /** The principal who created the document. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
            /** The principal who approved it, absent until approved. */
            approvedBy: field.subject().optional(),
        },
        attachments: [
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
            subscription.attach({ by: "read" }),
        ],
        access: {
            relations: {
                editor: { subjects: [principal.user] },
                viewer: { subjects: [principal.user] },
            },
            permissions: {
                read: union(relation("owner"), relation("editor"), relation("viewer")),
                edit: union(relation("owner"), relation("editor")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
        remark: method.mutation({ title: t`Remark`, permission: "read", input: Remark }),
        reply: method.mutation({
            title: t`Reply`,
            permission: "read",
            input: schema.object({ text: schema.string().min(1) }),
        }),
        publish: method.mutation({
            title: t`Publish`,
            permission: "edit",
            input: schema.object({ state: schema.enum(["publishing", "published", "failed"]) }),
        }),
        request: method.mutation({
            title: t`Request`,
            permission: "edit",
            input: schema.object({ reviewer: Subject }),
        }),
        approve: method.mutation({ title: t`Approve`, permission: "edit" }),
        edit: method.mutation({
            title: t`Edit`,
            permission: "edit",
            input: schema.object({ summary: schema.string() }),
        }),
        broadcast: method.mutation({
            title: t`Broadcast`,
            permission: "manage",
            input: schema.object({
                audience: schema.enum(["subscribers", "members", "editors"]),
                excluded: schema.array(Subject).exactOptional(),
            }),
        }),
    }),
    { package: NOTES },
);

/** The documents' calls, which notify the people sharing a document. */
export const documentHandlers = document.implement({
    methods: {
        remark: async (call) => {
            // notify the principals a remark mentions, as its author's profile names them
            const { text, mentions } = call.input;
            const author = call.caller.subject;
            const profile = await call
                .read(user)
                .findFirst({ where: { id: user.identifier(author.id) } });
            await call.mutate(call.caller, [
                mention.call({
                    source: call.reference(),
                    recipients: mentions,
                    reason: "mention",
                    payload: {
                        author: present(profile, `the profile of ${author.id}`).name,
                        excerpt: text,
                    },
                }),
            ]);

            return call.target;
        },
        reply: async (call) => {
            // remark with the reply's text, mentioning no one
            await call.mutate(call.caller, [
                document.call("remark", {
                    id: call.target.id,
                    text: call.input.text,
                    mentions: [],
                }),
            ]);

            return call.target;
        },
        publish: async (call) => {
            // tell the owner the latest publishing state
            const { state } = call.input;
            await call.mutate(call.caller, [
                status.call({
                    source: call.reference(),
                    recipients: [ObjectReference.at("owner", call.target)],
                    reason: "author",
                    payload: { state },
                    key: "publishing",
                }),
            ]);

            return call.target;
        },
        request: async (call) => {
            // ask a reviewer for approval
            const { reviewer } = call.input;
            await call.mutate(call.caller, [
                review.call({
                    source: call.reference(),
                    recipients: [reviewer],
                    reason: "assigned",
                    payload: { title: call.target.title },
                    key: "review",
                }),
            ]);

            return call.target;
        },
        approve: async (call) =>
            call.update(ObjectReference.columns("approvedBy", call.caller.subject)),
        edit: async (call) => {
            // tell the document's subscribers about the change
            const { summary } = call.input;
            await call.mutate(call.caller, [
                change.call({
                    source: call.reference(),
                    audience: { kind: "subscribers" },
                    payload: { summary },
                }),
            ]);

            return call.target;
        },
        broadcast: async (call) => {
            // announce the document to its subscribers, the space's members or its editors
            const { audience, excluded } = call.input;
            await call.mutate(call.caller, [
                change.call({
                    source: call.reference(),
                    audience:
                        audience === "editors"
                            ? { kind: "permission", permission: "edit", reason: "mention" }
                            : audience === "members"
                              ? { kind: "members", reason: "mention" }
                              : { kind: "subscribers" },
                    payload: { summary: `${call.target.title} is out` },
                    ...(excluded === undefined ? {} : { excluded }),
                }),
            ]);

            return call.target;
        },
    },
});

/** Write a message of the documents' package, which its catalogs translate. */
const write = Message.context("", { package: NOTES });

/** The headline of a change. */
export const changed = write`Document changed`;

/** The summary of a thread's changes. */
export const changes = write`${plural(argument("count"), { one: "# change", other: "# changes" })}`;

/** Someone mentions the recipient in a remark to answer from the activity. */
export const mention = defineNotification(
    {
        name: "mention",
        title: t`Mentions`,
        description: t`Someone mentions you in a remark.`,
        payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: {
            title: write`${argument("author")} mentioned you`,
            body: write`${argument("excerpt")}`,
        },
        summary: write`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
        actions: [
            {
                name: "reply",
                title: "Reply",
                text: { placeholder: "Reply", button: "Send" },
                type: document,
                method: "reply",
            },
        ],
    },
    { package: NOTES },
);

/** A document waits for the recipient's approval through a focus. */
export const review = defineNotification(
    {
        name: "review",
        title: t`Review requests`,
        description: t`A document waits for your approval.`,
        payload: schema.object({ title: schema.string() }),
        interruption: "timeSensitive",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: { title: write`Approval requested`, body: write`${argument("title")}` },
        summary: write`${plural(argument("count"), { one: "# approval requested", other: "# approvals requested" })}`,
        actions: [{ name: "approve", title: "Approve", type: document, method: "approve" }],
    },
    { package: NOTES },
);

/** A document changed, sent to recipients in a summary or at once. */
export const change = defineNotification(
    {
        name: "change",
        title: t`Changes`,
        description: t`A document you follow changed.`,
        payload: schema.object({ summary: schema.string() }),
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        content: { title: changed, body: write`${argument("summary")}` },
        summary: changes,
    },
    { package: NOTES },
);

/** A document's publishing status, replaced by each later status. */
export const status = defineNotification(
    {
        name: "status",
        title: t`Publishing status`,
        description: t`Where publishing a document stands.`,
        payload: schema.object({ state: schema.enum(["publishing", "published", "failed"]) }),
        interruption: "passive",
        preference: { channels: ["desktop", "push"], delivery: "immediate" },
        content: { title: write`Publishing`, body: write`${argument("state")}` },
        summary: write`${plural(argument("count"), { one: "# publishing update", other: "# publishing updates" })}`,
    },
    { package: NOTES },
);

/** Every notification the documents application declares. */
export const notifications = [mention, review, change, status];

/** The documents' package's German catalog, which its build ships. */
const NOTES_CATALOG: Catalog = {
    package: NOTES.id,
    locale: "de",
    messages: {
        [changed.id]: "Dokument geändert",
        [changes.id]:
            ".input {$count :number}\n.match $count\none {{{$count} Änderung}}\n* {{{$count} Änderungen}}",
    },
};

/** The documents package's handle. */
export const notesPackage = Package.of(NOTES, { objects: [document] });

/** The documents package's server. */
export const notesServer = implementPackage(notesPackage, { objects: [documentHandlers] });

/** Write the documents' build: its package, notifications and German catalog, on notification. */
export function openNotesBuild(crypto: Crypto, notification: BuildReader): Promise<BuildReader> {
    return testBuild(notesPackage, crypto, {
        dependencies: [notification],
        declared: notifications.map((declared) => ({
            kind: "notification",
            kindPackageId: notificationPackage.id,
            name: declared.name,
            description: declared.describe(),
        })),
        files: new Map([
            ["locale/de.json", new TextEncoder().encode(JSON.stringify(NOTES_CATALOG))],
        ]),
    });
}

/** Refer to a document as its attachments' parent. */
export function documentParent(id: string) {
    return {
        parent: { packageId: document.policy.definition.packageId, type: document.name, id },
    };
}
