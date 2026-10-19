import { schema } from "@destack/schema";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TestTemplate } from "./template.ts";

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
    tables: [tally],
    value: schema.literal("started"),
    write: async (storage) => {
        await storage.database.insert(tally).values({ name: "started", value: 1 });

        return "started" as const;
    },
});

/** The templates db's tests copy, which the run writes once. */
export const templates = [tallyTemplate];
