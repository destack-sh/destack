import { expect, test } from "@destack/test";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { defineDatabase } from "./database.ts";

/** Users, whose rows live in the global tier. */
const user = defineTable("user", { id: text("id").primaryKey() }, { tier: "global" });

/** Spaces, whose rows live in a region. */
const space = defineTable("space", { id: text("id").primaryKey() }, { tier: "regional" });

/** Notes, which live in any tier. */
const note = defineTable("note", { id: text("id").primaryKey() });

test("hold tables of a database's own tier and of wider tiers, and refuse a narrower tier's", () => {
    // replicate global users into a regional database beside its own and untiered tables
    const regional = defineDatabase({
        name: "cell",
        tier: "regional",
        tables: [user, space, note],
    });
    expect(regional.tables).toEqual([user, space, note]);

    // replicate global users and regional spaces into a zonal database
    const zonal = defineDatabase({ name: "zone", tables: [note, user, space] });
    expect(zonal.tables).toEqual([note, user, space]);

    // refuse regional spaces in the global database
    expect(() => defineDatabase({ name: "global", tier: "global", tables: [user, space] })).toThrow(
        new TypeError("regional table destack__db__space in a global database"),
    );
});
