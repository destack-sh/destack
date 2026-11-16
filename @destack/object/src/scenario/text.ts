import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a text world: one editing operation. */
export const TextStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Type words at a random place. */
        action: schema.literal("type"),
        /** The words typed in one change. */
        words: schema.number().int().positive(),
    }),
    schema.object({
        /** Append characters at the end of the text. */
        action: schema.literal("append"),
        /** The characters appended in one change. */
        characters: schema.number().int().positive(),
    }),
    schema.object({
        /** Prepend characters at the start of the text. */
        action: schema.literal("prepend"),
        /** The characters prepended in one change. */
        characters: schema.number().int().positive(),
    }),
    schema.object({
        /** Delete characters at a random place. */
        action: schema.literal("delete"),
        /** The characters deleted in one change. */
        characters: schema.number().int().positive(),
    }),
    schema.object({
        /** Mark a random range of words. */
        action: schema.literal("mark"),
        /** The mark's type. */
        type: schema.enum(["bold", "link", "highlight"]),
    }),
    schema.object({
        /** Remove a random mark. */
        action: schema.literal("unmark"),
    }),
    schema.object({
        /** Insert a block node at a random place. */
        action: schema.literal("node"),
    }),
    schema.object({
        /** Convert a random node into a heading. */
        action: schema.literal("convert"),
    }),
    schema.object({
        /** Read the whole text as spans. */
        action: schema.literal("spans"),
    }),
    schema.object({
        /** Merge concurrent edits of several parties. */
        action: schema.literal("merge"),
        /** The parties editing concurrently. */
        parties: schema.number().int().positive(),
        /** The changes each party makes before merging. */
        changes: schema.number().int().positive(),
    }),
]);
/** A step of a text world. */
export type TextStep = schema.Infer<typeof TextStep>;

/** An observation of a text world: a count of its characters, pieces, marks or nodes. */
export const TextObservation = schema.object({
    /** What to count. */
    count: schema.enum(["characters", "pieces", "marks", "nodes"]),
});
/** An observation of a text world. */
export type TextObservation = schema.Infer<typeof TextObservation>;

/** The interaction of text worlds. */
export const textInteraction: Interaction<
    TextStep,
    TextObservation,
    Record<string, never>
> = Object.freeze({
    name: "text",
    step: TextStep,
    observation: TextObservation,
    globals: schema.object({}),
});
