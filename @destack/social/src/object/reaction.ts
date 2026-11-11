import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The longest emoji in UTF-16 code units: 15, the longest RGI sequence. */
const EMOJI_LENGTH = 15;

/** One principal's emoji reaction to an object. */
export const reaction = defineObject(
    {
        name: "reaction",
        plural: "reactions",
        title: t`${plural(argument("count"), { one: "Reaction", other: "Reactions" })}`,
        scope: space,
        fields: {
            /** The author. */
            author: field.subject().caller(),
            /** The emoji. */
            emoji: field.string(schema.emoji().max(EMOJI_LENGTH)),
        },
        aggregates: { reactionCount: { function: "count" } },
        constraints: {
            authorEmoji: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "authorId", "emoji"],
            },
        },
        access: {
            permissions: {
                read: through("parent", "read"),
                write: intersection(relation("author"), through("parent", "react")),
            },
        },
        traits: [nested({ in: "any", add: "react" })],
    },
    () => ({ update: false }),
);
