import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { unique } from "@destack/db";
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
    constraints: (entry) => [
        unique("reaction_author_emoji").on(
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.author,
            entry.emoji,
        ),
    ],
    permissions: {
        read: through("parent", "read"),
        write: intersection(relation("author"), through("parent", "react")),
    },
    aggregates: { reactionCount: { function: "count" } },
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        delete: method.delete("write"),
    }),
});
