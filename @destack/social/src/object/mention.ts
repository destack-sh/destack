import { argument, plural, t } from "@destack/locale";
import { textNode } from "@destack/object";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";
import { defineNotification } from "@destack/notification/declare";
import { Subject } from "@destack/sync";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** The longest passage a mention notice carries: at most 840 bytes of a 2048-byte payload. */
export const PASSAGE_LENGTH = 280;

/** Who mentioned the recipient, and the start of the text that does. */
export const Passage = schema.object({
    /** The editor who wrote the mention. */
    author: Subject,
    /** The start of the text. */
    text: schema.string().max(PASSAGE_LENGTH),
});
/** Who mentioned the recipient, and the start of the text that does. */
export type Passage = schema.Infer<typeof Passage>;

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

/** A mention a text gained: the text's owner and field, and the mentioned subject. */
export const MentionInput = schema.object({
    /** The scope with the text. */
    scope: schema.string(),
    /** The text's owner. */
    parent: schema.object({ packageId: PackageId, type: schema.string(), id: schema.string() }),
    /** The owner's text field. */
    field: schema.string(),
    /** The mentioned subject. */
    subject: Subject,
});
/** A mention a text gained. */
export type MentionInput = schema.Infer<typeof MentionInput>;

/** The row a text gains for a node, as the mention trigger reads it. */
const GAINED = schema.looseObject({
    scope: schema.string(),
    parentPackageId: PackageId,
    parentType: schema.string(),
    parentId: schema.string(),
    field: schema.string(),
    value: Subject,
});

/** Notify each principal a text starts to mention, once per text and principal. */
export const mentioning = defineWorkflow({
    name: "mentioning",
    title: t.context("title")`Mention notices`,
    input: MentionInput,
    output: NO_OUTPUT,
    triggers: [
        {
            name: "mentioned",
            kind: "change",
            change: {
                object: textNode,
                where: { type: "mention" },
                operations: ["create"],
                actor: "change",
            },
            input: (change) => {
                const row = GAINED.parse(change.after);

                return {
                    scope: row.scope,
                    parent: {
                        packageId: row.parentPackageId,
                        type: row.parentType,
                        id: row.parentId,
                    },
                    field: row.field,
                    subject: row.value,
                };
            },
        },
    ],
});
