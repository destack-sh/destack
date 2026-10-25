import { schema } from "@destack/schema";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TestTemplate } from "./template.ts";

/** The moment the templates' writes run at: 2026-10-01T00:00:00Z. */
const WRITTEN_AT = Date.UTC(2026, 9, 1);

/** The seed of the templates' worlds. */
const SEED = 1;

/** Counters by name. */
export const tally = defineTable("template_tally", {
    /** The counter's name. */
    name: text("name").primaryKey(),
    /** The counted value. */
    value: integer("value").notNull(),
});

/** One counter, which the template's copies start with. */
export const tallyTemplate = new TestTemplate({
    name: "db.tally",
    databases: { tally: [tally] },
    value: schema.literal("started"),
    world: { seed: SEED, start: WRITTEN_AT },
    write: async (databases) => {
        // write the one counter
        await databases.tally.database.insert(tally).values({ name: "started", value: 1 });

        return "started" as const;
    },
});

/** Two counters in two databases written together, the second counting the first's rows. */
export const pairTemplate = new TestTemplate({
    name: "db.pair",
    databases: { counted: [tally], counting: [tally] },
    value: schema.number(),
    world: { seed: SEED, start: WRITTEN_AT },
    write: async ({ counted, counting }) => {
        // write two rows, and count them into the second database
        await counted.database.insert(tally).values([
            { name: "first", value: 1 },
            { name: "second", value: 2 },
        ]);
        const rows = await counted.database.select().from(tally);
        await counting.database.insert(tally).values({ name: "counted", value: rows.length });

        return rows.length;
    },
});

/** The templates db's tests copy, which the run writes once. */
export const templates = [tallyTemplate, pairTemplate];
