import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a code world: one operation on its source. */
export const CodeStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Write a view of some lines as the source. */
        action: schema.literal("write"),
        /** The lines. */
        lines: schema.number().int().positive(),
    }),
    schema.object({
        /** Parse the source, the whole of it or its first screen. */
        action: schema.literal("parse"),
        /** How much of it. */
        extent: schema.enum(["whole", "screen"]),
    }),
    schema.object({
        /** Type a character at a random word, parsing again what it changes. */
        action: schema.literal("type"),
    }),
    schema.object({
        /** Read the tokens of a random screen. */
        action: schema.literal("tokens"),
    }),
    schema.object({
        /** Compare the source with one some of whose lines changed. */
        action: schema.literal("compare"),
        /** How many lines of a hundred changed. */
        share: schema.number().int().min(0).max(100),
    }),
    schema.object({
        /** Merge two sources changed from the source, at different lines of a hundred. */
        action: schema.literal("merge"),
        /** How many lines of a hundred each changed. */
        share: schema.number().int().min(0).max(100),
    }),
]);
/** A step of a code world. */
export type CodeStep = schema.Infer<typeof CodeStep>;

/** An observation of a code world: a count of its lines or of the changes found last. */
export const CodeObservation = schema.object({
    /** What to count. */
    count: schema.enum(["lines", "changes"]),
});
/** An observation of a code world. */
export type CodeObservation = schema.Infer<typeof CodeObservation>;

/** The interaction of code worlds. */
export const codeInteraction: Interaction<
    CodeStep,
    CodeObservation,
    Record<string, never>
> = Object.freeze({
    name: "code",
    step: CodeStep,
    observation: CodeObservation,
    globals: schema.object({}),
});
