import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a database world: writes and reads of notes, log transactions and blobs. */
export const DatabaseStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Insert notes in one database transaction. */
        action: schema.literal("insert"),
        /** The notes. */
        rows: schema.number().int().positive(),
    }),
    schema.object({
        /** Read the next note in turn by its key. */
        action: schema.literal("read"),
    }),
    schema.object({
        /** Update the note inserted last by its key. */
        action: schema.literal("update"),
    }),
    schema.object({
        /** Insert a note in one optimistic transaction over the log. */
        action: schema.literal("transact"),
    }),
    schema.object({
        /** Store a blob of some bytes. */
        action: schema.literal("write"),
        /** The blob's bytes. */
        bytes: schema.number().int().positive(),
    }),
    schema.object({
        /** Read the next stored blob in turn. */
        action: schema.literal("fetch"),
    }),
]);
/** A step of a database world. */
export type DatabaseStep = schema.Infer<typeof DatabaseStep>;

/** An observation of a database world: the notes it keeps or the blobs it stores. */
export const DatabaseObservation = schema.object({
    /** Count the notes or the stored blobs. */
    count: schema.enum(["notes", "blobs"]),
});
/** An observation of a database world. */
export type DatabaseObservation = schema.Infer<typeof DatabaseObservation>;

/** The interaction of database worlds, which the database's driver plays. */
export const databaseInteraction: Interaction<
    DatabaseStep,
    DatabaseObservation,
    Record<string, never>
> = Object.freeze({
    name: "database",
    step: DatabaseStep,
    observation: DatabaseObservation,
    globals: schema.object({}),
});
