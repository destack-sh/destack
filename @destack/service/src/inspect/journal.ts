import { defineSchema, schema } from "@destack/schema";
import { TABLE, type Table } from "@destack/db";

/** A declared journal, as manifests describe it. */
export const JournalDescription = defineSchema(
    schema.object({
        /** The journal table's name within its package. */
        name: schema.string().min(1),
    }),
);
/** A declared journal, as manifests describe it. */
export type JournalDescription = schema.Infer<typeof JournalDescription>;

/** Describe a journal by its table's name. */
export function describeJournal(journal: Table): JournalDescription {
    return { name: journal[TABLE].name };
}
