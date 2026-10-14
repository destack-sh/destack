import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The longest emoji in UTF-16 code units: 15, the longest RGI sequence. */
const EMOJI_LENGTH = 15;

/** One principal's emoji reaction to an object. */
export const reaction = defineObject({
    name: "reaction",
    plural: "reactions",
    scope: space,
    nested: { in: "any", receive: "react" },
    fields: {
        /** The author. */
        author: field.subject().caller(),
        /** The emoji. */
        emoji: field.string(schema.emoji().max(EMOJI_LENGTH)),
    },
    permissions: {
        read: through("parent", "read"),
        write: intersection(relation("author"), through("parent", "react")),
    },
    aggregates: { reactionCount: { function: "count" } },
    constraints: {
        authorEmoji: {
            kind: "unique",
            on: ["parentPackageId", "parentType", "parentId", "author", "emoji"],
        },
    },
}).methods((method) => ({
    create: method.create("write"),
    delete: method.delete("write"),
}));
