import { space } from "@destack/account/object";
import { context, through } from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, nested } from "@destack/object";
import { defineNotification } from "@destack/notification/declare";
import { Subject } from "@destack/sync";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** The longest passage a mention notice carries: at most 840 bytes of a 2048-byte payload. */
export const PASSAGE_LENGTH = 280;

/** A subject an object's text field mentions, one row per field and subject while it does. */
export const mention = defineObject(
    {
        name: "mention",
        plural: "mentions",
        title: t`${plural(argument("count"), { one: "Mention", other: "Mentions" })}`,
        scope: space,
        fields: {
            /** The owner's text field holding the mention. */
            field: field.string(),
            /** The mentioned person, group or object. */
            subject: field.object(),
        },
        constraints: {
            pair: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "field", "subjectId"],
            },
            subject: { kind: "index", on: ["subjectId"] },
        },
        access: {
            permissions: {
                read: through("parent", "read"),
                track: through("parent", "read"),
                // let any identified editor match a text it changed, each create still requiring read on the host
                observe: context({}),
            },
        },
        traits: [nested({ in: "any", add: "read" })],
        observes: { node: "mention" },
    },
    (method) => ({
        /** Record a mention a text gained, notifying its subject, as the controller does. */
        create: method.create("track", { visibility: "package" }),
        /** Forget a mention a text lost, as the controller does. */
        delete: method.delete("track", { visibility: "package" }),
        list: method.list("read"),
        /** Match a host text's mentions to its nodes after an edit changed them, as its editor. */
        observe: method.mutation({
            title: t.context("action")`Observe`,
            permission: "observe",
            target: "scope",
            input: schema.object({
                /** The host whose text changed. */
                parent: schema.object({
                    packageId: PackageId,
                    type: schema.string(),
                    id: schema.string(),
                }),
                /** The host's text field. */
                field: schema.string().min(1),
            }),
            output: schema.null(),
        }),
    }),
);

/** Who mentioned the recipient, and the start of the text that does. */
export const Passage = schema.object({
    /** The editor who wrote the mention. */
    author: Subject,
    /** The start of the text. */
    text: schema.string().max(PASSAGE_LENGTH),
});
/** Who mentioned the recipient, and the start of the text that does. */
export type Passage = schema.Infer<typeof Passage>;

// TODO #Cleanup: fold comments' `mention` notification into this one once comment bodies are rich text
/** A text mentions the recipient. */
export const mentioned = defineNotification({
    name: "mentioned",
    title: t.context("action")`Mentions in texts`,
    description: t`Someone mentions you in a text.`,
    payload: Passage,
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t.context("title")`New mention`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
    actions: [],
});
